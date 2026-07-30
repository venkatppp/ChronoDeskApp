use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::graph::{GraphEdge, GraphEdgeRow, GraphEdgeType, GraphNode, GraphStats};
use crate::models::search::SearchEntityType;

/// Repository for managing knowledge graph nodes and edges.
#[derive(Debug, Clone)]
pub struct GraphRepository {
    pool: SqlitePool,
}

impl GraphRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetches a single node's basic information.
    pub async fn get_node(
        &self,
        id: Uuid,
        entity_type: SearchEntityType,
    ) -> Result<Option<GraphNode>, DatabaseError> {
        match entity_type {
            SearchEntityType::Workspace => {
                let row: Option<(Uuid, String, Uuid)> =
                    sqlx::query_as("SELECT id, name, id FROM workspaces WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await?;

                Ok(row.map(|r| GraphNode {
                    entity_type: SearchEntityType::Workspace,
                    entity_id: r.0,
                    title: r.1,
                    workspace_id: r.2,
                }))
            }
            SearchEntityType::File => {
                let row: Option<(Uuid, String, Uuid)> =
                    sqlx::query_as("SELECT id, path_or_url, workspace_id FROM files WHERE id = ?")
                        .bind(id)
                        .fetch_optional(&self.pool)
                        .await?;

                Ok(row.map(|r| GraphNode {
                    entity_type: SearchEntityType::File,
                    entity_id: r.0,
                    title: r.1,
                    workspace_id: r.2,
                }))
            }
        }
    }

    /// Lists all nodes (workspaces and files) in a given workspace, or all nodes if `workspace_id` is None.
    pub async fn list_nodes(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<GraphNode>, DatabaseError> {
        let mut nodes = Vec::new();

        // Workspaces
        let ws_rows: Vec<(Uuid, String, Uuid)> = if let Some(ws_id) = workspace_id {
            sqlx::query_as("SELECT id, name, id FROM workspaces WHERE id = ?")
                .bind(ws_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT id, name, id FROM workspaces")
                .fetch_all(&self.pool)
                .await?
        };
        for r in ws_rows {
            nodes.push(GraphNode {
                entity_type: SearchEntityType::Workspace,
                entity_id: r.0,
                title: r.1,
                workspace_id: r.2,
            });
        }

        // Files
        let f_rows: Vec<(Uuid, String, Uuid)> = if let Some(ws_id) = workspace_id {
            sqlx::query_as("SELECT id, path_or_url, workspace_id FROM files WHERE workspace_id = ?")
                .bind(ws_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT id, path_or_url, workspace_id FROM files")
                .fetch_all(&self.pool)
                .await?
        };
        for r in f_rows {
            nodes.push(GraphNode {
                entity_type: SearchEntityType::File,
                entity_id: r.0,
                title: r.1,
                workspace_id: r.2,
            });
        }

        Ok(nodes)
    }

    /// Fetches all edges, optionally filtered by workspace and edge types.
    pub async fn get_edges(
        &self,
        workspace_id: Option<Uuid>,
        edge_types: Option<&[GraphEdgeType]>,
    ) -> Result<Vec<GraphEdge>, DatabaseError> {
        let mut sql = "SELECT * FROM graph_edges WHERE 1=1".to_string();

        if workspace_id.is_some() {
            sql.push_str(" AND workspace_id = ?");
        }

        if let Some(types) = edge_types {
            if !types.is_empty() {
                let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
                sql.push_str(&format!(" AND edge_type IN ({})", placeholders.join(",")));
            }
        }

        let mut query = sqlx::query_as::<_, GraphEdgeRow>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        if let Some(types) = edge_types {
            for t in types {
                query = query.bind(t.as_str());
            }
        }
        let rows: Vec<GraphEdgeRow> = query.fetch_all(&self.pool).await?;

        rows.into_iter().map(GraphEdge::try_from).collect()
    }

    /// Fetches all edges connected to a specific node (either as source or target).
    pub async fn get_edges_for_node(
        &self,
        entity_id: Uuid,
        entity_type: SearchEntityType,
    ) -> Result<Vec<GraphEdge>, DatabaseError> {
        let rows: Vec<GraphEdgeRow> = sqlx::query_as(
            "SELECT * FROM graph_edges 
             WHERE (source_entity_id = ? AND source_entity_type = ?)
                OR (target_entity_id = ? AND target_entity_type = ?)",
        )
        .bind(entity_id)
        .bind(entity_type.as_str())
        .bind(entity_id)
        .bind(entity_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(GraphEdge::try_from).collect()
    }

    /// Upserts a graph edge atomically.
    ///
    /// Uses `INSERT ... ON CONFLICT DO UPDATE` with SQLite's `RETURNING *`
    /// — a single atomic query rather than a SELECT-then-INSERT/UPDATE
    /// pattern, eliminating the TOCTOU race window where concurrent callers
    /// could both see "no existing row" and create duplicate edges. The
    /// `ON CONFLICT` target is the unique index from migration 0007 on
    /// (source_entity_type, source_entity_id, target_entity_type,
    /// target_entity_id, edge_type, workspace_id).
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
        let id = Uuid::new_v4();
        let now = Utc::now();

        let row: GraphEdgeRow = sqlx::query_as(
            "INSERT INTO graph_edges 
             (id, source_entity_type, source_entity_id, target_entity_type, target_entity_id, edge_type, weight, workspace_id, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_entity_type, source_entity_id, target_entity_type, target_entity_id, edge_type, workspace_id)
             DO UPDATE SET weight = excluded.weight, metadata = excluded.metadata, updated_at = excluded.updated_at
             RETURNING *"
        )
        .bind(id)
        .bind(source_entity_type.as_str())
        .bind(source_entity_id)
        .bind(target_entity_type.as_str())
        .bind(target_entity_id)
        .bind(edge_type.as_str())
        .bind(weight)
        .bind(workspace_id)
        .bind(&metadata)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            edge_id = %row.id,
            source_type = %row.source_entity_type,
            source_id = %row.source_entity_id,
            target_type = %row.target_entity_type,
            target_id = %row.target_entity_id,
            edge_type = %row.edge_type,
            workspace_id = %row.workspace_id,
            "graph edge upserted"
        );

        GraphEdge::try_from(row)
    }

    pub async fn get_edge_by_id(&self, id: Uuid) -> Result<GraphEdge, DatabaseError> {
        let row: GraphEdgeRow = sqlx::query_as("SELECT * FROM graph_edges WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DatabaseError::not_found("graph_edge", id.to_string()))?;

        GraphEdge::try_from(row)
    }

    pub async fn delete_edge(&self, id: Uuid) -> Result<(), DatabaseError> {
        let result = sqlx::query("DELETE FROM graph_edges WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(DatabaseError::not_found("graph_edge", id.to_string()));
        }

        Ok(())
    }

    pub async fn delete_edges_for_workspace(
        &self,
        workspace_id: Uuid,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM graph_edges WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_graph_stats(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<GraphStats, DatabaseError> {
        let mut sql = "SELECT COUNT(*) FROM graph_edges WHERE 1=1".to_string();
        if workspace_id.is_some() {
            sql.push_str(" AND workspace_id = ?");
        }
        let mut count_query = sqlx::query_as(&sql);
        if let Some(ws_id) = workspace_id {
            count_query = count_query.bind(ws_id);
        }
        let edge_count: (i64,) = count_query.fetch_one(&self.pool).await?;

        let mut weight_sql =
            "SELECT AVG(weight), MAX(weight) FROM graph_edges WHERE 1=1".to_string();
        if workspace_id.is_some() {
            weight_sql.push_str(" AND workspace_id = ?");
        }
        let mut weight_query = sqlx::query_as(&weight_sql);
        if let Some(ws_id) = workspace_id {
            weight_query = weight_query.bind(ws_id);
        }
        let weights: (Option<f64>, Option<f64>) = weight_query.fetch_one(&self.pool).await?;

        let nodes = self.list_nodes(workspace_id).await?;
        let node_count = nodes.len() as i64;

        let density = if node_count > 1 {
            edge_count.0 as f64 / (node_count * (node_count - 1)) as f64
        } else {
            0.0
        };

        Ok(GraphStats {
            node_count,
            edge_count: edge_count.0,
            avg_weight: weights.0.unwrap_or(0.0),
            max_weight: weights.1.unwrap_or(0.0),
            density,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::CreateWorkspaceInput;
    use crate::repositories::WorkspaceRepository;

    async fn setup() -> (
        GraphRepository,
        WorkspaceRepository,
        SqlitePool,
        tempfile::TempDir,
    ) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();
        (
            GraphRepository::new(pool.clone()),
            WorkspaceRepository::new(pool.clone()),
            pool,
            temp_dir,
        )
    }

    #[tokio::test]
    async fn upsert_and_retrieve_edge() {
        let (repo, ws_repo, _pool, _guard) = setup().await;

        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        let edge = repo
            .upsert_edge(
                SearchEntityType::Workspace,
                ws.id,
                SearchEntityType::Workspace,
                ws.id,
                GraphEdgeType::CoOccurrence,
                0.5,
                ws.id,
                None,
            )
            .await
            .unwrap();

        assert_eq!(edge.weight, 0.5);

        let edges = repo.get_edges(Some(ws.id), None).await.unwrap();
        assert_eq!(edges.len(), 1);
    }
}
