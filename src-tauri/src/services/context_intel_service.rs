//! Context Intelligence service (RC-8 M3).
//!
//! Business logic composing the RC-8 knowledge graph
//! ([`KgService`], [`KgLiveService`]) with the
//! [`ContextIntelRepository`](crate::repositories::ContextIntelRepository)
//! (persisted cross-workspace relations, snapshots, clusters) and the
//! workspace registry — behind the shared M2 query cache:
//!
//! - **Context inference engine** — [`ContextIntelService::infer_context`]
//!   ranks an entity's neighbors by structural reachability, semantic
//!   `related_to` confidence and recency, with a per-signal confidence
//!   breakdown.
//! - **Workspace similarity + cross-workspace relationship discovery** —
//!   goal overlap, graph edges bridging two workspaces, and semantic
//!   profile similarity; strong pairs are persisted.
//! - **Goal similarity clustering** — agglomerative clustering of
//!   goal-bearing nodes on shared vocabulary, persisted per scope.
//! - **Knowledge summaries, context snapshots + timeline** — a
//!   per-entity summary card and a persisted, diffed history of a
//!   workspace's graph context.
//! - **Memory + KG context fusion / planner retrieval** — merge
//!   knowledge-graph and memory hits into one ranked list; anchor a
//!   planner goal on its best graph match.
//! - **Context explanation engine** — the shortest graph path (or a
//!   shared-topic fallback) explaining any node pair.
//!
//! All SQL lives in repositories; every score/similarity/cluster policy
//! lives here. Context results share the M2 `graph_query_cache`, so a
//! graph write invalidates them for free.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde::{de::DeserializeOwned, Serialize};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{GraphNodeType, GraphRelationshipType, KgEdge, KgNode};
use crate::models::kg_context::{
    ClusterMember, ConfidenceBreakdown, ContextExplanation, ContextHit, ContextInference,
    ContextIntelSnapshot, ContextSignalType, ContextTimelineEntry, ExplanationLink, FusedContext,
    FusedHit, FusedHitSource, GoalCluster, KnowledgeSummary, PlannerContext, SignalEvidence,
    SummaryPoint, WorkspaceSimilarity, WorkspaceSimilarityResult,
};
use crate::models::kg_live::{GraphEmbedder, QueryCacheStats};
use crate::repositories::{ContextIntelRepository, WorkspaceRepository};
use crate::services::{KgLiveService, KgService};

/// Cache TTL (seconds) for context results — results live until a graph
/// write clears the shared M2 query cache.
const CONTEXT_TTL_SECONDS: i64 = 60;
/// Minimum combined similarity kept/persisted for a workspace pair.
const SIMILARITY_MIN: f64 = 0.18;
/// Max related workspaces returned.
const SIMILARITY_TOP_N: usize = 8;
/// A goal node joins a centroid-cluster above this Jaccard score.
const CLUSTER_THRESHOLD: f64 = 0.30;
/// Max clusters returned per scope.
const CLUSTER_MAX: usize = 8;
/// Max centroid terms shown on a cluster.
const CLUSTER_CENTROID_TERMS: usize = 5;
/// Max context hits in any inference / fusion payload.
const MAX_CONTEXT_HITS: usize = 40;
/// Default inference hit limit.
const DEFAULT_HIT_LIMIT: usize = 20;
/// Timeline / snapshot list cap.
const TIMELINE_LIMIT: usize = 30;
/// Max hops traversed by an explanation path.
const EXPLAIN_MAX_HOPS: usize = 4;
/// Recency threshold (days) for a "fresh" neighbor.
const FRESH_DAYS: i64 = 7;
/// Goal-bearing node types (source aggregates whose titles carry intent).
const GOAL_NODE_TYPES: [GraphNodeType; 3] = [
    GraphNodeType::Execution,
    GraphNodeType::PlannerReport,
    GraphNodeType::MemoryRecord,
];

type NodeKey = (String, Uuid);
/// Undirected adjacency: `neighbors[(key, relationship, weight,
/// confidence, edge)]`.
type Adjacency = HashMap<NodeKey, Vec<(NodeKey, GraphRelationshipType, f64, f64, KgEdge)>>;

/// Context Intelligence service.
#[derive(Clone)]
pub struct ContextIntelService {
    kg: KgService,
    live: KgLiveService,
    workspaces: WorkspaceRepository,
    repository: ContextIntelRepository,
    embedder: Option<Arc<dyn GraphEmbedder>>,
}

impl ContextIntelService {
    pub fn new(
        kg_service: KgService,
        live_service: KgLiveService,
        workspace_repository: WorkspaceRepository,
        repository: ContextIntelRepository,
    ) -> Self {
        Self {
            kg: kg_service,
            live: live_service,
            workspaces: workspace_repository,
            repository,
            embedder: None,
        }
    }

    /// Enables embedding-based signals (workspace profile similarity,
    /// memory-fusion boost) by attaching the memory vector system's
    /// embedder.
    pub fn with_embedder(mut self, embedder: Arc<dyn GraphEmbedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    // ------------------------------------------------------------------
    // Cache helpers (shared M2 query cache)
    // ------------------------------------------------------------------

    async fn cached_get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, DatabaseError> {
        self.live.cached_get::<T>(key).await
    }

    async fn cached_put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), DatabaseError> {
        self.live.cached_put(key, CONTEXT_TTL_SECONDS, value).await
    }

