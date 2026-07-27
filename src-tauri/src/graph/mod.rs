//! Knowledge Graph Engine (blueprint §4.2, §8).
//!
//! Provides a facade for graph-related operations and orchestration.

use uuid::Uuid;
use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdgeType, GraphStats, GraphView, NodeDetails};
use crate::models::search::SearchEntityType;
use crate::services::GraphService;

/// Facade for Knowledge Graph operations.
#[derive(Debug, Clone)]
pub struct GraphEngine {
    graph_service: GraphService,
}

impl GraphEngine {
    pub fn new(graph_service: GraphService) -> Self {
        Self { graph_service }
    }

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
        self.graph_service.get_node_details(entity_id, entity_type).await
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
}
