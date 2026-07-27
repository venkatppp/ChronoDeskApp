use uuid::Uuid;
use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdgeType, GraphStats, GraphView, NodeDetails, GraphEdge};
use crate::models::search::SearchEntityType;
use crate::repositories::GraphRepository;

/// Service for knowledge graph logic.
#[derive(Debug, Clone)]
pub struct GraphService {
    graph_repository: GraphRepository,
}

impl GraphService {
    pub fn new(graph_repository: GraphRepository) -> Self {
        Self { graph_repository }
    }

    /// Fetches nodes and edges for a workspace.
    pub async fn get_graph(
        &self,
        workspace_id: Option<Uuid>,
        edge_types: Option<Vec<GraphEdgeType>>,
    ) -> Result<GraphView, DatabaseError> {
        let nodes = self.graph_repository.list_nodes(workspace_id).await?;
        let edges = self.graph_repository.get_edges(workspace_id, edge_types.as_deref()).await?;

        Ok(GraphView { nodes, edges })
    }

    /// Fetches details for a node including its related edges.
    pub async fn get_node_details(
        &self,
        entity_id: Uuid,
        entity_type: SearchEntityType,
    ) -> Result<NodeDetails, DatabaseError> {
        let node = self.graph_repository.get_node(entity_id, entity_type).await?
            .ok_or_else(|| DatabaseError::not_found("node", entity_id.to_string()))?;

        let related_edges = self.graph_repository.get_edges_for_node(entity_id, entity_type).await?;

        Ok(NodeDetails {
            node,
            related_edges,
        })
    }

    /// Placeholder for edge inference logic.
    /// In a real system, this might look for co-occurrence in search results or timeline events.
    pub async fn infer_edges(&self, workspace_id: Uuid) -> Result<(), DatabaseError> {
        // Example: Look for files in the same workspace and create a CoOccurrence edge if they share search terms.
        // For Phase 4, we just provide the scaffold for this logic.
        tracing::info!(workspace_id = %workspace_id, "triggering edge inference");
        
        // This is where one would call search_repository to find related entities 
        // and then graph_repository to upsert_edge.
        
        Ok(())
    }

    pub async fn get_graph_stats(&self, workspace_id: Option<Uuid>) -> Result<GraphStats, DatabaseError> {
        self.graph_repository.get_graph_stats(workspace_id).await
    }

    pub async fn upsert_edge(
        &self,
        source_entity_type: SearchEntityType,
        source_entity_id: Uuid,
        target_entity_type: SearchEntityType,
        target_entity_id: Uuid,
        edge_type: GraphEdgeType,
        weight: f64,
        workspace_id: Uuid,
        metadata: Option<String>,
    ) -> Result<GraphEdge, DatabaseError> {
        self.graph_repository.upsert_edge(
            source_entity_type,
            source_entity_id,
            target_entity_type,
            target_entity_id,
            edge_type,
            weight,
            workspace_id,
            metadata
        ).await
    }
}