    /// Cache bookkeeping (dashboard).
    pub async fn cache_stats(&self) -> Result<QueryCacheStats, DatabaseError> {
        self.live.cache_stats().await
    }

    // ------------------------------------------------------------------
    // Context inference engine
    // ------------------------------------------------------------------

    /// Ranks an entity's direct neighbors by structural reachability,
    /// semantic `related_to` confidence and recency, and reports the
    /// per-signal confidence breakdown. Cached.
    pub async fn infer_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<ContextInference, DatabaseError> {
        let source = self.node_of(node_type, entity_id).await?;
        let limit = limit.unwrap_or(DEFAULT_HIT_LIMIT).min(MAX_CONTEXT_HITS);
        let key = format!("infer:{node_type}:{entity_id}:{limit}");
        if cached {
            if let Some(value) = self.cached_get::<ContextInference>(&key).await? {
                return Ok(value);
            }
        }

        let details = self.live.relationship_details(node_type, entity_id).await?;
        let now = Utc::now();
        let mut hits: Vec<ContextHit> = Vec::with_capacity(details.relationships.len());
        let mut signal_scores: HashMap<ContextSignalType, Vec<f64>> = HashMap::new();
        for relationship in &details.relationships {
            let neighbor = &relationship.neighbor;
            let is_semantic =
                relationship.edge.relationship_type == GraphRelationshipType::RelatedTo;
            let signal = if is_semantic {
                ContextSignalType::Semantic
            } else {
                ContextSignalType::Structural
            };
            let mut score = if is_semantic {
                relationship.edge.confidence
            } else {
                structural_score(&relationship.edge)
            };
            if now.signed_duration_since(neighbor.updated_at).num_days() <= FRESH_DAYS {
                score = 0.8 * score + 0.1;
            }
            let score = clamp01(score);
            signal_scores.entry(signal).or_default().push(score);
            hits.push(ContextHit {
                node: neighbor.clone(),
                reason: relationship_reason(&relationship.edge).to_string(),
                score,
                signal,
            });
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(limit);

        let result = ContextInference {
            source,
            related: hits,
            confidence: build_breakdown(&signal_scores),
            inferred_at: now,
        };
        self.cached_put(&key, &result).await?;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Workspace similarity + cross-workspace relationship discovery
    // ------------------------------------------------------------------

    /// Similarity between `workspace_id` and every other active workspace
    /// from goal overlap, graph edges bridging the two, and semantic
    /// profile similarity. Strong pairs are persisted. Cached.
    pub async fn workspace_similarity(
        &self,
        workspace_id: Uuid,
        cached: bool,
    ) -> Result<WorkspaceSimilarityResult, DatabaseError> {
        let key = format!("ws-sim:{workspace_id}");
        if cached {
            if let Some(mut value) = self.cached_get::<WorkspaceSimilarityResult>(&key).await? {
                value.cached = true;
                return Ok(value);
            }
        }
        let mut result = self.compute_workspace_similarity(workspace_id).await?;
        result.cached = false;
        self.cached_put(&key, &result).await?;
        Ok(result)
    }

    /// Forced recompute + persistence of cross-workspace relationships
    /// (the frontend "discover" action).
    pub async fn discover_cross_workspace_relationships(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceSimilarityResult, DatabaseError> {
        self.compute_workspace_similarity(workspace_id).await
    }

    async fn compute_workspace_similarity(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceSimilarityResult, DatabaseError> {
        let workspaces = self.workspaces.list_active_workspaces().await?;
        let source_name = match workspaces.iter().find(|ws| ws.id == workspace_id) {
            Some(ws) => ws.name.clone(),
            None => self
                .kg
                .get_node(GraphNodeType::Workspace, workspace_id)
                .await
                .ok()
                .flatten()
                .map(|node| node.title)
                .unwrap_or_default(),
        };

        let nodes = self.live.graph_nodes(None).await?;
        let edges = self.live.graph_edges().await?;
        let by_key = node_map(&nodes);
        let profiles = workspace_profiles(&nodes);

        let mut cross: HashMap<(Uuid, Uuid), f64> = HashMap::new();
        let mut node_count: HashMap<Uuid, usize> = HashMap::new();
        for node in &nodes {
            if let Some(ws) = node.workspace_id {
                *node_count.entry(ws).or_default() += 1;
            }
        }
        for edge in &edges {
            let source_ws = by_key
                .get(&key_of(edge.source_node_type, edge.source_entity_id))
                .and_then(|node| node.workspace_id);
            let target_ws = by_key
                .get(&key_of(edge.target_node_type, edge.target_entity_id))
                .and_then(|node| node.workspace_id);
            if let (Some(a), Some(b)) = (source_ws, target_ws) {
                if a != b {
                    let pair = unordered_pair(a, b);
                    *cross.entry(pair).or_default() += edge.confidence * edge.weight;
                }
            }
        }

        let now = Utc::now();
        let mut related: Vec<WorkspaceSimilarity> = Vec::new();
        for target_ws in &workspaces {
            if target_ws.id == workspace_id {
                continue;
            }
            let Some(source_profile) = profiles.get(&workspace_id) else {
                break;
            };
            let Some(target_profile) = profiles.get(&target_ws.id) else {
                continue;
            };
            let goal = jaccard(&source_profile.terms, &target_profile.terms);
            let pair = unordered_pair(workspace_id, target_ws.id);
            let cross_weight = cross.get(&pair).copied().unwrap_or(0.0);
            let min_participants = node_count
                .get(&workspace_id)
                .copied()
                .unwrap_or(0)
                .min(node_count.get(&target_ws.id).copied().unwrap_or(0));
            let graph = clamp01(cross_weight / (1.0 + min_participants as f64));

            let semantic = match &self.embedder {
                Some(embedder) => match (
                    embedder.embed(&source_profile.text).await,
                    embedder.embed(&target_profile.text).await,
                ) {
                    (Some(a), Some(b)) => Some(clamp01(cosine(&a, &b))),
                    _ => None,
                },
                None => None,
            };

            let (similarity, signals) = combine_signals(goal, graph, semantic);
            if similarity < SIMILARITY_MIN {
                continue;
            }
            let strongest = signals
                .iter()
                .map(|signal| signal.score)
                .fold(0.0f64, f64::max);
            let confidence = clamp01(0.4 + 0.6 * strongest);

            // Persist each unordered pair once, in a deterministic
            // direction, so the repository lookup scans both sides.
            let (canonical_from, canonical_to) = ordered_pair(workspace_id, target_ws.id);
            let signal_values: Vec<serde_json::Value> = signals
                .iter()
                .map(|signal| serde_json::to_value(signal).unwrap_or(serde_json::Value::Null))
                .collect();
            self.repository
                .upsert_workspace_similarity(
                    canonical_from,
                    canonical_to,
                    similarity,
                    confidence,
                    &signal_values,
                )
                .await?;

            related.push(WorkspaceSimilarity {
                source_workspace_id: workspace_id,
                target_workspace_id: target_ws.id,
                target_name: target_ws.name.clone(),
                similarity,
                confidence,
                signals,
                persisted: true,
            });
        }
        related.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
        related.truncate(SIMILARITY_TOP_N);

        Ok(WorkspaceSimilarityResult {
            source_workspace_id: workspace_id,
            source_name,
            related,
            cached: false,
            computed_at: now,
        })
    }

    // ------------------------------------------------------------------
    // Goal similarity clustering
    // ------------------------------------------------------------------

    /// Groups goal-bearing nodes into clusters by shared vocabulary and
    /// persists them per scope (`None` = whole graph). Cached.
    pub async fn goal_clusters(
        &self,
        workspace_id: Option<Uuid>,
        cached: bool,
    ) -> Result<Vec<GoalCluster>, DatabaseError> {
        let key = match workspace_id {
            Some(ws) => format!("clusters:{ws}"),
            None => "clusters:all".to_string(),
        };
        if cached {
            if let Some(value) = self.cached_get::<Vec<GoalCluster>>(&key).await? {
                return Ok(value);
            }
        }
        let nodes = self.live.graph_nodes(workspace_id).await?;
        let clusters = cluster_goals(&nodes, workspace_id);
        self.repository
            .clusters_replace(workspace_id, &clusters)
            .await?;
        let persisted = self.load_clusters(workspace_id).await?;
        self.cached_put(&key, &persisted).await?;
        Ok(persisted)
    }

    async fn load_clusters(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<GoalCluster>, DatabaseError> {
        let rows = self.repository.clusters_list(workspace_id).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let members: Vec<ClusterMember> = serde_json::from_str(&row.members)?;
            let centroid_terms: Vec<String> = serde_json::from_str(&row.centroid)?;
            out.push(GoalCluster {
                id: row.id,
                workspace_id: row.workspace_id,
                name: row.name,
                member_count: row.member_count.max(0) as u64,
                members,
                centroid_terms,
                confidence: row.confidence,
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Knowledge summaries
    // ------------------------------------------------------------------

    /// Builds a knowledge summary card for one entity: connection counts
    /// by relationship type, top neighbors, and recency. Cached.
    pub async fn knowledge_summary(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        cached: bool,
    ) -> Result<KnowledgeSummary, DatabaseError> {
        let source = self.node_of(node_type, entity_id).await?;
        let key = format!("summary:{node_type}:{entity_id}");
        if cached {
            if let Some(value) = self.cached_get::<KnowledgeSummary>(&key).await? {
                return Ok(value);
            }
        }
        let details = self.live.relationship_details(node_type, entity_id).await?;

        let mut kind_counts: HashMap<String, usize> = HashMap::new();
        let mut neighbor_titles: Vec<String> = Vec::with_capacity(details.relationships.len());
        for relationship in &details.relationships {
            *kind_counts
                .entry(relationship.edge.relationship_type.as_str().to_string())
                .or_default() += 1;
            neighbor_titles.push(relationship.neighbor.title.clone());
        }

        let mut points = vec![
            SummaryPoint {
                label: "Entity".to_string(),
                value: source.title.clone(),
                detail: Some(source.node_type.as_str().to_string()),
            },
            SummaryPoint {
                label: "Graph connections".to_string(),
                value: details.relationships.len().to_string(),
                detail: Some(format_breaks(&kind_counts)),
            },
            SummaryPoint {
                label: "Workspace".to_string(),
                value: source
                    .workspace_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "global".to_string()),
                detail: None,
            },
        ];
        if !neighbor_titles.is_empty() {
            let sample: Vec<String> = neighbor_titles[..neighbor_titles.len().min(4)].to_vec();
            points.push(SummaryPoint {
                label: "Neighbors".to_string(),
                value: sample.join(", "),
                detail: None,
            });
        }
        let age = Utc::now().signed_duration_since(source.updated_at);
        points.push(SummaryPoint {
            label: "Last updated".to_string(),
            value: age_human(&age),
            detail: None,
        });

        let confidence = clamp01(0.5 + 0.08 * details.relationships.len() as f64);
        let result = KnowledgeSummary {
            node: source,
            points,
            confidence,
            generated_at: Utc::now(),
        };
        self.cached_put(&key, &result).await?;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Context snapshots + timeline
    // ------------------------------------------------------------------

    /// Persists one graph context snapshot for a workspace (node/edge
    /// counts + knowledge points) and returns the stored record.
    pub async fn context_snapshot_create(
        &self,
        workspace_id: Uuid,
        snapshot_type: &str,
    ) -> Result<ContextIntelSnapshot, DatabaseError> {
        let nodes = self.live.graph_nodes(Some(workspace_id)).await?;
        let edges = workspace_edges(
            &self.live.graph_edges().await?,
            &node_map(&nodes),
            workspace_id,
        );

        let mut hist: HashMap<String, usize> = HashMap::new();
        for node in &nodes {
            *hist.entry(node.node_type.as_str().to_string()).or_default() += 1;
        }
        let summary = snapshot_points(&nodes, &edges);
        let confidence = clamp01(0.4 + 0.05 * nodes.len() as f64);
        let payload = serde_json::json!({ "nodeTypes": hist });
        let id = self
            .repository
            .snapshot_insert(
                workspace_id,
                snapshot_type,
                nodes.len() as i64,
                edges.len() as i64,
                confidence,
                &summary,
                &payload,
            )
            .await?;
        Ok(ContextIntelSnapshot {
            id,
            workspace_id,
            snapshot_type: snapshot_type.to_string(),
            node_count: nodes.len() as u64,
            edge_count: edges.len() as u64,
            confidence,
            summary,
            created_at: Utc::now(),
        })
    }

    /// Most recent snapshots for a workspace, newest first.
    pub async fn context_snapshot_list(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ContextIntelSnapshot>, DatabaseError> {
        let limit = limit.unwrap_or(TIMELINE_LIMIT);
        let rows = self
            .repository
            .snapshot_list(workspace_id, Some(limit))
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let summary: Vec<SummaryPoint> = serde_json::from_str(&row.summary).unwrap_or_default();
            out.push(ContextIntelSnapshot {
                id: row.id,
                workspace_id: row.workspace_id,
                snapshot_type: row.snapshot_type,
                node_count: row.node_count.max(0) as u64,
                edge_count: row.edge_count.max(0) as u64,
                confidence: row.confidence,
                summary,
                created_at: row.created_at,
            });
        }
        Ok(out)
    }

    /// Snapshot history with per-entry deltas against the prior snapshot
    /// (what changed since the last capture).
    pub async fn context_timeline(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ContextTimelineEntry>, DatabaseError> {
        let snapshots = self.context_snapshot_list(workspace_id, limit).await?;
        let mut entries = Vec::with_capacity(snapshots.len());
        for (index, snapshot) in snapshots.iter().enumerate() {
            let previous = snapshots.get(index + 1);
            let (nodes_delta, edges_delta, confidence_delta) = match previous {
                Some(older) => (
                    snapshot.node_count as i64 - older.node_count as i64,
                    snapshot.edge_count as i64 - older.edge_count as i64,
                    snapshot.confidence - older.confidence,
                ),
                None => (0, 0, 0.0),
            };
            entries.push(ContextTimelineEntry {
                snapshot: snapshot.clone(),
                nodes_delta,
                edges_delta,
                confidence_delta,
            });
        }
        Ok(entries)
    }

    // ------------------------------------------------------------------
    // Memory + KG context fusion & planner retrieval
    // ------------------------------------------------------------------

    /// Fuses knowledge-graph hits with memory-record hits into one ranked
    /// list. Memory hits receive an embedding boost when an embedder is
    /// configured. Cached.
    pub async fn fused_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        cached: bool,
    ) -> Result<FusedContext, DatabaseError> {
        let source = self.node_of(node_type, entity_id).await?;
        let key = format!("fused:{node_type}:{entity_id}");
        if cached {
            if let Some(value) = self.cached_get::<FusedContext>(&key).await? {
                return Ok(value);
            }
        }
        let expanded = self
            .live
            .expand_context(node_type, entity_id, Some(2), Some(MAX_CONTEXT_HITS), false)
            .await?;

        let source_text = semantic_text(&source);
        let mut kg_hits: Vec<ContextHit> = Vec::new();
        let mut memory_hits: Vec<ContextHit> = Vec::new();
        for hit in &expanded.related {
            let is_memory = hit.node.node_type == GraphNodeType::MemoryRecord;
            let signal = if is_memory {
                ContextSignalType::Memory
            } else if hit.relationship_type == Some(GraphRelationshipType::RelatedTo) {
                ContextSignalType::Semantic
            } else {
                ContextSignalType::Structural
            };
            let mut score = clamp01(hit.weight);
            let mut reason = hit.reason.clone();
            if is_memory {
                if let Some(embedder) = &self.embedder {
                    if let (Some(node_vector), Some(source_vector)) = (
                        embedder.embed(&semantic_text(&hit.node)).await,
                        embedder.embed(&source_text).await,
                    ) {
                        let semantic = clamp01(cosine(&source_vector, &node_vector));
                        if semantic > score {
                            score = semantic;
                            reason = "Similar memory record".to_string();
                        }
                    }
                }
            }
            let hit = ContextHit {
                node: hit.node.clone(),
                reason,
                score,
                signal,
            };
            if is_memory {
                memory_hits.push(hit);
            } else {
                kg_hits.push(hit);
            }
        }
        kg_hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        memory_hits.sort_by(|a, b| b.score.total_cmp(&a.score));

        let mut fused: Vec<FusedHit> = Vec::with_capacity(kg_hits.len() + memory_hits.len());
        fused.extend(kg_hits.iter().map(|hit| FusedHit {
            node: hit.node.clone(),
            source: FusedHitSource::KnowledgeGraph,
            reason: hit.reason.clone(),
            score: hit.score,
            confidence: hit.score,
        }));
        fused.extend(memory_hits.iter().map(|hit| FusedHit {
            node: hit.node.clone(),
            source: FusedHitSource::Memory,
            reason: hit.reason.clone(),
            score: hit.score,
            confidence: hit.score,
        }));
        fused.sort_by(|a, b| b.score.total_cmp(&a.score));
        fused.truncate(MAX_CONTEXT_HITS);

        let mut signals: HashMap<ContextSignalType, Vec<f64>> = HashMap::new();
        for hit in &kg_hits {
            signals.entry(hit.signal).or_default().push(hit.score);
        }
        signals.insert(
            ContextSignalType::Memory,
            memory_hits.iter().map(|hit| hit.score).collect(),
        );

        let result = FusedContext {
            source,
            kg_hits,
            memory_hits,
            fused,
            confidence: build_breakdown(&signals),
            fused_at: Utc::now(),
        };
        self.cached_put(&key, &result).await?;
        Ok(result)
    }

    /// Graph-assisted planner context retrieval: anchors `goal` on its
    /// best matching graph node and returns the fused context + a summary
    /// the planner can consume. Cached (keyed by goal content).
    pub async fn planner_context(
        &self,
        goal: &str,
        cached: bool,
    ) -> Result<PlannerContext, DatabaseError> {
        let key = format!("planner:{}:{}", goal.len(), simple_hash(goal));
        if cached {
            if let Some(value) = self.cached_get::<PlannerContext>(&key).await? {
                return Ok(value);
            }
        }
        let mut node_types: Vec<GraphNodeType> = GOAL_NODE_TYPES.to_vec();
        node_types.push(GraphNodeType::Workspace);
        let matches = self
            .kg
            .search_nodes(goal, Some(node_types), Some(10))
            .await?;
        let anchor = matches.first().cloned();
        let context = match &anchor {
            Some(anchor_node) => Some(
                self.fused_context(anchor_node.node_type, anchor_node.entity_id, false)
                    .await?,
            ),
            None => None,
        };
        let summary = match (&anchor, &context) {
            (Some(anchor_node), Some(fused)) => format!(
                "Anchored '{goal}' on '{}' ({}) with {} context items",
                anchor_node.title,
                anchor_node.node_type.as_str(),
                fused.fused.len()
            ),
            _ => format!("No knowledge-graph anchor matched '{goal}'"),
        };
        let result = PlannerContext {
            goal: goal.to_string(),
            anchor,
            context,
            summary,
            retrieved_at: Utc::now(),
        };
        self.cached_put(&key, &result).await?;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Context explanation engine
    // ------------------------------------------------------------------

    /// Explains why `source` and `target` are related: the shortest graph
    /// path (each hop an [`ExplanationLink`]), or — when unreachable
    /// within [`EXPLAIN_MAX_HOPS`] — the shared-topic overlap.
    pub async fn explain(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
    ) -> Result<ContextExplanation, DatabaseError> {
        let source = self.node_of(source_type, source_id).await?;
        let target = self.node_of(target_type, target_id).await?;

        let nodes = self.live.graph_nodes(None).await?;
        let edges = self.live.graph_edges().await?;
        let by_key = node_map(&nodes);
        let adjacency = build_adjacency(&edges);
        let path = shortest_path(
            &adjacency,
            key_of(source.node_type, source.entity_id),
            key_of(target.node_type, target.entity_id),
            EXPLAIN_MAX_HOPS,
        );

        if let Some(keys) = path {
            let mut chain = Vec::with_capacity(keys.len().saturating_sub(1));
            for pair in keys.windows(2) {
                let (Some(from), Some(to)) = (by_key.get(&pair[0]), by_key.get(&pair[1])) else {
                    continue;
                };
                let edge = adjacency
                    .get(&pair[0])
                    .and_then(|neighbors| {
                        neighbors.iter().find(|(key, _, _, _, _)| key == &pair[1])
                    })
                    .map(|(_, rel, weight, conf, _)| (*rel, *weight, *conf))
                    .or_else(|| {
                        adjacency.get(&pair[1]).and_then(|neighbors| {
                            neighbors
                                .iter()
                                .find(|(key, _, _, _, _)| key == &pair[0])
                                .map(|(_, rel, weight, conf, _)| (*rel, *weight, *conf))
                        })
                    })
                    .unwrap_or((GraphRelationshipType::RelatedTo, 1.0, 1.0));
                chain.push(ExplanationLink {
                    from: from.clone(),
                    to: to.clone(),
                    relationship_type: edge.0,
                    reason: format!("connected by {}", edge.0.as_str()),
                    score: clamp01(edge.1 * edge.2),
                    confidence: edge.2,
                });
            }
            let path_len = chain.len();
            let confidence = if path_len > 0 {
                (chain.iter().map(|link| link.confidence.ln()).sum::<f64>() / path_len as f64)
                    .exp()
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            let relationships = chain
                .iter()
                .map(|link| link.relationship_type.as_str())
                .collect::<Vec<_>>()
                .join(" → ");
            return Ok(ContextExplanation {
                source,
                target,
                chain,
                summary: format!("Connected in {path_len} hop(s): {relationships}"),
                confidence,
            });
        }

        // No path within the hop cap: fall back to shared vocabulary.
        let shared = terms(&source).intersection(&terms(&target)).count();
        let confidence = clamp01(0.15 + 0.05 * shared as f64);
        Ok(ContextExplanation {
            source,
            target,
            chain: Vec::new(),
            summary: if shared > 0 {
                format!(
                    "No graph path within {EXPLAIN_MAX_HOPS} hops — related by {shared} shared topic term{}",
                    if shared == 1 { "" } else { "s" }
                )
            } else {
                format!("No graph path within {EXPLAIN_MAX_HOPS} hops and no shared topic terms")
            },
            confidence,
        })
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn node_of(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<KgNode, DatabaseError> {
        self.kg
            .get_node(node_type, entity_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph node", entity_id.to_string()))
    }
}

impl std::fmt::Debug for ContextIntelService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextIntelService")
            .field("kg", &self.kg)
            .field("live", &self.live)
            .field("repository", &self.repository)
            .finish_non_exhaustive()
    }
}

// ----------------------------------------------------------------------
// Scoring helpers (pure — directly unit-testable)
// ----------------------------------------------------------------------

/// Structural edges are trusted: base score 0.85, nudged by confidence.
fn structural_score(edge: &KgEdge) -> f64 {
    0.85 + 0.15 * edge.confidence
}

fn relationship_reason(edge: &KgEdge) -> &'static str {
    match edge.relationship_type {
        GraphRelationshipType::Contains => "Direct file connection",
        GraphRelationshipType::RunsIn => "Runs in this workspace",
        GraphRelationshipType::ReportsOn => "Reported on this run",
        GraphRelationshipType::DerivedFrom => "Learned from this run",
        GraphRelationshipType::RelatedTo => "Semantically related",
    }
}

fn clamp01(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// Lowercased, alphanumeric-only token set of `text`.
fn tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| term.len() >= 2)
        .map(ToString::to_string)
        .collect()
}

/// Token Jaccard; 0 when either side is empty.
fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b) {
        dot += f64::from(*x) * f64::from(*y);
        norm_a += f64::from(*x) * f64::from(*x);
        norm_b += f64::from(*y) * f64::from(*y);
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

fn key_of(node_type: GraphNodeType, entity_id: Uuid) -> NodeKey {
    (node_type.as_str().to_string(), entity_id)
}

fn node_map(nodes: &[KgNode]) -> HashMap<NodeKey, KgNode> {
    nodes
        .iter()
        .map(|node| (key_of(node.node_type, node.entity_id), node.clone()))
        .collect()
}

fn unordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn ordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    unordered_pair(a, b)
}

/// Per-workspace profile: goal vocabulary + embedding text.
struct WorkspaceProfile {
    terms: HashSet<String>,
    text: String,
}

fn workspace_profiles(nodes: &[KgNode]) -> HashMap<Uuid, WorkspaceProfile> {
    let mut profiles: HashMap<Uuid, WorkspaceProfile> = HashMap::new();
    for node in nodes {
        let Some(ws) = node.workspace_id else {
            continue;
        };
        let profile = profiles.entry(ws).or_insert_with(|| WorkspaceProfile {
            terms: HashSet::new(),
            text: String::new(),
        });
        if GOAL_NODE_TYPES.contains(&node.node_type) {
            profile.terms.extend(tokens(&node.title));
        }
        profile.text.push_str(&node.title);
        profile.text.push(' ');
        if let Some(summary) = &node.summary {
            profile.text.push_str(summary);
            profile.text.push(' ');
        }
    }
    profiles
}

/// Combines the three workspace-similarity signals into one score plus
/// per-signal evidence. `semantic` is `None` when no embedder exists, in
/// which case the remaining weights are renormalized.
fn combine_signals(goal: f64, graph: f64, semantic: Option<f64>) -> (f64, Vec<SignalEvidence>) {
    let mut signals = vec![
        SignalEvidence {
            signal: ContextSignalType::GoalOverlap,
            score: clamp01(goal),
            detail: format!("goal overlap {:.2}", clamp01(goal)),
        },
        SignalEvidence {
            signal: ContextSignalType::Structural,
            score: clamp01(graph),
            detail: format!("cross-workspace graph bridges {:.2}", clamp01(graph)),
        },
    ];
    if let Some(sem) = semantic {
        signals.push(SignalEvidence {
            signal: ContextSignalType::Semantic,
            score: clamp01(sem),
            detail: format!("semantic profile similarity {:.2}", clamp01(sem)),
        });
    }
    let (sum, weight): (f64, f64) = signals.iter().fold((0.0, 0.0), |(sum, weight), signal| {
        let w = match signal.signal {
            ContextSignalType::GoalOverlap => 0.45,
            ContextSignalType::Structural => 0.30,
            ContextSignalType::Semantic => 0.25,
            _ => 0.0,
        };
        (sum + w * signal.score, weight + w)
    });
    let similarity = if weight > 0.0 {
        clamp01(sum / weight)
    } else {
        0.0
    };
    (similarity, signals)
}

/// Per-signal confidence breakdown: mean score per signal plus the
/// weighted total.
fn build_breakdown(signals: &HashMap<ContextSignalType, Vec<f64>>) -> ConfidenceBreakdown {
    let mean = |key: ContextSignalType| {
        signals
            .get(&key)
            .map(|scores| {
                if scores.is_empty() {
                    0.0
                } else {
                    scores.iter().sum::<f64>() / scores.len() as f64
                }
            })
            .unwrap_or(0.0)
    };
    let structural = mean(ContextSignalType::Structural);
    let semantic = mean(ContextSignalType::Semantic);
    let memory = mean(ContextSignalType::Memory);
    let temporal = mean(ContextSignalType::Temporal);
    let total_weight = structural * 0.5 + semantic * 0.35 + memory * 0.15;
    ConfidenceBreakdown {
        structural,
        semantic,
        temporal,
        memory,
        total: clamp01(total_weight),
    }
}

/// Agglomerative clustering of goal-bearing nodes on centroid Jaccard.
fn cluster_goals(nodes: &[KgNode], workspace_id: Option<Uuid>) -> Vec<GoalCluster> {
    let goal_nodes: Vec<&KgNode> = nodes
        .iter()
        .filter(|node| GOAL_NODE_TYPES.contains(&node.node_type))
        .collect();
    if goal_nodes.is_empty() {
        return Vec::new();
    }

    let mut clusters: Vec<GoalCluster> = Vec::new();
    for node in goal_nodes {
        let terms = tokens(&node.title);
        let mut best: Option<(usize, f64)> = None;
        for (index, cluster) in clusters.iter().enumerate() {
            let centroid: HashSet<String> = cluster.centroid_terms.iter().cloned().collect();
            let score = jaccard(&terms, &centroid);
            if score >= CLUSTER_THRESHOLD && best.map_or(true, |(_, existing)| score > existing) {
                best = Some((index, score));
            }
        }
        match best {
            Some((index, score)) => {
                clusters[index].members.push(ClusterMember {
                    node_type: node.node_type,
                    entity_id: node.entity_id,
                    title: node.title.clone(),
                    workspace_id: node.workspace_id,
                    score,
                });
                clusters[index].centroid_terms = centroid_terms(&clusters[index].members);
            }
            None => clusters.push(GoalCluster {
                id: 0,
                workspace_id,
                name: String::new(),
                member_count: 1,
                members: vec![ClusterMember {
                    node_type: node.node_type,
                    entity_id: node.entity_id,
                    title: node.title.clone(),
                    workspace_id: node.workspace_id,
                    score: 1.0,
                }],
                centroid_terms: centroid_terms(std::slice::from_ref(&ClusterMember {
                    node_type: node.node_type,
                    entity_id: node.entity_id,
                    title: node.title.clone(),
                    workspace_id: node.workspace_id,
                    score: 1.0,
                })),
                confidence: 1.0,
            }),
        }
    }

    for cluster in &mut clusters {
        let cohesion = cluster
            .members
            .iter()
            .map(|member| member.score)
            .sum::<f64>()
            / cluster.members.len() as f64;
        cluster.confidence = clamp01(cohesion);
        cluster.name = if cluster.centroid_terms.is_empty() {
            "Untitled cluster".to_string()
        } else {
            cluster
                .centroid_terms
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(" · ")
                .chars()
                .take(40)
                .collect()
        };
        cluster.member_count = cluster.members.len() as u64;
    }
    clusters.sort_by_key(|cluster| std::cmp::Reverse(cluster.member_count));
    clusters.truncate(CLUSTER_MAX);
    clusters
}

/// Top terms by cumulative frequency across member titles.
fn centroid_terms(members: &[ClusterMember]) -> Vec<String> {
    let mut frequency: HashMap<String, u64> = HashMap::new();
    for member in members {
        for term in tokens(&member.title) {
            *frequency.entry(term).or_default() += 1;
        }
    }
    let mut ranked: Vec<(u64, String)> = frequency
        .into_iter()
        .map(|(term, count)| (count, term))
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .take(CLUSTER_CENTROID_TERMS)
        .map(|(_, term)| term)
        .collect()
}

/// Edges with both endpoints inside `workspace_id`.
fn workspace_edges(
    edges: &[KgEdge],
    by_key: &HashMap<NodeKey, KgNode>,
    workspace_id: Uuid,
) -> Vec<KgEdge> {
    edges
        .iter()
        .filter(|edge| {
            by_key
                .get(&key_of(edge.source_node_type, edge.source_entity_id))
                .and_then(|node| node.workspace_id)
                == Some(workspace_id)
                && by_key
                    .get(&key_of(edge.target_node_type, edge.target_entity_id))
                    .and_then(|node| node.workspace_id)
                    == Some(workspace_id)
        })
        .cloned()
        .collect()
}

fn snapshot_points(nodes: &[KgNode], edges: &[KgEdge]) -> Vec<SummaryPoint> {
    let mut hist: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        *hist.entry(node.node_type.as_str().to_string()).or_default() += 1;
    }
    let top: Vec<String> = {
        let mut ranked: Vec<(usize, String)> = hist
            .into_iter()
            .map(|(kind, count)| (count, kind))
            .collect();
        ranked.sort_by_key(|(count, _)| std::cmp::Reverse(*count));
        ranked.into_iter().take(3).map(|(_, kind)| kind).collect()
    };
    vec![
        SummaryPoint {
            label: "Graph nodes".to_string(),
            value: nodes.len().to_string(),
            detail: Some(top.join(", ")),
        },
        SummaryPoint {
            label: "Graph edges".to_string(),
            value: edges.len().to_string(),
            detail: None,
        },
    ]
}

fn format_breaks(kind_counts: &HashMap<String, usize>) -> String {
    let mut entries: Vec<(usize, &String)> = kind_counts
        .iter()
        .map(|(kind, count)| (*count, kind))
        .collect();
    entries.sort_by_key(|(count, _)| std::cmp::Reverse(*count));
    entries
        .into_iter()
        .map(|(count, kind)| format!("{kind}: {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn age_human(age: &Duration) -> String {
    let days = age.num_days();
    if days >= 30 {
        format!("{} month(s) ago", days / 30)
    } else if days > 0 {
        format!("{days} day(s) ago")
    } else {
        let hours = age.num_hours().max(0);
        if hours > 0 {
            format!("{hours} hour(s) ago")
        } else {
            "just now".to_string()
        }
    }
}

fn semantic_text(node: &KgNode) -> String {
    match &node.summary {
        Some(summary) if !summary.is_empty() => format!("{} {}", node.title, summary),
        _ => node.title.clone(),
    }
}

fn terms(node: &KgNode) -> HashSet<String> {
    tokens(&semantic_text(node))
}

fn build_adjacency(edges: &[KgEdge]) -> Adjacency {
    let mut adjacency: Adjacency = HashMap::new();
    for edge in edges {
        let source_key = key_of(edge.source_node_type, edge.source_entity_id);
        let target_key = key_of(edge.target_node_type, edge.target_entity_id);
        adjacency.entry(source_key.clone()).or_default().push((
            target_key.clone(),
            edge.relationship_type,
            edge.weight,
            edge.confidence,
            edge.clone(),
        ));
        adjacency.entry(target_key).or_default().push((
            source_key,
            edge.relationship_type,
            edge.weight,
            edge.confidence,
            edge.clone(),
        ));
    }
    adjacency
}

/// BFS shortest path (undirected) within `max_hops`, returning the node
/// key chain `source → … → target`.
fn shortest_path(
    adjacency: &Adjacency,
    source: NodeKey,
    target: NodeKey,
    max_hops: usize,
) -> Option<Vec<NodeKey>> {
    if source == target {
        return Some(vec![source]);
    }
    let mut parents: HashMap<NodeKey, NodeKey> = HashMap::new();
    let mut frontier: Vec<NodeKey> = vec![source.clone()];
    let mut seen: HashSet<NodeKey> = HashSet::from([source.clone()]);
    for _ in 0..max_hops {
        let mut next: Vec<NodeKey> = Vec::new();
        for key in &frontier {
            let Some(neighbors) = adjacency.get(key) else {
                continue;
            };
            for (neighbor, _, _, _, _) in neighbors {
                if seen.insert(neighbor.clone()) {
                    parents.insert(neighbor.clone(), key.clone());
                    if neighbor == &target {
                        return Some(reconstruct(&parents, source, target));
                    }
                    next.push(neighbor.clone());
                }
            }
        }
        frontier = next;
        if frontier.is_empty() {
            break;
        }
    }
    None
}

fn reconstruct(
    parents: &HashMap<NodeKey, NodeKey>,
    source: NodeKey,
    target: NodeKey,
) -> Vec<NodeKey> {
    let mut chain = vec![target.clone()];
    let mut current = target;
    while current != source {
        let Some(parent) = parents.get(&current) else {
            break;
        };
        current = parent.clone();
        chain.push(current.clone());
    }
    chain.reverse();
    chain
}

/// FNV-1a hash for planner cache keys.
fn simple_hash(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
#[path = "context_intel_service_tests.rs"]
mod tests;
