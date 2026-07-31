use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdge, GraphEdgeType, GraphStats, GraphView, NodeDetails};
use crate::models::search::SearchEntityType;
use crate::repositories::GraphRepository;
use uuid::Uuid;

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
        let edges = self
            .graph_repository
            .get_edges(workspace_id, edge_types.as_deref())
            .await?;

        Ok(GraphView { nodes, edges })
    }

    /// Fetches details for a node including its related edges.
    pub async fn get_node_details(
        &self,
        entity_id: Uuid,
        entity_type: SearchEntityType,
    ) -> Result<NodeDetails, DatabaseError> {
        let node = self
            .graph_repository
            .get_node(entity_id, entity_type)
            .await?
            .ok_or_else(|| DatabaseError::not_found("node", entity_id.to_string()))?;

        let related_edges = self
            .graph_repository
            .get_edges_for_node(entity_id, entity_type)
            .await?;

        Ok(NodeDetails {
            node,
            related_edges,
        })
    }

    /// Placeholder for edge inference logic.
    /// In a real system, this might look for co-occurrence in search results or timeline events.
    pub async fn infer_edges(&self, workspace_id: Uuid) -> Result<(), DatabaseError> {
        tracing::info!(workspace_id = %workspace_id, "triggering edge inference");

        // Get all nodes for this workspace
        let nodes = self.graph_repository.list_nodes(Some(workspace_id)).await?;

        // Create edges between files in the same workspace (CoOccurrence)
        let file_nodes: Vec<_> = nodes
            .iter()
            .filter(|n| n.entity_type == SearchEntityType::File)
            .collect();

        for i in 0..file_nodes.len() {
            for j in (i + 1)..file_nodes.len() {
                let source = &file_nodes[i];
                let target = &file_nodes[j];

                // Calculate weight based on path similarity
                let weight = self.calculate_path_similarity(&source.title, &target.title);

                if weight > 0.3 {
                    self.graph_repository
                        .upsert_edge(
                            source.entity_type,
                            source.entity_id,
                            target.entity_type,
                            target.entity_id,
                            GraphEdgeType::CoOccurrence,
                            weight,
                            workspace_id,
                            Some(format!(
                                "{{\"inferred\": true, \"similarity\": {}}}",
                                weight
                            )),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Calculate similarity between two file paths.
    fn calculate_path_similarity(&self, path1: &str, path2: &str) -> f64 {
        let parts1: Vec<&str> = path1.split('/').collect();
        let parts2: Vec<&str> = path2.split('/').collect();

        let common_parts = parts1.iter().filter(|p| parts2.contains(p)).count();

        let max_parts = parts1.len().max(parts2.len());
        if max_parts == 0 {
            return 0.0;
        }

        common_parts as f64 / max_parts as f64
    }

    /// Incrementally update graph for a specific file or workspace.
    pub async fn refresh_graph(&self, workspace_id: Uuid) -> Result<(), DatabaseError> {
        tracing::info!(workspace_id = %workspace_id, "refreshing graph");

        // Re-infer edges without deleting existing ones
        self.infer_edges(workspace_id).await
    }

    /// Get workspace relationship graph (files and their connections).
    pub async fn get_workspace_relationship_graph(
        &self,
        workspace_id: Uuid,
    ) -> Result<GraphView, DatabaseError> {
        self.get_graph(
            Some(workspace_id),
            Some(vec![
                GraphEdgeType::CoOccurrence,
                GraphEdgeType::ExplicitReference,
            ]),
        )
        .await
    }

    /// Get related files for a specific file node.
    pub async fn get_related_files(&self, file_id: Uuid) -> Result<Vec<GraphEdge>, DatabaseError> {
        self.graph_repository
            .get_edges_for_node(file_id, SearchEntityType::File)
            .await
    }

    pub async fn get_graph_stats(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<GraphStats, DatabaseError> {
        self.graph_repository.get_graph_stats(workspace_id).await
    }

    #[allow(clippy::too_many_arguments)]
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
        self.graph_repository
            .upsert_edge(
                source_entity_type,
                source_entity_id,
                target_entity_type,
                target_entity_id,
                edge_type,
                weight,
                workspace_id,
                metadata,
            )
            .await
    }
}
