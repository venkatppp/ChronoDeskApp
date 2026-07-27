use tauri::State;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdgeType, GraphStats, GraphView, NodeDetails};
use crate::models::search::SearchEntityType;
use crate::services::GraphService;

/// Fetches a section of the knowledge graph.
#[tauri::command]
pub async fn get_graph(
    service: State<'_, GraphService>,
    workspace_id: Option<Uuid>,
    edge_types: Option<Vec<GraphEdgeType>>,
) -> Result<GraphView, DatabaseError> {
    service.get_graph(workspace_id, edge_types).await
}

/// Fetches details for a specific node and its neighbors.
#[tauri::command]
pub async fn get_node_details(
    service: State<'_, GraphService>,
    entity_id: Uuid,
    entity_type: SearchEntityType,
) -> Result<NodeDetails, DatabaseError> {
    service.get_node_details(entity_id, entity_type).await
}

/// Returns graph statistics for a workspace.
#[tauri::command]
pub async fn get_graph_stats(
    service: State<'_, GraphService>,
    workspace_id: Option<Uuid>,
) -> Result<GraphStats, DatabaseError> {
    service.get_graph_stats(workspace_id).await
}
