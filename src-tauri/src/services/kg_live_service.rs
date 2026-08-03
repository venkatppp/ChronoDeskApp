//! Live Knowledge Graph service (RC-8 M2).
//!
//! Business logic composing [`KgService`] (construction/traversal/search)
//! with [`KgLiveRepository`](crate::repositories::KgLiveRepository)
//! (semantic edge SQL, decay, cache):
//!
//! - **Incremental sync** — the watermark-driven pass from `KgService`
//!   with query-cache invalidation after every write.
//! - **Semantic `related_to` edges** — [`KgLiveService::rebuild_semantic_edges`]
//!   embeds node text, thresholds pairwise cosine similarity, and
//!   persists `related_to` edges carrying that similarity as confidence.
//! - **Edge confidence + decay** — semantic confidence ages exponentially
//!   (`decay_f ^ days`) and is pruned below a floor; structural edges
//!   (confidence 1.0) are exempt.
//! - **Analytics** — degree distribution, degree + eigenvector
//!   centrality, connected components, workspace importance, cached per
//!   scope.
//! - **Multi-hop context, recommendations, relationship inspector** —
//!   weighted traversal with hop decay behind a persisted query cache.
//!
//! All SQL lives in repositories; every scoring/decay/centrality policy
//! lives here.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::copilot::memory::vector::MemoryVectorSystem;
use crate::errors::DatabaseError;
use crate::models::kg::{GraphNodeType, GraphRelationshipType, KgEdge, KgNode};
use crate::models::kg_live::{
    DegreeBucket, EdgeDecaySummary, GraphAnalytics, GraphComponent, GraphEmbedder,
    GraphRecommendation, MultiHopContext, MultiHopHit, NodeCentrality, QueryCacheStats,
    RelationshipDetail, RelationshipDetails, SemanticEdgeResult, TypeCount, WorkspaceImportance,
};
use crate::repositories::KgLiveRepository;
use crate::services::KgService;

/// Minimum cosine similarity for a persisted `related_to` edge.
const SEMANTIC_THRESHOLD: f64 = 0.45;
/// Per-day exponential confidence decay factor for semantic edges.
const DECAY_FACTOR_PER_DAY: f64 = 0.92;
/// Confidence floor under which a semantic edge is pruned.
const MIN_CONFIDENCE: f64 = 0.10;
/// Edges younger than this (in days) are left untouched by a decay pass
/// — their `updated_at` is their creation or last semantic refresh.
const DECAY_FRESH_MIN_AGE_DAYS: f64 = 0.5;
/// Maximum nodes embedded in one semantic rebuild pass.
const MAX_SEMANTIC_NODES: usize = 500;
/// Maximum depth explored by multi-hop expansion.
const MAX_HOPS: usize = 4;
/// Maximum distance (in edges) of a recommendation.
const MAX_RECOMMENDATION_HOPS: usize = 3;
/// Max recommendations returned by one call.
const MAX_RECOMMENDATIONS: usize = 20;
/// Max member titles sampled per analytics component.
const MAX_COMPONENT_SAMPLES: usize = 5;
/// Cache TTLs.
const ANALYTICS_TTL_SECONDS: i64 = 60;
const QUERY_TTL_SECONDS: i64 = 60;
/// Power-iteration rounds for eigenvector centrality.
const CENTRALITY_ITERATIONS: usize = 12;

type NodeKey = (String, Uuid);

/// Live Knowledge Graph service.
#[derive(Clone)]
pub struct KgLiveService {
    kg: KgService,
    repository: KgLiveRepository,
    embedder: Option<Arc<dyn GraphEmbedder>>,
}

impl KgLiveService {
    pub fn new(kg_service: KgService, repository: KgLiveRepository) -> Self {
        Self {
            kg: kg_service,
            repository,
            embedder: None,
        }
    }

