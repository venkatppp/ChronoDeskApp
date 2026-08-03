use tauri::State;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::graph::GraphEngine;
use crate::models::graph::{GraphEdgeType, GraphStats, GraphView, NodeDetails};
use crate::models::kg::{
    ContextDiscovery, GraphNodeType, GraphPath, GraphSyncSummary, KgNode, KgStats, KgSubgraph,
};
use crate::models::kg_live::{
    EdgeDecaySummary, EntitySyncResult, GraphAnalytics, GraphRecommendation, MultiHopContext,
    QueryCacheStats, RelationshipDetails, SemanticEdgeResult,
};
use crate::models::search::SearchEntityType;
use crate::services::GraphService;

/// Fetches a section of the knowledge graph (legacy `graph_edges` view).
#[tauri::command]
pub async fn get_graph(
    service: State<'_, GraphService>,
    workspace_id: Option<Uuid>,
    edge_types: Option<Vec<GraphEdgeType>>,
) -> Result<GraphView, DatabaseError> {
    service.get_graph(workspace_id, edge_types).await
}

/// Fetches details for a specific node and its neighbors (legacy view).
#[tauri::command]
pub async fn get_node_details(
    service: State<'_, GraphService>,
    entity_id: Uuid,
    entity_type: SearchEntityType,
) -> Result<NodeDetails, DatabaseError> {
    service.get_node_details(entity_id, entity_type).await
}

/// Returns graph statistics for a workspace (legacy view).
#[tauri::command]
pub async fn get_graph_stats(
    service: State<'_, GraphService>,
    workspace_id: Option<Uuid>,
) -> Result<GraphStats, DatabaseError> {
    service.get_graph_stats(workspace_id).await
}

// ----------------------------------------------------------------------
// RC-8 Knowledge Graph commands
// ----------------------------------------------------------------------

/// Rebuilds the knowledge graph from all six source aggregates.
#[tauri::command]
pub async fn graph_sync(engine: State<'_, GraphEngine>) -> Result<GraphSyncSummary, DatabaseError> {
    engine.sync_graph().await
}

/// Searches knowledge graph nodes by title/summary.
#[tauri::command]
pub async fn graph_search(
    engine: State<'_, GraphEngine>,
    query: String,
    node_types: Option<Vec<GraphNodeType>>,
    limit: Option<u32>,
) -> Result<Vec<KgNode>, DatabaseError> {
    engine.search_graph_nodes(&query, node_types, limit).await
}

/// Extracts the BFS subgraph around a node (traversal).
#[tauri::command]
pub async fn graph_subgraph(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
    depth: Option<usize>,
) -> Result<KgSubgraph, DatabaseError> {
    engine.graph_subgraph(node_type, entity_id, depth).await
}

/// Finds the shortest path between two knowledge graph nodes.
#[tauri::command]
pub async fn graph_path(
    engine: State<'_, GraphEngine>,
    source_node_type: GraphNodeType,
    source_entity_id: Uuid,
    target_node_type: GraphNodeType,
    target_entity_id: Uuid,
    max_depth: Option<usize>,
) -> Result<Option<GraphPath>, DatabaseError> {
    engine
        .graph_path(
            source_node_type,
            source_entity_id,
            target_node_type,
            target_entity_id,
            max_depth,
        )
        .await
}

/// Discovers ranked context around one entity.
#[tauri::command]
pub async fn graph_context(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
    limit: Option<usize>,
) -> Result<ContextDiscovery, DatabaseError> {
    engine.graph_context(node_type, entity_id, limit).await
}

/// Aggregate statistics for the RC-8 knowledge graph.
#[tauri::command]
pub async fn graph_kg_stats(engine: State<'_, GraphEngine>) -> Result<KgStats, DatabaseError> {
    engine.graph_stats().await
}

/// Lists nodes of the given types, optionally scoped to a workspace.
#[tauri::command]
pub async fn graph_nodes(
    engine: State<'_, GraphEngine>,
    node_types: Vec<GraphNodeType>,
    workspace_id: Option<Uuid>,
    limit: Option<u32>,
) -> Result<Vec<KgNode>, DatabaseError> {
    engine.graph_nodes(node_types, workspace_id, limit).await
}

// ----------------------------------------------------------------------
// RC-8 M2: Live Knowledge Graph commands
// ----------------------------------------------------------------------

/// Watermark-driven incremental graph sync (event-driven updates).
#[tauri::command]
pub async fn graph_incremental_sync(
    engine: State<'_, GraphEngine>,
) -> Result<GraphSyncSummary, DatabaseError> {
    engine.incremental_sync().await
}

/// Syncs a single entity into the graph.
#[tauri::command]
pub async fn graph_sync_entity(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
) -> Result<EntitySyncResult, DatabaseError> {
    engine.sync_graph_entity(node_type, entity_id).await
}

/// Rebuilds semantic `related_to` edges from node embeddings.
#[tauri::command]
pub async fn graph_rebuild_semantic_edges(
    engine: State<'_, GraphEngine>,
    max_nodes: Option<usize>,
) -> Result<SemanticEdgeResult, DatabaseError> {
    engine.rebuild_semantic_edges(max_nodes).await
}

/// Ages semantic edge confidence and prunes edges below the floor.
#[tauri::command]
pub async fn graph_apply_edge_decay(
    engine: State<'_, GraphEngine>,
) -> Result<EdgeDecaySummary, DatabaseError> {
    engine.apply_edge_decay().await
}

/// Graph analytics for the dashboard (cached per scope).
#[tauri::command]
pub async fn graph_analytics(
    engine: State<'_, GraphEngine>,
    workspace_id: Option<Uuid>,
    cached: Option<bool>,
) -> Result<GraphAnalytics, DatabaseError> {
    engine
        .graph_analytics(workspace_id, cached.unwrap_or(true))
        .await
}

/// Multi-hop context expansion around one entity.
#[tauri::command]
pub async fn graph_expand_context(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
    hops: Option<usize>,
    limit: Option<usize>,
    cached: Option<bool>,
) -> Result<MultiHopContext, DatabaseError> {
    engine
        .graph_expand_context(node_type, entity_id, hops, limit, cached.unwrap_or(true))
        .await
}

/// Related-work recommendations around one entity.
#[tauri::command]
pub async fn graph_recommendations(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
    limit: Option<usize>,
    cached: Option<bool>,
) -> Result<Vec<GraphRecommendation>, DatabaseError> {
    engine
        .graph_recommendations(node_type, entity_id, limit, cached.unwrap_or(true))
        .await
}

/// The relationship inspector payload for one node.
#[tauri::command]
pub async fn graph_relationship_details(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
) -> Result<RelationshipDetails, DatabaseError> {
    engine
        .graph_relationship_details(node_type, entity_id)
        .await
}

/// Query-cache bookkeeping for the dashboard.
#[tauri::command]
pub async fn graph_cache_stats(
    engine: State<'_, GraphEngine>,
) -> Result<QueryCacheStats, DatabaseError> {
    engine.graph_cache_stats().await
}
