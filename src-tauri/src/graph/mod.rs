//! Knowledge Graph Engine (blueprint §4.2, §8).
//!
//! Facade for all graph operations: the Phase 4 `graph_edges` adjacency
//! graph (`GraphService`) and the RC-8 knowledge graph
//! ([`KgService`](crate::services::KgService)) — the typed node registry
//! constructed automatically from workspaces, files, planner reports,
//! executions, memory records, and autonomous sessions.
//!
//! The RC-8 half is optional at construction time (`with_kg_service`)
//! so legacy callers of `GraphEngine::new` keep working; `lib.rs` wires
//! it in.

use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdgeType, GraphStats, GraphView, NodeDetails};
use crate::models::kg::{
    ContextDiscovery, GraphNodeType, GraphPath, GraphSyncSummary, KgNode, KgStats, KgSubgraph,
};
use crate::models::kg_context::{
    ContextExplanation, ContextInference, ContextIntelSnapshot, ContextTimelineEntry, FusedContext,
    GoalCluster, KnowledgeSummary, PlannerContext, WorkspaceSimilarityResult,
};
use crate::models::kg_live::{
    EdgeDecaySummary, EntitySyncResult, GraphAnalytics, GraphRecommendation, MultiHopContext,
    QueryCacheStats, RelationshipDetails, SemanticEdgeResult,
};
use crate::models::kg_opt::{
    BenchmarkSuiteResult, ConsistencyReport, EdgePage, GraphDiagnostics, GraphMemoryStats,
    IntegrityCheckResult, MaintenanceRun, NeighborPage, NodePage, OrphanCleanupResult,
    OrphanSummary, ParallelWalkResult, QueryMetric, RankedSearchHit, RepairResult,
};
use crate::models::search::SearchEntityType;
use crate::services::{
    ContextIntelService, GraphHealthService, GraphService, KgLiveService, KgOptService, KgService,
};
use uuid::Uuid;

/// Facade for Knowledge Graph operations.
#[derive(Clone)]
pub struct GraphEngine {
    graph_service: GraphService,
    kg_service: Option<KgService>,
    kg_live_service: Option<KgLiveService>,
    context_intel_service: Option<ContextIntelService>,
    kg_opt_service: Option<KgOptService>,
    graph_health_service: Option<GraphHealthService>,
}

impl GraphEngine {
    /// Constructs the engine with the legacy graph service only. Use
    /// [`GraphEngine::with_kg_service`] to enable RC-8 knowledge graph
    /// operations.
    pub fn new(graph_service: GraphService) -> Self {
        Self {
            graph_service,
            kg_service: None,
            kg_live_service: None,
            context_intel_service: None,
            kg_opt_service: None,
            graph_health_service: None,
        }
    }

    /// Enables the RC-8 knowledge graph half of this facade.
    pub fn with_kg_service(mut self, kg_service: KgService) -> Self {
        self.kg_service = Some(kg_service);
        self
    }

    /// Enables the RC-8 M2 live knowledge graph half (incremental sync,
    /// semantic edges, analytics, multi-hop context, recommendations).
    pub fn with_kg_live_service(mut self, kg_live_service: KgLiveService) -> Self {
        self.kg_live_service = Some(kg_live_service);
        self
    }

    /// Enables the RC-8 M3 context intelligence half (inference,
    /// workspace similarity, goal clusters, summaries, snapshots,
    /// fusion, planner retrieval, explanations).
    pub fn with_context_intel_service(
        mut self,
        context_intel_service: ContextIntelService,
    ) -> Self {
        self.context_intel_service = Some(context_intel_service);
        self
    }

    /// Enables the RC-8 M4 optimization half (paginated loading,
    /// ranked/vector search, parallel traversal, cache trimming,
    /// memory statistics).
    pub fn with_kg_opt_service(mut self, kg_opt_service: KgOptService) -> Self {
        self.kg_opt_service = Some(kg_opt_service);
        self
    }