    /// Enables semantic `related_to` edges by attaching the memory vector
    /// system's embedder.
    pub fn with_embedder(mut self, embedder: Arc<dyn GraphEmbedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    // ------------------------------------------------------------------
    // Incremental sync
    // ------------------------------------------------------------------

    /// Watermark-driven incremental sync plus cache invalidation.
    pub async fn incremental_sync(
        &self,
    ) -> Result<crate::models::kg::GraphSyncSummary, DatabaseError> {
        let summary = self.kg.sync_incremental().await?;
        let _ = self.repository.query_cache_clear().await?;
        Ok(summary)
    }

    /// Syncs a single entity into the graph plus cache invalidation.
    pub async fn sync_entity(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<crate::models::kg_live::EntitySyncResult, DatabaseError> {
        let result = self.kg.sync_entity(node_type, entity_id).await?;
        let _ = self.repository.query_cache_clear().await?;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Semantic `related_to` edges + confidence decay
    // ------------------------------------------------------------------

    /// Public threshold: an edge is persisted only at or above this
    /// cosine similarity, and stale edges below it are pruned.
    pub fn semantic_threshold(&self) -> f64 {
        SEMANTIC_THRESHOLD
    }

    /// Rebuilds semantic `related_to` edges over up to `max_nodes` nodes:
    /// embed node text, compare pairwise cosine similarity, upsert an
    /// edge (confidence = similarity) per pair above the threshold, then
    /// prune stale semantic edges below it. Requires an embedder.
    pub async fn rebuild_semantic_edges(
        &self,
        max_nodes: Option<usize>,
    ) -> Result<SemanticEdgeResult, DatabaseError> {
        let embedder = self
            .embedder
            .as_ref()
            .ok_or_else(|| DatabaseError::InvalidInput("no vector embedder configured".into()))?;

        let limit = max_nodes.unwrap_or(MAX_SEMANTIC_NODES).min(1000);
        let nodes = self
            .kg
            .list_by_type(
                vec![
                    GraphNodeType::Workspace,
                    GraphNodeType::File,
                    GraphNodeType::PlannerReport,
                    GraphNodeType::Execution,
                    GraphNodeType::MemoryRecord,
                    GraphNodeType::AutonomousSession,
                ],
                None,
                Some(limit as u32),
            )
            .await?;

        let mut embedded: Vec<(NodeKey, Vec<f32>)> = Vec::new();
        for node in &nodes {
            if let Some(vector) = embedder.embed(&semantic_text(node)).await {
                embedded.push((key_of(node.node_type, node.entity_id), vector));
            }
        }

        let mut created = 0usize;
        let mut updated = 0usize;
        let mut candidate_pairs = 0usize;
        for i in 0..embedded.len() {
            for j in (i + 1)..embedded.len() {
                let similarity = cosine(&embedded[i].1, &embedded[j].1);
                if similarity >= SEMANTIC_THRESHOLD {
                    candidate_pairs += 1;
                    let evidence = serde_json::json!({
                        "source": "semantic",
                        "method": "cosine",
                        "similarity": (similarity * 100.0).round() / 100.0,
                    });
                    let created_row = self
                        .repository
                        .upsert_semantic_relationship(
                            split_key(&embedded[i].0).0,
                            embedded[i].0 .1,
                            split_key(&embedded[j].0).0,
                            embedded[j].0 .1,
                            similarity,
                            similarity,
                            evidence,
                        )
                        .await?;
                    if created_row {
                        created += 1;
                    } else {
                        updated += 1;
                    }
                }
            }
        }

        let pruned = self
            .repository
            .prune_low_confidence_edges(SEMANTIC_THRESHOLD)
            .await? as usize;
        let _ = self.repository.query_cache_clear().await?;

        Ok(SemanticEdgeResult {
            candidate_pairs,
            created,
            updated,
            pruned,
            threshold: SEMANTIC_THRESHOLD,
        })
    }

    /// Decays semantic edge confidence and prunes edges below the floor
    /// (structural edges are untouched). The per-day exponential policy
    /// is computed here — the repository only reports candidate ages and
    /// writes back the results.
    pub async fn apply_edge_decay(&self) -> Result<EdgeDecaySummary, DatabaseError> {
        let now = Utc::now();
        let mut decayed = 0u64;
        for candidate in self
            .repository
            .decay_candidates(now, DECAY_FRESH_MIN_AGE_DAYS)
            .await?
        {
            let aged = (candidate.confidence * DECAY_FACTOR_PER_DAY.powf(candidate.age_days))
                .clamp(0.0, 1.0);
            let rounded = (aged * 10000.0).round() / 10000.0;
            self.repository
                .update_edge_confidence(candidate.id, rounded, now)
                .await?;
            decayed += 1;
        }
        let pruned = self
            .repository
            .prune_low_confidence_edges(MIN_CONFIDENCE)
            .await?;
        let _ = self.repository.query_cache_clear().await?;
        Ok(EdgeDecaySummary {
            decayed,
            pruned,
            min_confidence: MIN_CONFIDENCE,
        })
    }

    // ------------------------------------------------------------------
    // Persisted query cache
    // ------------------------------------------------------------------

    /// Reads a fresh cached result, if present — `None` on any miss
    /// (absent key, expired TTL, or an un-decodable payload).
    pub async fn cached_get<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, DatabaseError> {
        let Some((created_at, payload, ttl_seconds)) = self.repository.query_cache_get(key).await?
        else {
            return Ok(None);
        };
        let age = Utc::now().signed_duration_since(created_at).num_seconds();
        if age > ttl_seconds {
            return Ok(None);
        }
        serde_json::from_str(&payload)
            .map(Some)
            .map_err(DatabaseError::from)
    }

    /// Stores a query result for `ttl_seconds`.
    pub async fn cached_put<T: Serialize>(
        &self,
        key: &str,
        ttl_seconds: i64,
        value: &T,
    ) -> Result<(), DatabaseError> {
        let payload = serde_json::to_string(value)?;
        self.repository
            .query_cache_put(key, &payload, ttl_seconds)
            .await
    }

    /// Cache bookkeeping for the dashboard.
    pub async fn cache_stats(&self) -> Result<QueryCacheStats, DatabaseError> {
        Ok(QueryCacheStats {
            cached_queries: self.repository.query_cache_count().await?,
        })
    }

    /// Every graph node, optionally scoped to one workspace — the
    /// context intelligence layer reads the full registry for
    /// cross-workspace scoring and scoped snapshots.
    pub async fn graph_nodes(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        self.repository.all_nodes(workspace_id).await
    }

    /// Every relationship in the graph (context intelligence reads the
    /// full adjacency for cross-workspace scoring and path explanations).
    pub async fn graph_edges(&self) -> Result<Vec<KgEdge>, DatabaseError> {
        self.repository.all_edges().await
    }

    // ------------------------------------------------------------------
    // Graph analytics
    // ------------------------------------------------------------------

    /// Computes (or serves from cache) the analytics payload for a scope
    /// — `None` = whole graph, `Some(workspace_id)` = one workspace.
    pub async fn analytics(
        &self,
        workspace_id: Option<Uuid>,
        cached: bool,
    ) -> Result<GraphAnalytics, DatabaseError> {
        let scope = workspace_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "all".to_string());
        let key = format!("analytics:{scope}");
        if cached {
            if let Some(value) = self.cached_get::<GraphAnalytics>(&key).await? {
                let mut value = value;
                value.cached = true;
                return Ok(value);
            }
        }
        let mut value = self.build_analytics(&scope, workspace_id).await?;
        value.cached = false;
        self.cached_put(&key, ANALYTICS_TTL_SECONDS, &value).await?;
        Ok(value)
    }

    async fn build_analytics(
        &self,
        scope: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<GraphAnalytics, DatabaseError> {
        let nodes = self.repository.all_nodes(workspace_id).await?;
        let node_keys: HashSet<NodeKey> = nodes
            .iter()
            .map(|n| key_of(n.node_type, n.entity_id))
            .collect();
        let edges: Vec<KgEdge> = self
            .repository
            .all_edges()
            .await?
            .into_iter()
            .filter(|e| {
                node_keys.contains(&key_of(e.source_node_type, e.source_entity_id))
                    && node_keys.contains(&key_of(e.target_node_type, e.target_entity_id))
            })
            .collect();

        let node_count = nodes.len();
        let edge_count = edges.len();

        let mut in_degree: HashMap<NodeKey, u64> = HashMap::new();
        let mut out_degree: HashMap<NodeKey, u64> = HashMap::new();
        let mut adjacency: HashMap<NodeKey, Vec<NodeKey>> = HashMap::new();
        for node in &nodes {
            let key = key_of(node.node_type, node.entity_id);
            in_degree.entry(key.clone()).or_default();
            out_degree.entry(key.clone()).or_default();
            adjacency.entry(key).or_default();
        }
        for edge in &edges {
            let s = key_of(edge.source_node_type, edge.source_entity_id);
            let t = key_of(edge.target_node_type, edge.target_entity_id);
            *out_degree.entry(s.clone()).or_default() += 1;
            *in_degree.entry(t.clone()).or_default() += 1;
            adjacency.entry(s.clone()).or_default().push(t.clone());
            adjacency.entry(t).or_default().push(s);
        }

        let eigenvector = eigenvector_centrality(&adjacency, &nodes, CENTRALITY_ITERATIONS);

        let total_degree: u64 = in_degree.values().sum();
        let average_degree = if node_count > 0 {
            total_degree as f64 / node_count as f64
        } else {
            0.0
        };
        let density = if node_count > 1 {
            2.0 * edge_count as f64 / (node_count as f64 * (node_count as f64 - 1.0))
        } else {
            0.0
        };

        let mut histogram: HashMap<u64, u64> = HashMap::new();
        let mut centralities: Vec<NodeCentrality> = Vec::with_capacity(node_count);
        for node in &nodes {
            let key = key_of(node.node_type, node.entity_id);
            let deg_in = in_degree.get(&key).copied().unwrap_or(0);
            let deg_out = out_degree.get(&key).copied().unwrap_or(0);
            let total = deg_in + deg_out;
            *histogram.entry(total).or_default() += 1;
            centralities.push(NodeCentrality {
                node_type: node.node_type,
                entity_id: node.entity_id,
                title: node.title.clone(),
                in_degree: deg_in,
                out_degree: deg_out,
                degree_centrality: if node_count > 1 {
                    total as f64 / (node_count as f64 - 1.0)
                } else {
                    0.0
                },
                eigenvector: eigenvector.get(&key).copied().unwrap_or(0.0),
            });
        }
        let mut degree_distribution: Vec<DegreeBucket> = histogram
            .into_iter()
            .map(|(degree, count)| DegreeBucket { degree, count })
            .collect();
        degree_distribution.sort_by_key(|bucket| bucket.degree);
        centralities.sort_by(|a, b| b.eigenvector.total_cmp(&a.eigenvector));
        let top_central_nodes: Vec<NodeCentrality> = centralities.into_iter().take(10).collect();

        let components = connected_components(&nodes, &adjacency);
        let workspace_importance = if workspace_id.is_none() {
            workspace_importance(&nodes, &edges, &eigenvector)
        } else {
            Vec::new()
        };

        Ok(GraphAnalytics {
            scope: scope.to_string(),
            node_count: node_count as u64,
            edge_count: edge_count as u64,
            average_degree,
            density,
            degree_distribution,
            top_central_nodes,
            components,
            workspace_importance,
            cached: false,
            computed_at: Utc::now(),
        })
    }

    // ------------------------------------------------------------------
    // Multi-hop context expansion
    // ------------------------------------------------------------------

    /// Expands an entity's context across up to `hops` edges, keeping the
    /// strongest accumulated path score per node
    /// (`∏ edge.weight × confidence`, halved per hop). Cached.
    pub async fn expand_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        hops: Option<usize>,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<MultiHopContext, DatabaseError> {
        let source = self
            .kg
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))?;
        let hops = hops.unwrap_or(2).min(MAX_HOPS);
        let limit = limit.unwrap_or(100).min(200);

        let key = format!("expand:{node_type}:{entity_id}:{hops}:{limit}");
        if cached {
            if let Some(value) = self.cached_get::<MultiHopContext>(&key).await? {
                return Ok(value);
            }
        }

        let walk = self
            .multi_hop_walk(key_of(node_type, entity_id), hops)
            .await?;
        let mut related: Vec<MultiHopHit> = Vec::new();
        for (key, (score, hop, relationship, via)) in walk {
            let node = self.get_node(key.clone()).await?;
            let Some(node) = node else { continue };
            let reason = match via {
                Some(via_key) => {
                    let via_title = self
                        .get_node(via_key)
                        .await?
                        .map(|n| n.title)
                        .unwrap_or_default();
                    if hop == 1 {
                        format!("Direct {relationship} connection")
                    } else {
                        format!("Reached through '{via_title}'")
                    }
                }
                None => format!("Connected within {hop} hop(s)"),
            };
            related.push(MultiHopHit {
                node,
                relationship_type: Some(relationship),
                reason,
                weight: score,
                hop,
            });
            if related.len() >= 200 {
                break;
            }
        }
        related.sort_by(|a, b| b.weight.total_cmp(&a.weight));
        related.truncate(limit);

        let result = MultiHopContext { source, related };
        self.cached_put(&key, QUERY_TTL_SECONDS, &result).await?;
        Ok(result)
    }

    /// Recommends related work around one node: 2- and 3-hop neighbors
    /// scored by path strength plus, when an embedder is configured,
    /// semantic similarity. Cached.
    pub async fn recommendations(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<Vec<GraphRecommendation>, DatabaseError> {
        let source = self
            .kg
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))?;
        let limit = limit.unwrap_or(10).min(MAX_RECOMMENDATIONS);

        let key = format!("recommend:{node_type}:{entity_id}:{limit}");
        if cached {
            if let Some(value) = self.cached_get::<Vec<GraphRecommendation>>(&key).await? {
                return Ok(value);
            }
        }

        let source_key = key_of(node_type, entity_id);
        let walk = self
            .multi_hop_walk(source_key.clone(), MAX_RECOMMENDATION_HOPS)
            .await?;
        let direct: HashSet<NodeKey> = walk
            .iter()
            .filter(|(_, (_, hop, _, _))| *hop == 1)
            .map(|(key, _)| key.clone())
            .collect();

        let source_text = semantic_text(&source);
        let mut recs: Vec<GraphRecommendation> = Vec::new();
        for (key, (score, hop, _, via)) in &walk {
            if *hop < 2 || direct.contains(key) {
                continue;
            }
            let Some(node) = self.get_node(key.clone()).await? else {
                continue;
            };

            let mut via_node = None;
            let mut final_score = *score;
            let mut reason = format!("Related {hop} hop(s) away");
            if let Some(via_key) = via {
                if let Some(via_kg) = self.get_node(via_key.clone()).await? {
                    reason = format!("Connected through '{}'", via_kg.title);
                    via_node = Some(via_kg);
                }
            }

            if let Some(embedder) = &self.embedder {
                if let (Some(source_vector), Some(candidate_vector)) = (
                    embedder.embed(&source_text).await,
                    embedder.embed(&semantic_text(&node)).await,
                ) {
                    let similarity = cosine(&source_vector, &candidate_vector);
                    if similarity > final_score && similarity > 0.35 {
                        final_score = similarity;
                        reason = "Semantically similar".to_string();
                    }
                }
            }

            recs.push(GraphRecommendation {
                node,
                score: final_score,
                reason,
                hop: *hop,
                via: via_node,
            });
        }
        recs.sort_by(|a, b| b.score.total_cmp(&a.score));
        recs.truncate(limit);

        self.cached_put(&key, QUERY_TTL_SECONDS, &recs).await?;
        Ok(recs)
    }

