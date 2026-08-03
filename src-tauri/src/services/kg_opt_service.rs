//! Knowledge Graph Optimization service (RC-8 M4).
//!
//! Business logic for the optimization/scale surfaces, composing
//! [`KgOptRepository`](crate::repositories::KgOptRepository)
//! (pagination + operational ledger SQL) with
//! [`KgService`](crate::services::KgService) (node search/list/stats)
//! and [`KgLiveRepository`](crate::repositories::KgLiveRepository)
//! (persisted query cache):
//!
//! - **Paginated graph loading** — node/edge/neighbor pages for
//!   progressive, virtualized UI loading.
//! - **Ranked search** — keyword hits re-ranked by match quality and
//!   recency, with an explainable score.
//! - **Vector-assisted search** — cosine ranking of node titles against
//!   an embedded query via the memory vector system's embedder.
//! - **Parallel multi-root traversal** — rayon-parallel BFS from several
//!   roots over an in-memory adjacency snapshot.
//! - **Cache trimming + memory statistics** — TTL/oldest-first eviction
//!   and the graph's storage footprint for the dashboard.
//!
//! Every tracked operation records a [`crate::models::kg_opt::QueryMetric`]
//! so the performance dashboard shows real latencies. All SQL lives in
//! repositories; every scoring/traversal policy lives here.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use rayon::prelude::*;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{GraphNodeType, KgEdge, KgNode};
use crate::models::kg_live::{GraphEmbedder, QueryCacheStats};
use crate::models::kg_opt::{
    EdgePage, GraphMemoryStats, NeighborPage, NodePage, ParallelWalkResult, RankedSearchHit,
};
use crate::repositories::{KgLiveRepository, KgOptRepository};
use crate::services::KgService;

/// All six node kinds — the default filter for whole-graph surfaces.
const ALL_NODE_TYPES: [GraphNodeType; 6] = [
    GraphNodeType::Workspace,
    GraphNodeType::File,
    GraphNodeType::PlannerReport,
    GraphNodeType::Execution,
    GraphNodeType::MemoryRecord,
    GraphNodeType::AutonomousSession,
];

/// Default page size for paginated loading.
const DEFAULT_PAGE_SIZE: u32 = 100;
/// Cap on nodes embedded per vector-search call.
const MAX_VECTOR_CANDIDATES: usize = 250;
/// Minimum cosine similarity surfaced by vector search.
const VECTOR_HIT_THRESHOLD: f64 = 0.20;
/// Default maximum BFS depth per root in a parallel traversal.
const DEFAULT_TRAVERSAL_DEPTH: usize = 2;
/// Default per-root node budget in a parallel traversal.
const DEFAULT_TRAVERSAL_BUDGET: usize = 400;
/// Estimated bytes of one node row in memory.
const ESTIMATED_NODE_BYTES: u64 = 512;
/// Estimated bytes of one edge row in memory.
const ESTIMATED_EDGE_BYTES: u64 = 256;

type NodeKey = (String, Uuid);

fn key_of(node_type: GraphNodeType, entity_id: Uuid) -> NodeKey {
    (node_type.as_str().to_string(), entity_id)
}

/// Knowledge Graph Optimization service.
#[derive(Clone)]
pub struct KgOptService {
    kg: KgService,
    opt: KgOptRepository,
    live: KgLiveRepository,
    embedder: Option<Arc<dyn GraphEmbedder>>,
}

impl KgOptService {
    pub fn new(kg: KgService, opt: KgOptRepository, live: KgLiveRepository) -> Self {
        Self {
            kg,
            opt,
            live,
            embedder: None,
        }
    }