    /// Enables the RC-8 M4 health half (integrity, repair, orphans,
    /// consistency, maintenance, benchmarks, diagnostics, metrics).
    pub fn with_graph_health_service(mut self, graph_health_service: GraphHealthService) -> Self {
        self.graph_health_service = Some(graph_health_service);
        self
    }

    fn kg(&self) -> Result<&KgService, DatabaseError> {
        self.kg_service.as_ref().ok_or_else(|| {
            DatabaseError::InvalidInput("knowledge graph engine is not configured".to_string())
        })
    }

    fn kg_live(&self) -> Result<&KgLiveService, DatabaseError> {
        self.kg_live_service.as_ref().ok_or_else(|| {
            DatabaseError::InvalidInput("live knowledge graph engine is not configured".to_string())
        })
    }

    fn context_intel(&self) -> Result<&ContextIntelService, DatabaseError> {
        self.context_intel_service.as_ref().ok_or_else(|| {
            DatabaseError::InvalidInput("context intelligence engine is not configured".to_string())
        })
    }

    fn kg_opt(&self) -> Result<&KgOptService, DatabaseError> {
        self.kg_opt_service.as_ref().ok_or_else(|| {
            DatabaseError::InvalidInput("graph optimization engine is not configured".to_string())
        })
    }

    fn graph_health(&self) -> Result<&GraphHealthService, DatabaseError> {
        self.graph_health_service.as_ref().ok_or_else(|| {
            DatabaseError::InvalidInput("graph health engine is not configured".to_string())
        })
    }

    // ------------------------------------------------------------------
    // Legacy (Phase 4) graph_edges operations
    // ------------------------------------------------------------------

    /// Fetches a section of the knowledge graph.
    pub async fn get_graph(
        &self,
        workspace_id: Option<Uuid>,
        edge_types: Option<Vec<GraphEdgeType>>,
    ) -> Result<GraphView, DatabaseError> {
        self.graph_service.get_graph(workspace_id, edge_types).await
    }

    /// Fetches details for a specific node and its neighbors.
    pub async fn get_node_details(
        &self,
        entity_id: Uuid,
        entity_type: SearchEntityType,
        _workspace_id: Option<Uuid>,
    ) -> Result<NodeDetails, DatabaseError> {
        self.graph_service
            .get_node_details(entity_id, entity_type)
            .await
    }

    /// Triggers background edge inference logic.
    pub async fn infer_edges(&self, workspace_id: Uuid) -> Result<(), DatabaseError> {
        self.graph_service.infer_edges(workspace_id).await
    }

    /// Returns graph statistics for a workspace.
    pub async fn get_workspace_graph_stats(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<GraphStats, DatabaseError> {
        self.graph_service.get_graph_stats(workspace_id).await
    }

    // ------------------------------------------------------------------
    // RC-8 Knowledge Graph operations
    // ------------------------------------------------------------------

    /// Rebuilds the knowledge graph from all six source aggregates.
    pub async fn sync_graph(&self) -> Result<GraphSyncSummary, DatabaseError> {
        self.kg()?.sync_graph().await
    }

    /// Searches knowledge graph nodes by title/summary substring.
    pub async fn search_graph_nodes(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        self.kg()?.search_nodes(query, node_types, limit).await
    }

    /// Extracts the BFS subgraph around a node.
    pub async fn graph_subgraph(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        depth: Option<usize>,
    ) -> Result<KgSubgraph, DatabaseError> {
        self.kg()?.subgraph(node_type, entity_id, depth).await
    }

    /// Finds the shortest path between two nodes.
    pub async fn graph_path(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
        max_depth: Option<usize>,
    ) -> Result<Option<GraphPath>, DatabaseError> {
        self.kg()?
            .find_path(source_type, source_id, target_type, target_id, max_depth)
            .await
    }

    /// Discovers ranked context around one entity.
    pub async fn graph_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
    ) -> Result<ContextDiscovery, DatabaseError> {
        self.kg()?
            .discover_context(node_type, entity_id, limit)
            .await
    }