    /// The relationship inspector: one node plus every incident edge with
    /// its resolved neighbor and confidence, for frontend visualization.
    pub async fn relationship_details(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<RelationshipDetails, DatabaseError> {
        let node = self
            .kg
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))?;
        let key = key_of(node_type, entity_id);
        let mut relationships = Vec::new();
        for edge in self.repository.all_edges().await? {
            let s = key_of(edge.source_node_type, edge.source_entity_id);
            let t = key_of(edge.target_node_type, edge.target_entity_id);
            let neighbor_key = if s == key {
                t.clone()
            } else if t == key {
                s.clone()
            } else {
                continue;
            };
            if let Some(neighbor) = self.get_node(neighbor_key).await? {
                relationships.push(RelationshipDetail { edge, neighbor });
            }
        }
        relationships.sort_by(|a, b| b.edge.confidence.total_cmp(&a.edge.confidence));
        Ok(RelationshipDetails {
            node,
            relationships,
        })
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn get_node(&self, key: NodeKey) -> Result<Option<KgNode>, DatabaseError> {
        self.kg.get_node(split_key(&key).0, key.1).await
    }

    /// Best-path walk from `source` over all persisted edges (treated as
    /// undirected): every reachable node within `hops` edges, with the
    /// strongest accumulated score (`∏ edge.weight × confidence`).
    async fn multi_hop_walk(
        &self,
        source: NodeKey,
        hops: usize,
    ) -> Result<HashMap<NodeKey, (f64, usize, GraphRelationshipType, Option<NodeKey>)>, DatabaseError>
    {
        let edges = self.repository.all_edges().await?;
        let mut adjacency: HashMap<NodeKey, Vec<(NodeKey, f64, f64, GraphRelationshipType)>> =
            HashMap::new();
        for edge in &edges {
            let s = key_of(edge.source_node_type, edge.source_entity_id);
            let t = key_of(edge.target_node_type, edge.target_entity_id);
            adjacency.entry(s.clone()).or_default().push((
                t.clone(),
                edge.weight,
                edge.confidence,
                edge.relationship_type,
            ));
            adjacency.entry(t).or_default().push((
                s,
                edge.weight,
                edge.confidence,
                edge.relationship_type,
            ));
        }

        // Level-relaxation DP over hop depth. `level[h]` holds the
        // strongest accumulation reaching each node in exactly `h` hops;
        // `best` keeps the strongest path across all depths so shallower
        // and deeper routes compete fairly. Because every edge factor is
        // in (0, 1] and the hop decay halves each hop beyond the first,
        // revisiting a node only ever weakens a path, so this finds the
        // optimal ≤ `hops`-edge walk for each node.
        let mut best: HashMap<NodeKey, (f64, usize, GraphRelationshipType, Option<NodeKey>)> =
            HashMap::new();
        // (acc_score, relationship, via) per node at the current depth.
        let mut current: HashMap<NodeKey, (f64, GraphRelationshipType, NodeKey)> = HashMap::new();
        current.insert(
            source.clone(),
            (1.0, GraphRelationshipType::RelatedTo, source.clone()),
        );
        for depth in 1..=hops {
            let hop_decay = 0.5f64.powf((depth - 1) as f64);
            let mut next: HashMap<NodeKey, (f64, GraphRelationshipType, NodeKey)> = HashMap::new();
            for (key, (acc, _, _)) in &current {
                let Some(neighbors) = adjacency.get(key) else {
                    continue;
                };
                for (target, weight, confidence, kind) in neighbors {
                    if *target == source {
                        continue;
                    }
                    let score = acc * weight * confidence * hop_decay;
                    let improved = next
                        .get(target)
                        .map_or(true, |(existing, _, _)| score > *existing);
                    if improved {
                        next.insert(target.clone(), (score, *kind, key.clone()));
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            for (node, (score, kind, via)) in &next {
                let entry = (*score, depth, *kind, Some(via.clone()));
                if best.get(node).map_or(true, |existing| existing.0 < *score) {
                    best.insert(node.clone(), entry);
                }
            }
            current = next;
        }
        best.remove(&source);
        Ok(best)
    }
}

fn key_of(node_type: GraphNodeType, entity_id: Uuid) -> NodeKey {
    (node_type.as_str().to_string(), entity_id)
}

fn split_key(key: &NodeKey) -> (GraphNodeType, Uuid) {
    (
        GraphNodeType::from_str(&key.0).unwrap_or(GraphNodeType::Workspace),
        key.1,
    )
}

fn semantic_text(node: &KgNode) -> String {
    match &node.summary {
        Some(summary) if !summary.is_empty() => format!("{} {}", node.title, summary),
        _ => node.title.clone(),
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b) {
        dot += (*x as f64) * (*y as f64);
        norm_a += (*x as f64) * (*x as f64);
        norm_b += (*y as f64) * (*y as f64);
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

fn eigenvector_centrality(
    adjacency: &HashMap<NodeKey, Vec<NodeKey>>,
    nodes: &[KgNode],
    iterations: usize,
) -> HashMap<NodeKey, f64> {
    if nodes.is_empty() {
        return HashMap::new();
    }
    let mut x: HashMap<NodeKey, f64> = HashMap::new();
    for node in nodes {
        x.insert(key_of(node.node_type, node.entity_id), 1.0);
    }
    let all_keys: Vec<NodeKey> = x.keys().cloned().collect();
    for _ in 0..iterations {
        let mut next: HashMap<NodeKey, f64> = HashMap::new();
        let mut norm = 0.0f64;
        for key in &all_keys {
            let mut sum = 0.0;
            if let Some(neighbors) = adjacency.get(key) {
                for neighbor in neighbors {
                    sum += x.get(neighbor).copied().unwrap_or(0.0);
                }
            }
            next.insert(key.clone(), sum);
            norm += sum * sum;
        }
        let norm = norm.sqrt();
        let len = next.len().max(1);
        for value in next.values_mut() {
            *value = if norm > 0.0 {
                *value / norm
            } else {
                1.0 / len as f64
            };
        }
        x = next;
    }
    x
}

fn connected_components(
    nodes: &[KgNode],
    adjacency: &HashMap<NodeKey, Vec<NodeKey>>,
) -> Vec<GraphComponent> {
    let mut seen: HashSet<NodeKey> = HashSet::new();
    let mut components = Vec::new();
    for node in nodes {
        let start = key_of(node.node_type, node.entity_id);
        if !seen.insert(start.clone()) {
            continue;
        }
        let mut members: Vec<NodeKey> = Vec::new();
        let mut stack = vec![start];
        while let Some(key) = stack.pop() {
            members.push(key.clone());
            if let Some(neighbors) = adjacency.get(&key) {
                for neighbor in neighbors {
                    if seen.insert(neighbor.clone()) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }
        let mut type_counts: HashMap<String, u64> = HashMap::new();
        for key in &members {
            *type_counts.entry(key.0.clone()).or_default() += 1;
        }
        let mut node_types: Vec<TypeCount> = type_counts
            .into_iter()
            .map(|(name, count)| TypeCount {
                name,
                count: count as i64,
            })
            .collect();
        node_types.sort_by_key(|entry| std::cmp::Reverse(entry.count));
        let member_titles: Vec<String> = nodes
            .iter()
            .filter(|n| members.contains(&key_of(n.node_type, n.entity_id)))
            .map(|n| n.title.clone())
            .take(MAX_COMPONENT_SAMPLES)
            .collect();
        components.push(GraphComponent {
            index: 0,
            size: members.len() as u64,
            node_types,
            member_titles,
        });
    }
    components.sort_by_key(|component| std::cmp::Reverse(component.size));
    for (index, component) in components.iter_mut().enumerate() {
        component.index = index;
    }
    components
}

fn workspace_importance(
    nodes: &[KgNode],
    edges: &[KgEdge],
    eigenvector: &HashMap<NodeKey, f64>,
) -> Vec<WorkspaceImportance> {
    let mut ws_of: HashMap<NodeKey, Option<Uuid>> = HashMap::new();
    let mut names: HashMap<Uuid, String> = HashMap::new();
    for node in nodes {
        let key = key_of(node.node_type, node.entity_id);
        ws_of.insert(key.clone(), node.workspace_id);
        if node.node_type == GraphNodeType::Workspace {
            if let Some(ws) = node.workspace_id {
                names.insert(ws, node.title.clone());
            }
        }
    }

    let mut groups: HashMap<Uuid, WorkspaceImportance> = HashMap::new();
    for node in nodes {
        if let Some(ws) = node.workspace_id {
            groups
                .entry(ws)
                .or_insert_with(|| WorkspaceImportance {
                    workspace_id: ws,
                    name: names.get(&ws).cloned().unwrap_or_default(),
                    importance: 0.0,
                    node_count: 0,
                    edge_count: 0,
                    weight_sum: 0.0,
                })
                .node_count += 1;
        }
    }

    for edge in edges {
        let s_key = key_of(edge.source_node_type, edge.source_entity_id);
        let t_key = key_of(edge.target_node_type, edge.target_entity_id);
        let weight = edge.weight * edge.confidence;
        match (
            ws_of.get(&s_key).copied().flatten(),
            ws_of.get(&t_key).copied().flatten(),
        ) {
            (Some(ws), Some(ws2)) if ws == ws2 => {
                groups.entry(ws).or_default().edge_count += 1;
                groups.entry(ws).or_default().weight_sum += weight;
            }
            (Some(ws), None) => {
                groups.entry(ws).or_default().edge_count += 1;
                groups.entry(ws).or_default().weight_sum += weight;
            }
            (None, Some(ws)) => {
                groups.entry(ws).or_default().edge_count += 1;
                groups.entry(ws).or_default().weight_sum += weight;
            }
            (Some(ws), Some(ws2)) => {
                groups.entry(ws).or_default().edge_count += 1;
                groups.entry(ws).or_default().weight_sum += weight / 2.0;
                groups.entry(ws2).or_default().edge_count += 1;
                groups.entry(ws2).or_default().weight_sum += weight / 2.0;
            }
            _ => {}
        }
    }

    for node in nodes {
        if let Some(ws) = node.workspace_id {
            if let Some(mass) = eigenvector.get(&key_of(node.node_type, node.entity_id)) {
                groups.entry(ws).or_default().importance += mass;
            }
        }
    }
    let mut list: Vec<WorkspaceImportance> = groups.into_values().collect();
    for entry in &mut list {
        entry.importance += 0.1 * entry.weight_sum;
    }
    list.sort_by(|a, b| b.importance.total_cmp(&a.importance));
    list
}

#[cfg(test)]
#[path = "kg_live_service_tests.rs"]
mod tests;

impl std::fmt::Debug for KgLiveService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KgLiveService")
            .field("kg", &self.kg)
            .field("repository", &self.repository)
            .finish_non_exhaustive()
    }
}

/// The memory vector system embeds via its cache tiers (in-memory LRU +
/// durable provider), which is exactly the embedding surface semantic
/// `related_to` edges need. The adapter lives here — not in `models` —
/// to keep the model layer free of `copilot` dependencies.
#[async_trait::async_trait]
impl GraphEmbedder for crate::copilot::memory::vector::MemoryVectorSystem {
    async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        MemoryVectorSystem::embed(self, text).await
    }
}