    /// Enables vector-assisted search by attaching the memory vector
    /// system's embedder.
    pub fn with_embedder(mut self, embedder: Arc<dyn GraphEmbedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    /// Whether vector-assisted search is available (an embedder is
    /// attached). The benchmark suite uses this to decide which
    /// benchmarks to run.
    pub fn vector_search_available(&self) -> bool {
        self.embedder.is_some()
    }

    // ------------------------------------------------------------------
    // Paginated graph loading
    // ------------------------------------------------------------------

    /// One page of graph nodes (progressive loading).
    pub async fn nodes_page(
        &self,
        node_types: Option<Vec<GraphNodeType>>,
        workspace_id: Option<Uuid>,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<NodePage, DatabaseError> {
        let started = Instant::now();
        let types = node_types.as_deref();
        let page = self
            .opt
            .nodes_page(
                types,
                workspace_id,
                offset,
                limit.unwrap_or(DEFAULT_PAGE_SIZE),
            )
            .await?;
        self.record_metric(
            "paginate_nodes",
            scope_label(workspace_id),
            None,
            started,
            page.nodes.len() as u64,
            false,
        )
        .await;
        Ok(page)
    }

    /// One page of graph edges (progressive loading).
    pub async fn edges_page(
        &self,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<EdgePage, DatabaseError> {
        let started = Instant::now();
        let page = self
            .opt
            .edges_page(offset, limit.unwrap_or(DEFAULT_PAGE_SIZE))
            .await?;
        self.record_metric(
            "paginate_edges",
            None,
            None,
            started,
            page.edges.len() as u64,
            false,
        )
        .await;
        Ok(page)
    }

    /// One page of a node's neighbors (relationship inspector pages).
    pub async fn neighbors_page(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<NeighborPage, DatabaseError> {
        let started = Instant::now();
        let page = self
            .opt
            .neighbors_page(
                node_type,
                entity_id,
                offset,
                limit.unwrap_or(DEFAULT_PAGE_SIZE),
            )
            .await?;
        self.record_metric(
            "paginate_neighbors",
            None,
            Some(format!("{node_type}:{entity_id}")),
            started,
            page.neighbors.len() as u64,
            false,
        )
        .await;
        Ok(page)
    }

    /// Total node count for the given filters — the virtualized list's
    /// `total` before any page is loaded.
    pub async fn nodes_total(
        &self,
        node_types: Option<Vec<GraphNodeType>>,
        workspace_id: Option<Uuid>,
    ) -> Result<u64, DatabaseError> {
        self.opt
            .nodes_page_count(node_types.as_deref(), workspace_id)
            .await
    }

    // ------------------------------------------------------------------
    // Ranked search
    // ------------------------------------------------------------------

    /// Keyword search re-ranked by match quality (title prefix >
    /// title contains > summary contains) with a recency bonus and an
    /// explainable per-hit score in `[0, 1]`.
    pub async fn ranked_search(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<RankedSearchHit>, DatabaseError> {
        let started = Instant::now();
        let limit = limit.unwrap_or(DEFAULT_PAGE_SIZE).min(200);
        let candidates = self
            .kg
            .search_nodes(query, node_types, Some(limit * 2))
            .await?;

        let normalized = query.trim().to_lowercase();
        let mut scored: Vec<RankedSearchHit> = candidates
            .into_iter()
            .map(|node| {
                let title = node.title.to_lowercase();
                let summary = node.summary.as_deref().unwrap_or("").to_lowercase();
                let (score, reason): (f64, &str) = if title.starts_with(&normalized) {
                    (1.0, "Title prefix match")
                } else if title.contains(&normalized) {
                    (0.85, "Title match")
                } else if summary.contains(&normalized) {
                    (0.6, "Summary match")
                } else {
                    (0.3, "Indexed record")
                };
                let recency = if node
                    .updated_at
                    .signed_duration_since(chrono::Utc::now())
                    .num_days()
                    .unsigned_abs()
                    <= 7
                {
                    0.05
                } else {
                    0.0
                };
                RankedSearchHit {
                    score: (score + recency).clamp(0.0, 1.0),
                    reason: reason.to_string(),
                    method: "keyword".to_string(),
                    node,
                }
            })
            .collect();
        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(limit as usize);

        self.record_metric(
            "ranked_search",
            None,
            Some(query.to_string()),
            started,
            scored.len() as u64,
            false,
        )
        .await;
        Ok(scored)
    }

    // ------------------------------------------------------------------
    // Vector-assisted search
    // ------------------------------------------------------------------

    /// Cosine-ranks candidate node titles against the embedded query.
    /// Requires an embedder (the memory vector system); falls back to an
    /// empty result with a recorded metric when none is configured.
    pub async fn vector_search(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<RankedSearchHit>, DatabaseError> {
        let started = Instant::now();
        let Some(embedder) = self.embedder.as_ref() else {
            self.record_metric(
                "vector_search",
                None,
                Some(query.to_string()),
                started,
                0,
                false,
            )
            .await;
            return Ok(Vec::new());
        };

        let Some(query_vector) = embedder.embed(query).await else {
            self.record_metric(
                "vector_search",
                None,
                Some(query.to_string()),
                started,
                0,
                false,
            )
            .await;
            return Ok(Vec::new());
        };

        let types = node_types.unwrap_or_else(|| ALL_NODE_TYPES.to_vec());
        let candidates = self
            .kg
            .list_by_type(types, None, Some(MAX_VECTOR_CANDIDATES as u32))
            .await?;

        let mut scored = Vec::new();
        for node in candidates {
            if let Some(vector) = embedder.embed(&node.title).await {
                let similarity = cosine(&query_vector, &vector);
                if similarity >= VECTOR_HIT_THRESHOLD {
                    scored.push(RankedSearchHit {
                        node,
                        score: (similarity * 100.0).round() / 100.0,
                        method: "vector".to_string(),
                        reason: format!("Semantic similarity {:.0}%", similarity * 100.0),
                    });
                }
            }
        }
        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(limit.unwrap_or(20) as usize);

        self.record_metric(
            "vector_search",
            None,
            Some(query.to_string()),
            started,
            scored.len() as u64,
            false,
        )
        .await;
        Ok(scored)
    }

    // ------------------------------------------------------------------
    // Parallel multi-root traversal
    // ------------------------------------------------------------------

    /// Traverses BFS from several roots in parallel (rayon) over an
    /// in-memory adjacency snapshot, returning the deduplicated union of
    /// reached nodes/edges plus timing. Depth and per-root budget bound
    /// the walk so it stays cheap on large graphs.
    pub async fn parallel_traversal(
        &self,
        roots: Vec<(GraphNodeType, Uuid)>,
        max_depth: Option<usize>,
        budget: Option<usize>,
    ) -> Result<ParallelWalkResult, DatabaseError> {
        let started = Instant::now();
        if roots.is_empty() {
            return Ok(ParallelWalkResult {
                roots: 0,
                nodes: Vec::new(),
                edges: Vec::new(),
                node_count: 0,
                edge_count: 0,
                max_depth: 0,
                duration_ms: 0,
            });
        }
        let max_depth = max_depth.unwrap_or(DEFAULT_TRAVERSAL_DEPTH).min(6);
        let budget = budget.unwrap_or(DEFAULT_TRAVERSAL_BUDGET);

        // Snapshot the graph once, then share it across the parallel
        // walks (read-only borrows; rayon does the parallel fan-out).
        let nodes = self.live.all_nodes(None).await?;
        let edges = self.live.all_edges().await?;
        let node_map: HashMap<NodeKey, KgNode> = nodes
            .iter()
            .map(|n| (key_of(n.node_type, n.entity_id), n.clone()))
            .collect();
        let mut adjacency: HashMap<NodeKey, Vec<(NodeKey, KgEdge)>> = HashMap::new();
        for edge in &edges {
            let src = key_of(edge.source_node_type, edge.source_entity_id);
            let tgt = key_of(edge.target_node_type, edge.target_entity_id);
            adjacency
                .entry(src.clone())
                .or_default()
                .push((tgt.clone(), edge.clone()));
            adjacency.entry(tgt).or_default().push((src, edge.clone()));
        }

        let root_keys: Vec<NodeKey> = roots.iter().map(|(t, id)| key_of(*t, *id)).collect();

        let walks: Vec<(HashSet<NodeKey>, HashSet<Uuid>, usize)> = root_keys
            .par_iter()
            .map(|root| {
                let mut visited: HashSet<NodeKey> = HashSet::new();
                let mut edges_seen: HashSet<Uuid> = HashSet::new();
                let mut frontier: Vec<(NodeKey, usize)> = vec![(root.clone(), 0)];
                let mut max_depth_reached = 0usize;
                while let Some((key, depth)) = frontier.pop() {
                    if depth > max_depth || visited.len() >= budget {
                        continue;
                    }
                    if !visited.insert(key.clone()) {
                        continue;
                    }
                    max_depth_reached = max_depth_reached.max(depth);
                    if depth == max_depth {
                        continue;
                    }
                    if let Some(neighbors) = adjacency.get(&key) {
                        for (neighbor, edge) in neighbors {
                            edges_seen.insert(edge.id);
                            if !visited.contains(neighbor) {
                                frontier.push((neighbor.clone(), depth + 1));
                            }
                        }
                    }
                }
                (visited, edges_seen, max_depth_reached)
            })
            .collect();

        let mut all_nodes: HashSet<NodeKey> = HashSet::new();
        let mut all_edges: HashSet<Uuid> = HashSet::new();
        let mut max_depth_reached = 0usize;
        for (visited, edges_seen, depth) in &walks {
            all_nodes.extend(visited.iter().cloned());
            all_edges.extend(edges_seen.iter().cloned());
            max_depth_reached = max_depth_reached.max(*depth);
        }

        let reached_nodes: Vec<KgNode> = all_nodes
            .iter()
            .filter_map(|key| node_map.get(key).cloned())
            .collect();
        let reached_edges: Vec<KgEdge> = edges
            .into_iter()
            .filter(|edge| all_edges.contains(&edge.id))
            .collect();

        let duration_ms = started.elapsed().as_millis() as u64;
        self.record_metric(
            "parallel_traversal",
            None,
            Some(format!("{} roots", roots.len())),
            started,
            reached_nodes.len() as u64,
            false,
        )
        .await;

        Ok(ParallelWalkResult {
            roots: roots.len(),
            node_count: reached_nodes.len() as u64,
            edge_count: reached_edges.len() as u64,
            nodes: reached_nodes,
            edges: reached_edges,
            max_depth: max_depth_reached,
            duration_ms,
        })
    }

    // ------------------------------------------------------------------
    // Cache trimming + memory statistics
    // ------------------------------------------------------------------

    /// Drops the `n` oldest cached query entries.
    pub async fn trim_cache(&self, n: u64) -> Result<u64, DatabaseError> {
        let removed = self.live.cache_trim(n).await?;
        tracing::info!(removed, "graph query cache trimmed");
        Ok(removed)
    }

    /// Drops every cached entry past its own TTL.
    pub async fn clear_expired_cache(&self) -> Result<u64, DatabaseError> {
        let removed = self.live.cache_clear_expired().await?;
        tracing::info!(removed, "expired graph cache entries cleared");
        Ok(removed)
    }

    /// Query-cache bookkeeping (count + payload bytes).
    pub async fn cache_stats(&self) -> Result<QueryCacheStats, DatabaseError> {
        let cached_queries = self.live.query_cache_count().await?;
        Ok(QueryCacheStats { cached_queries })
    }

    /// Graph memory bookkeeping: registry size, cache footprint, and a
    /// rough in-memory estimate for the dashboard.
    pub async fn memory_stats(&self) -> Result<GraphMemoryStats, DatabaseError> {
        let stats = self.kg.stats().await?;
        let cache_entries = self.live.query_cache_count().await?;
        let cache_size_bytes = self.live.cache_size_bytes().await?;
        let estimated_bytes = stats.node_count as u64 * ESTIMATED_NODE_BYTES
            + stats.edge_count as u64 * ESTIMATED_EDGE_BYTES
            + cache_size_bytes;
        Ok(GraphMemoryStats {
            node_count: stats.node_count as u64,
            edge_count: stats.edge_count as u64,
            cache_entries,
            cache_size_bytes,
            estimated_bytes,
        })
    }

    // ------------------------------------------------------------------
    // Metrics ledger (shared with the health service's passes)
    // ------------------------------------------------------------------

    /// Records one operation metric (best-effort — a failed insert must
    /// never fail the operation that produced it).
    pub async fn record_metric(
        &self,
        operation: &str,
        scope: Option<String>,
        query: Option<String>,
        started: Instant,
        rows: u64,
        hit_cache: bool,
    ) {
        let duration_ms = started.elapsed().as_millis() as u64;
        let _ = self
            .opt
            .insert_query_metric(
                operation,
                scope.as_deref(),
                query.as_deref(),
                duration_ms,
                rows,
                hit_cache,
            )
            .await;
    }

    /// Most recent recorded metrics (performance dashboard).
    pub async fn recent_metrics(
        &self,
        limit: u32,
    ) -> Result<Vec<crate::models::kg_opt::QueryMetric>, DatabaseError> {
        self.opt.recent_query_metrics(limit).await
    }
}

/// Scope label for metrics: `all` or the workspace id.
fn scope_label(workspace_id: Option<Uuid>) -> Option<String> {
    workspace_id.map(|id| id.to_string())
}

/// Cosine similarity between two equal-length vectors.
fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let mut dot = 0f64;
    let mut norm_a = 0f64;
    let mut norm_b = 0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += *x as f64 * *y as f64;
        norm_a += *x as f64 * *x as f64;
        norm_b += *y as f64 * *y as f64;
    }
    if norm_a <= 0.0 || norm_b <= 0.0 {
        return 0.0;
    }
    (dot / (norm_a.sqrt() * norm_b.sqrt())).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "kg_opt_service_tests.rs"]
mod tests;