    /// Aggregate knowledge graph statistics.
    pub async fn graph_stats(&self) -> Result<KgStats, DatabaseError> {
        self.kg()?.stats().await
    }

    /// Lists nodes of the given types (the frontend entity filter).
    pub async fn graph_nodes(
        &self,
        node_types: Vec<GraphNodeType>,
        workspace_id: Option<Uuid>,
        limit: Option<u32>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        self.kg()?
            .list_by_type(node_types, workspace_id, limit)
            .await
    }

    // ------------------------------------------------------------------
    // RC-8 M2: Live Knowledge Graph operations
    // ------------------------------------------------------------------

    /// Watermark-driven incremental sync (only aggregates whose source
    /// rows changed are rebuilt), invalidating the query cache.
    pub async fn incremental_sync(&self) -> Result<GraphSyncSummary, DatabaseError> {
        self.kg_live()?.incremental_sync().await
    }

    /// Syncs one entity into the graph (event-driven update).
    pub async fn sync_graph_entity(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<EntitySyncResult, DatabaseError> {
        self.kg_live()?.sync_entity(node_type, entity_id).await
    }

    /// Rebuilds semantic `related_to` edges from node embeddings.
    pub async fn rebuild_semantic_edges(
        &self,
        max_nodes: Option<usize>,
    ) -> Result<SemanticEdgeResult, DatabaseError> {
        self.kg_live()?.rebuild_semantic_edges(max_nodes).await
    }

    /// Ages semantic edge confidence and prunes below the floor.
    pub async fn apply_edge_decay(&self) -> Result<EdgeDecaySummary, DatabaseError> {
        self.kg_live()?.apply_edge_decay().await
    }

    /// Graph analytics for the dashboard (cached per scope).
    pub async fn graph_analytics(
        &self,
        workspace_id: Option<Uuid>,
        cached: bool,
    ) -> Result<GraphAnalytics, DatabaseError> {
        self.kg_live()?.analytics(workspace_id, cached).await
    }

    /// Multi-hop context expansion around one entity.
    pub async fn graph_expand_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        hops: Option<usize>,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<MultiHopContext, DatabaseError> {
        self.kg_live()?
            .expand_context(node_type, entity_id, hops, limit, cached)
            .await
    }

    /// Related-work recommendations around one entity.
    pub async fn graph_recommendations(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<Vec<GraphRecommendation>, DatabaseError> {
        self.kg_live()?
            .recommendations(node_type, entity_id, limit, cached)
            .await
    }

    /// The relationship inspector payload for one node.
    pub async fn graph_relationship_details(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<RelationshipDetails, DatabaseError> {
        self.kg_live()?
            .relationship_details(node_type, entity_id)
            .await
    }

    /// Query-cache bookkeeping for the dashboard.
    pub async fn graph_cache_stats(&self) -> Result<QueryCacheStats, DatabaseError> {
        self.kg_live()?.cache_stats().await
    }

    // ------------------------------------------------------------------
    // RC-8 M3: Context Intelligence operations
    // ------------------------------------------------------------------

    /// Ranks an entity's graph neighbors by structural reachability,
    /// semantic confidence and recency, with a per-signal confidence
    /// breakdown (cached).
    pub async fn infer_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: Option<usize>,
        cached: bool,
    ) -> Result<ContextInference, DatabaseError> {
        self.context_intel()?
            .infer_context(node_type, entity_id, limit, cached)
            .await
    }

    /// Similarity between one workspace and every other active workspace
    /// (cached); strong pairs are persisted.
    pub async fn workspace_similarity(
        &self,
        workspace_id: Uuid,
        cached: bool,
    ) -> Result<WorkspaceSimilarityResult, DatabaseError> {
        self.context_intel()?
            .workspace_similarity(workspace_id, cached)
            .await
    }

    /// Forced recompute + persistence of cross-workspace relationships.
    pub async fn discover_cross_workspace_relationships(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceSimilarityResult, DatabaseError> {
        self.context_intel()?
            .discover_cross_workspace_relationships(workspace_id)
            .await
    }

    /// Goal-similarity clusters, persisted per scope (cached).
    pub async fn goal_clusters(
        &self,
        workspace_id: Option<Uuid>,
        cached: bool,
    ) -> Result<Vec<GoalCluster>, DatabaseError> {
        self.context_intel()?
            .goal_clusters(workspace_id, cached)
            .await
    }

    /// Knowledge summary card for one entity (cached).
    pub async fn knowledge_summary(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        cached: bool,
    ) -> Result<KnowledgeSummary, DatabaseError> {
        self.context_intel()?
            .knowledge_summary(node_type, entity_id, cached)
            .await
    }

    /// Persists one graph context snapshot for a workspace.
    pub async fn context_snapshot_create(
        &self,
        workspace_id: Uuid,
        snapshot_type: &str,
    ) -> Result<ContextIntelSnapshot, DatabaseError> {
        self.context_intel()?
            .context_snapshot_create(workspace_id, snapshot_type)
            .await
    }

    /// Most recent snapshots for a workspace, newest first.
    pub async fn context_snapshot_list(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ContextIntelSnapshot>, DatabaseError> {
        self.context_intel()?
            .context_snapshot_list(workspace_id, limit)
            .await
    }

    /// Snapshot history with per-entry deltas against the prior snapshot.
    pub async fn context_timeline(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<ContextTimelineEntry>, DatabaseError> {
        self.context_intel()?
            .context_timeline(workspace_id, limit)
            .await
    }

    /// Fuses knowledge-graph hits with memory-record hits (cached).
    pub async fn fused_context(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        cached: bool,
    ) -> Result<FusedContext, DatabaseError> {
        self.context_intel()?
            .fused_context(node_type, entity_id, cached)
            .await
    }

    /// Graph-assisted planner context retrieval anchored on `goal`
    /// (cached, keyed by goal content).
    pub async fn planner_context(
        &self,
        goal: &str,
        cached: bool,
    ) -> Result<PlannerContext, DatabaseError> {
        self.context_intel()?.planner_context(goal, cached).await
    }

    /// Explains why two nodes are related: the shortest graph path, or a
    /// shared-topic fallback when unreachable within the hop cap.
    pub async fn explain(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
    ) -> Result<ContextExplanation, DatabaseError> {
        self.context_intel()?
            .explain(source_type, source_id, target_type, target_id)
            .await
    }

    // ------------------------------------------------------------------
    // RC-8 M4: Knowledge Graph Optimization & Scale operations
    // ------------------------------------------------------------------

    /// One page of graph nodes (progressive loading).
    pub async fn graph_nodes_page(
        &self,
        node_types: Option<Vec<GraphNodeType>>,
        workspace_id: Option<Uuid>,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<NodePage, DatabaseError> {
        self.kg_opt()?
            .nodes_page(node_types, workspace_id, offset, limit)
            .await
    }

    /// One page of graph edges (progressive loading).
    pub async fn graph_edges_page(
        &self,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<EdgePage, DatabaseError> {
        self.kg_opt()?.edges_page(offset, limit).await
    }

    /// One page of a node's neighbors (relationship inspector pages).
    pub async fn graph_neighbors_page(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        offset: u64,
        limit: Option<u32>,
    ) -> Result<NeighborPage, DatabaseError> {
        self.kg_opt()?
            .neighbors_page(node_type, entity_id, offset, limit)
            .await
    }

    /// Total node count for the given filters (virtualized list header).
    pub async fn graph_nodes_total(
        &self,
        node_types: Option<Vec<GraphNodeType>>,
        workspace_id: Option<Uuid>,
    ) -> Result<u64, DatabaseError> {
        self.kg_opt()?.nodes_total(node_types, workspace_id).await
    }

    /// Keyword search re-ranked by match quality and recency.
    pub async fn graph_ranked_search(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<RankedSearchHit>, DatabaseError> {
        self.kg_opt()?.ranked_search(query, node_types, limit).await
    }

    /// Cosine-ranked vector search over node titles.
    pub async fn graph_vector_search(
        &self,
        query: &str,
        node_types: Option<Vec<GraphNodeType>>,
        limit: Option<u32>,
    ) -> Result<Vec<RankedSearchHit>, DatabaseError> {
        self.kg_opt()?.vector_search(query, node_types, limit).await
    }

    /// Parallel (rayon) multi-root BFS traversal.
    pub async fn graph_parallel_traverse(
        &self,
        roots: Vec<(GraphNodeType, Uuid)>,
        max_depth: Option<usize>,
        budget: Option<usize>,
    ) -> Result<ParallelWalkResult, DatabaseError> {
        self.kg_opt()?
            .parallel_traversal(roots, max_depth, budget)
            .await
    }

    /// Drops the `n` oldest cached query entries.
    pub async fn graph_cache_trim(&self, n: u64) -> Result<u64, DatabaseError> {
        self.kg_opt()?.trim_cache(n).await
    }

    /// Drops every cached entry past its TTL.
    pub async fn graph_clear_expired_cache(&self) -> Result<u64, DatabaseError> {
        self.kg_opt()?.clear_expired_cache().await
    }

    /// Graph memory statistics (registry + cache footprint).
    pub async fn graph_memory_stats(&self) -> Result<GraphMemoryStats, DatabaseError> {
        self.kg_opt()?.memory_stats().await
    }

    /// Most recent recorded operation metrics.
    pub async fn graph_recent_metrics(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<QueryMetric>, DatabaseError> {
        self.kg_opt()?.recent_metrics(limit.unwrap_or(20)).await
    }

    /// Runs the four integrity scans and persists new findings.
    pub async fn graph_integrity_check(&self) -> Result<IntegrityCheckResult, DatabaseError> {
        self.graph_health()?.integrity_check().await
    }

    /// Repairs every detectable graph problem.
    pub async fn graph_repair(&self) -> Result<RepairResult, DatabaseError> {
        self.graph_health()?.repair().await
    }

    /// Read-only orphan bookkeeping.
    pub async fn graph_orphan_summary(&self) -> Result<OrphanSummary, DatabaseError> {
        self.graph_health()?.orphan_summary().await
    }

    /// Removes every orphan edge and dangling workspace node.
    pub async fn graph_orphan_cleanup(&self) -> Result<OrphanCleanupResult, DatabaseError> {
        self.graph_health()?.orphan_cleanup().await
    }

    /// Runs the five consistency probes.
    pub async fn graph_consistency_report(&self) -> Result<ConsistencyReport, DatabaseError> {
        self.graph_health()?.consistency_report().await
    }

    /// Most recent maintenance runs, newest first.
    pub async fn graph_maintenance_runs(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<MaintenanceRun>, DatabaseError> {
        self.graph_health()?
            .recent_maintenance_runs(limit.unwrap_or(10))
            .await
    }

    /// Runs + persists the micro-benchmark suite.
    pub async fn graph_benchmark_suite(
        &self,
        suite_name: Option<String>,
    ) -> Result<BenchmarkSuiteResult, DatabaseError> {
        self.graph_health()?.benchmark_suite(suite_name).await
    }

    /// The full graph performance/health diagnostics bundle.
    pub async fn graph_diagnostics(&self) -> Result<GraphDiagnostics, DatabaseError> {
        self.graph_health()?.diagnostics().await
    }
}
