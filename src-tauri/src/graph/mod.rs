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
use crate::models::kg_live::{
    EdgeDecaySummary, EntitySyncResult, GraphAnalytics, GraphRecommendation, MultiHopContext,
    QueryCacheStats, RelationshipDetails, SemanticEdgeResult,
};
use crate::models::search::SearchEntityType;
use crate::services::{GraphService, KgLiveService, KgService};
use uuid::Uuid;

/// Facade for Knowledge Graph operations.
#[derive(Debug, Clone)]
pub struct GraphEngine {
    graph_service: GraphService,
    kg_service: Option<KgService>,
    kg_live_service: Option<KgLiveService>,
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
}
