//! Knowledge Graph repository (RC-8 M1).
//!
//! Owns every SQL statement behind the RC-8 knowledge graph: the
//! `graph_nodes` / `graph_relationships` CRUD, the source extraction
//! queries that feed the automatic construction pass, and the statistics
//! rollups. Traversal/ranking logic lives in `services::KgService` —
//! this module only moves rows in and out of SQLite.
//!
//! All construction writes are idempotent upserts keyed on the natural
//! graph keys, so `sync` is safe to run repeatedly.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{
    GraphNodeType, GraphRelationshipType, GraphSource, KgEdge, KgEdgeRow, KgNode, KgNodeRow,
    KgStats, TypeCount,
};

/// Repository for the RC-8 knowledge graph (`graph_nodes` +
/// `graph_relationships`).
#[derive(Debug, Clone)]
pub struct KgRepository {
    pool: SqlitePool,
}

impl KgRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Node CRUD
    // ------------------------------------------------------------------

    /// Upserts one graph node. Returns `true` when a row was created,
    /// `false` when an existing row was updated.
    pub async fn upsert_node(
        &self,
        node_type: GraphNodeType,
        source: &GraphSource,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now();
        let metadata = serde_json::to_string(&source.metadata).unwrap_or_else(|_| "{}".into());

        let row: KgNodeRow = sqlx::query_as(
            "INSERT INTO graph_nodes
                 (node_type, entity_id, title, workspace_id, summary, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(node_type, entity_id) DO UPDATE SET
                 title = excluded.title,
                 workspace_id = excluded.workspace_id,
                 summary = excluded.summary,
                 metadata = excluded.metadata,
                 updated_at = excluded.updated_at
             RETURNING *",
        )
        .bind(node_type.as_str())
        .bind(source.entity_id)
        .bind(&source.title)
        .bind(source.workspace_id)
        .bind(&source.summary)
        .bind(&metadata)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.created_at == row.updated_at)
    }

    pub async fn get_node(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<Option<KgNode>, DatabaseError> {
        let row: Option<KgNodeRow> =
            sqlx::query_as("SELECT * FROM graph_nodes WHERE node_type = ? AND entity_id = ?")
                .bind(node_type.as_str())
                .bind(entity_id)
                .fetch_optional(&self.pool)
                .await?;

        row.map(KgNode::try_from).transpose()
    }

    /// Lists nodes, optionally scoped to a workspace and/or a set of
    /// node types, newest-first, capped at `limit` (default 500).
    pub async fn list_nodes(
        &self,
        workspace_id: Option<Uuid>,
        node_types: Option<&[GraphNodeType]>,
        limit: Option<u32>,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        let mut sql = String::from("SELECT * FROM graph_nodes WHERE 1=1");
        if workspace_id.is_some() {
            sql.push_str(" AND workspace_id = ?");
        }
        if let Some(types) = node_types {
            if !types.is_empty() {
                let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
                sql.push_str(&format!(" AND node_type IN ({})", placeholders.join(",")));
            }
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT ?");

        let mut query = sqlx::query_as::<_, KgNodeRow>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        if let Some(types) = node_types {
            for t in types {
                query = query.bind(t.as_str());
            }
        }
        query = query.bind(limit.unwrap_or(500) as i64);

        let rows: Vec<KgNodeRow> = query.fetch_all(&self.pool).await?;
        rows.into_iter().map(KgNode::try_from).collect()
    }

    /// Case-insensitive substring search over node titles and summaries.
    pub async fn search_nodes(
        &self,
        query: &str,
        node_types: Option<&[GraphNodeType]>,
        limit: u32,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        let mut sql =
            String::from("SELECT * FROM graph_nodes WHERE (title LIKE ? OR summary LIKE ?)");
        if let Some(types) = node_types {
            if !types.is_empty() {
                let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
                sql.push_str(&format!(" AND node_type IN ({})", placeholders.join(",")));
            }
        }
        sql.push_str(" ORDER BY updated_at DESC LIMIT ?");

        let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let mut q = sqlx::query_as::<_, KgNodeRow>(&sql)
            .bind(&pattern)
            .bind(&pattern);
        if let Some(types) = node_types {
            for t in types {
                q = q.bind(t.as_str());
            }
        }
        q = q.bind(limit as i64);

        let rows: Vec<KgNodeRow> = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(KgNode::try_from).collect()
    }

    /// Removes a node; relationships cascade via FK. Returns whether a
    /// row was actually removed.
    pub async fn delete_node(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query("DELETE FROM graph_nodes WHERE node_type = ? AND entity_id = ?")
            .bind(node_type.as_str())
            .bind(entity_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ------------------------------------------------------------------
    // Relationship CRUD
    // ------------------------------------------------------------------

    /// Upserts one relationship. Returns `true` when created, `false`
    /// when an existing row was updated.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_relationship(
        &self,
        source_type: GraphNodeType,
        source_id: Uuid,
        target_type: GraphNodeType,
        target_id: Uuid,
        relationship_type: GraphRelationshipType,
        weight: f64,
        metadata: serde_json::Value,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now();
        let meta = serde_json::to_string(&metadata).unwrap_or_else(|_| "{}".into());

        let row: KgEdgeRow = sqlx::query_as(
            "INSERT INTO graph_relationships
                 (id, source_node_type, source_entity_id, target_node_type, target_entity_id,
                  relationship_type, weight, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(source_node_type, source_entity_id, target_node_type, target_entity_id,
                         relationship_type)
             DO UPDATE SET weight = excluded.weight, metadata = excluded.metadata,
                           updated_at = excluded.updated_at
             RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(source_type.as_str())
        .bind(source_id)
        .bind(target_type.as_str())
        .bind(target_id)
        .bind(relationship_type.as_str())
        .bind(weight)
        .bind(&meta)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.created_at == row.updated_at)
    }

    /// All relationships touching a node (either direction).
    pub async fn get_edges_for_node(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<Vec<KgEdge>, DatabaseError> {
        let rows: Vec<KgEdgeRow> = sqlx::query_as(
            "SELECT * FROM graph_relationships
             WHERE (source_node_type = ? AND source_entity_id = ?)
                OR (target_node_type = ? AND target_entity_id = ?)",
        )
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(node_type.as_str())
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(KgEdge::try_from).collect()
    }

    /// Direct neighbors of a node (nodes reachable over any relationship
    /// in either direction).
    pub async fn get_neighbors(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        limit: u32,
    ) -> Result<Vec<KgNode>, DatabaseError> {
        let rows: Vec<KgNodeRow> = sqlx::query_as(
            "SELECT n.* FROM graph_relationships r
             JOIN graph_nodes n
               ON (n.node_type = r.target_node_type AND n.entity_id = r.target_entity_id)
             WHERE r.source_node_type = ? AND r.source_entity_id = ?
             UNION
             SELECT n.* FROM graph_relationships r
             JOIN graph_nodes n
               ON (n.node_type = r.source_node_type AND n.entity_id = r.source_entity_id)
             WHERE r.target_node_type = ? AND r.target_entity_id = ?
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(KgNode::try_from).collect()
    }

    // ------------------------------------------------------------------
    // Source extraction (automatic construction)
    // ------------------------------------------------------------------

    /// Every workspace as a graph source. Metadata carries its status.
    pub async fn workspace_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, String, String, String)> =
            sqlx::query_as("SELECT id, name, status, COALESCE(description, '') FROM workspaces")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, status, description)| GraphSource {
                entity_id: id,
                title: name,
                workspace_id: Some(id),
                summary: Some(description).filter(|s| !s.is_empty()),
                metadata: serde_json::json!({ "status": status }),
            })
            .collect())
    }

    /// Every file as a graph source. Metadata carries its artifact type.
    pub async fn file_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid, String, String)> =
            sqlx::query_as("SELECT id, workspace_id, path_or_url, artifact_type FROM files")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, workspace_id, path, artifact_type)| GraphSource {
                entity_id: id,
                title: path
                    .rsplit('/')
                    .next()
                    .filter(|name| !name.is_empty())
                    .unwrap_or(&path)
                    .to_string(),
                workspace_id: Some(workspace_id),
                summary: Some(path),
                metadata: serde_json::json!({ "artifact_type": artifact_type }),
            })
            .collect())
    }

    /// Every planner report as a graph source. The report node is keyed
    /// on the execution id it summarizes (the table's primary key), and
    /// its summary is the report body, truncated.
    pub async fn planner_report_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, String, String)> =
            sqlx::query_as("SELECT execution_id, report, created_at FROM plan_execution_reports")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(execution_id, report, created_at)| {
                let summary: String = report.chars().take(400).collect();
                GraphSource {
                    entity_id: execution_id,
                    title: format!("Planner Report {}", short(&execution_id)),
                    workspace_id: None,
                    summary: Some(summary).filter(|s| !s.is_empty()),
                    metadata: serde_json::json!({
                        "execution_id": execution_id.to_string(),
                        "created_at": created_at,
                    }),
                }
            })
            .collect())
    }

    /// Every plan execution as a graph source. The title is the plan
    /// goal (joined through `copilot_plans`), the workspace comes from
    /// the owning conversation.
    pub async fn execution_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, Option<Uuid>, String, String, String, String)> = sqlx::query_as(
            "SELECT pe.id, c.workspace_id,
                    COALESCE(p.goal, 'Execution ' || substr(lower(hex(pe.id)), 1, 8)),
                    pe.status, pe.started_at, pe.completed_at
             FROM plan_executions pe
             LEFT JOIN copilot_conversations c ON c.id = pe.conversation_id
             LEFT JOIN copilot_plans p ON p.id = pe.plan_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, workspace_id, title, status, started_at, completed_at)| GraphSource {
                    entity_id: id,
                    title,
                    workspace_id,
                    summary: Some(format!("Status: {status}")),
                    metadata: serde_json::json!({
                        "status": status,
                        "started_at": started_at,
                        "completed_at": completed_at,
                    }),
                },
            )
            .collect())
    }

    /// Memory records as graph sources (execution + planner-report
    /// kinds; autonomous sessions have their own node type and are
    /// excluded here).
    pub async fn memory_record_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, Option<Uuid>, String, String, String)> = sqlx::query_as(
            "SELECT id, workspace_id, goal, kind, status
             FROM execution_memory
             WHERE kind != 'autonomous_session'",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, workspace_id, goal, kind, status)| GraphSource {
                entity_id: id,
                title: goal,
                workspace_id,
                summary: Some(format!("{kind} · {status}")),
                metadata: serde_json::json!({ "kind": kind, "status": status }),
            })
            .collect())
    }

    /// Autonomous sessions as graph sources. The session node is keyed
    /// on the session id (the memory row's `source_id`).
    pub async fn autonomous_session_sources(&self) -> Result<Vec<GraphSource>, DatabaseError> {
        let rows: Vec<(Uuid, Option<Uuid>, String, String, String)> = sqlx::query_as(
            "SELECT source_id, workspace_id, goal, status, error
             FROM execution_memory
             WHERE kind = 'autonomous_session'",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(source_id, workspace_id, goal, status, error)| GraphSource {
                    entity_id: source_id,
                    title: format!("Session: {goal}"),
                    workspace_id,
                    summary: Some(error)
                        .filter(|e| !e.is_empty())
                        .or_else(|| Some(format!("Status: {status}"))),
                    metadata: serde_json::json!({ "status": status }),
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------
    // Structural links (edges built during construction)
    // ------------------------------------------------------------------

    /// `(file_id, workspace_id)` pairs for `contains` edges.
    pub async fn file_workspace_links(&self) -> Result<Vec<(Uuid, Uuid)>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT id, workspace_id FROM files")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    /// `(execution_id, workspace_id)` pairs for `runs_in` edges.
    pub async fn execution_workspace_links(&self) -> Result<Vec<(Uuid, Uuid)>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT pe.id, c.workspace_id
             FROM plan_executions pe
             JOIN copilot_conversations c ON c.id = pe.conversation_id
             WHERE c.workspace_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Execution ids that have a planner report (`reports_on` edges —
    /// the report node and the execution node share the id).
    pub async fn planner_report_links(&self) -> Result<Vec<Uuid>, DatabaseError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT execution_id FROM plan_execution_reports")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// `(memory_id, execution_id)` pairs for `derived_from` edges
    /// (memory records learned from an engine execution).
    pub async fn memory_execution_links(&self) -> Result<Vec<(Uuid, Uuid)>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, source_id FROM execution_memory
             WHERE kind = 'execution' AND source_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// `(memory_id, workspace_id)` pairs for `runs_in` edges of memory
    /// records and autonomous sessions (both live in `execution_memory`).
    pub async fn memory_workspace_links(&self) -> Result<Vec<(Uuid, Uuid)>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, workspace_id FROM execution_memory
             WHERE kind != 'autonomous_session' AND workspace_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// `(session_id, workspace_id)` pairs for `runs_in` edges of
    /// autonomous sessions.
    pub async fn session_workspace_links(&self) -> Result<Vec<(Uuid, Uuid)>, DatabaseError> {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT source_id, workspace_id FROM execution_memory
             WHERE kind = 'autonomous_session' AND workspace_id IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------

    pub async fn stats(&self) -> Result<KgStats, DatabaseError> {
        let (node_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM graph_nodes")
            .fetch_one(&self.pool)
            .await?;
        let (edge_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM graph_relationships")
            .fetch_one(&self.pool)
            .await?;

        let nodes_by_type: Vec<(String, i64)> =
            sqlx::query_as("SELECT node_type, COUNT(*) FROM graph_nodes GROUP BY node_type")
                .fetch_all(&self.pool)
                .await?;
        let edges_by_type: Vec<(String, i64)> = sqlx::query_as(
            "SELECT relationship_type, COUNT(*) FROM graph_relationships
             GROUP BY relationship_type",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(KgStats {
            node_count,
            edge_count,
            nodes_by_type: nodes_by_type
                .into_iter()
                .map(|(name, count)| TypeCount { name, count })
                .collect(),
            edges_by_type: edges_by_type
                .into_iter()
                .map(|(name, count)| TypeCount { name, count })
                .collect(),
        })
    }
}

fn short(id: &Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::CreateWorkspaceInput;
    use crate::repositories::WorkspaceRepository;
    use serde_json::json;

    async fn setup() -> (
        KgRepository,
        WorkspaceRepository,
        sqlx::SqlitePool,
        tempfile::TempDir,
    ) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();
        (
            KgRepository::new(pool.clone()),
            WorkspaceRepository::new(pool.clone()),
            pool,
            temp_dir,
        )
    }

    fn source(id: Uuid, title: &str, workspace_id: Option<Uuid>) -> GraphSource {
        GraphSource {
            entity_id: id,
            title: title.into(),
            workspace_id,
            summary: Some("test summary".into()),
            metadata: json!({ "test": true }),
        }
    }

    #[tokio::test]
    async fn upsert_node_is_idempotent_and_reports_created_vs_updated() {
        let (repo, _ws_repo, _pool, _guard) = setup().await;
        let id = Uuid::new_v4();

        let created = repo
            .upsert_node(GraphNodeType::File, &source(id, "first title", None))
            .await
            .unwrap();
        assert!(created, "first insert creates the node");

        let updated = repo
            .upsert_node(GraphNodeType::File, &source(id, "second title", None))
            .await
            .unwrap();
        assert!(!updated, "second upsert updates, not creates");

        let node = repo
            .get_node(GraphNodeType::File, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(node.title, "second title");
        assert!(node.created_at < node.updated_at);
    }

    #[tokio::test]
    async fn list_and_search_nodes_filter_by_type_and_workspace() {
        let (repo, ws_repo, _pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Graph Workspace".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let other = Uuid::new_v4();

        repo.upsert_node(
            GraphNodeType::Workspace,
            &source(ws.id, "Graph Workspace", Some(ws.id)),
        )
        .await
        .unwrap();
        repo.upsert_node(GraphNodeType::File, &source(other, "alpha.rs", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_node(
            GraphNodeType::MemoryRecord,
            &source(other, "beta.goal", Some(ws.id)),
        )
        .await
        .unwrap();

        let files = repo
            .list_nodes(Some(ws.id), Some(&[GraphNodeType::File]), Some(10))
            .await
            .unwrap();
        assert_eq!(files.len(), 1);
        assert!(matches!(files[0].node_type, GraphNodeType::File));

        let hits = repo.search_nodes("alpha", None, 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "alpha.rs");

        let miss = repo.search_nodes("zeta", None, 10).await.unwrap();
        assert!(miss.is_empty());
    }

    #[tokio::test]
    async fn relationships_round_trip_with_direction_agnostic_neighbors() {
        let (repo, ws_repo, _pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let file_id = Uuid::new_v4();
        let exec_id = Uuid::new_v4();

        repo.upsert_node(GraphNodeType::Workspace, &source(ws.id, "WS", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_node(GraphNodeType::File, &source(file_id, "a.rs", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_node(
            GraphNodeType::Execution,
            &source(exec_id, "goal", Some(ws.id)),
        )
        .await
        .unwrap();

        repo.upsert_relationship(
            GraphNodeType::Workspace,
            ws.id,
            GraphNodeType::File,
            file_id,
            GraphRelationshipType::Contains,
            1.0,
            json!({}),
        )
        .await
        .unwrap();
        repo.upsert_relationship(
            GraphNodeType::Execution,
            exec_id,
            GraphNodeType::Workspace,
            ws.id,
            GraphRelationshipType::RunsIn,
            1.0,
            json!({}),
        )
        .await
        .unwrap();

        let file_neighbors = repo
            .get_neighbors(GraphNodeType::File, file_id, 10)
            .await
            .unwrap();
        assert_eq!(file_neighbors.len(), 1);
        assert!(matches!(
            file_neighbors[0].node_type,
            GraphNodeType::Workspace
        ));

        let ws_neighbors = repo
            .get_neighbors(GraphNodeType::Workspace, ws.id, 10)
            .await
            .unwrap();
        assert_eq!(ws_neighbors.len(), 2, "both directions resolve");

        let edges = repo
            .get_edges_for_node(GraphNodeType::Workspace, ws.id)
            .await
            .unwrap();
        assert_eq!(edges.len(), 2);
    }

    #[tokio::test]
    async fn delete_node_cascades_relationships() {
        let (repo, ws_repo, _pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let file_id = Uuid::new_v4();

        repo.upsert_node(GraphNodeType::Workspace, &source(ws.id, "WS", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_node(GraphNodeType::File, &source(file_id, "a.rs", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_relationship(
            GraphNodeType::Workspace,
            ws.id,
            GraphNodeType::File,
            file_id,
            GraphRelationshipType::Contains,
            1.0,
            json!({}),
        )
        .await
        .unwrap();

        assert!(repo
            .delete_node(GraphNodeType::File, file_id)
            .await
            .unwrap());
        let edges = repo
            .get_edges_for_node(GraphNodeType::Workspace, ws.id)
            .await
            .unwrap();
        assert!(edges.is_empty(), "relationship cascaded with the node");
    }

    #[tokio::test]
    async fn construction_sources_cover_all_six_aggregates() {
        let (repo, ws_repo, pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "Source WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        // files
        sqlx::query(
            "INSERT INTO files (id, workspace_id, artifact_type, path_or_url, created_at, updated_at)
             VALUES (?, ?, 'file', ?, datetime('now'), datetime('now'))",
        )
        .bind(Uuid::new_v4())
        .bind(ws.id)
        .bind("/tmp/source.rs")
        .execute(&pool)
        .await
        .unwrap();

        // plan + conversation + execution
        sqlx::query(
            "INSERT INTO copilot_conversations (id, workspace_id, title, created_at, updated_at)
             VALUES (?, ?, 'conv', datetime('now'), datetime('now'))",
        )
        .bind(Uuid::new_v4())
        .bind(ws.id)
        .execute(&pool)
        .await
        .unwrap();

        // planner report (also creates an implicit execution node source? no — plan_execution_reports references plan_executions via FK; insert execution first)
        let exec_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO plan_executions
                 (id, plan_id, status, current_step, total_steps, created_at, updated_at)
             VALUES (?, ?, 'completed', 0, 2, datetime('now'), datetime('now'))",
        )
        .bind(exec_id)
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO plan_execution_reports (execution_id, report) VALUES (?, 'all good')",
        )
        .bind(exec_id)
        .execute(&pool)
        .await
        .unwrap();

        // memory record (kind=execution) + autonomous session (kind=autonomous_session)
        let memory_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO execution_memory
                 (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
             VALUES (?, 'execution', ?, ?, 'learn goal', 'success', ?, ?)",
        )
        .bind(memory_id)
        .bind(exec_id)
        .bind(ws.id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO execution_memory
                 (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
             VALUES (?, 'autonomous_session', ?, ?, 'session goal', 'success', ?, ?)",
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(ws.id)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(repo.workspace_sources().await.unwrap().len(), 1);
        assert_eq!(repo.file_sources().await.unwrap().len(), 1);
        assert_eq!(repo.planner_report_sources().await.unwrap().len(), 1);
        assert_eq!(repo.execution_sources().await.unwrap().len(), 1);
        assert_eq!(repo.memory_record_sources().await.unwrap().len(), 1);
        assert_eq!(repo.autonomous_session_sources().await.unwrap().len(), 1);

        assert_eq!(repo.planner_report_links().await.unwrap(), vec![exec_id]);
        assert_eq!(
            repo.memory_execution_links().await.unwrap(),
            vec![(memory_id, exec_id)]
        );
    }

    #[tokio::test]
    async fn stats_roll_up_counts_by_type() {
        let (repo, ws_repo, _pool, _guard) = setup().await;
        let ws = ws_repo
            .create(CreateWorkspaceInput {
                name: "WS".into(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();
        let file_id = Uuid::new_v4();

        repo.upsert_node(GraphNodeType::Workspace, &source(ws.id, "WS", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_node(GraphNodeType::File, &source(file_id, "a.rs", Some(ws.id)))
            .await
            .unwrap();
        repo.upsert_relationship(
            GraphNodeType::Workspace,
            ws.id,
            GraphNodeType::File,
            file_id,
            GraphRelationshipType::Contains,
            1.0,
            json!({}),
        )
        .await
        .unwrap();

        let stats = repo.stats().await.unwrap();
        assert_eq!(stats.node_count, 2);
        assert_eq!(stats.edge_count, 1);
        assert_eq!(stats.nodes_by_type.len(), 2);
        assert_eq!(stats.edges_by_type[0].name, "contains");
        assert_eq!(stats.edges_by_type[0].count, 1);
    }
}
