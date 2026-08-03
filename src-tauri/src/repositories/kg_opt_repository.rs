//! Knowledge Graph Optimization repository (RC-8 M4).
//!
//! Owns every SQL statement behind the optimization/scale surfaces that
//! the M1/M2 repositories do not: paginated node/edge/neighbor loading,
//! the four integrity scans (orphan edges, dangling workspaces,
//! malformed nodes, invalid confidence) plus their repair helpers, and
//! the persisted operational ledger (integrity issues, maintenance runs,
//! query metrics, benchmarks). All SQL stays here; ranking, timing,
//! parallel traversal, and repair policy live in
//! [`crate::services::KgOptService`] and
//! [`crate::services::GraphHealthService`].

use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::{GraphNodeType, KgEdge, KgEdgeRow, KgNode, KgNodeRow};
use crate::models::kg_opt::{
    EdgePage, GraphBenchmarkResult, GraphIntegrityIssue, IssueSeverity, IssueType, MaintenanceRun,
    NeighborPage, NeighborRow, NodePage, QueryMetric,
};

/// Raw `graph_integrity_issues` row.
type IssueRow = (
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);
/// Raw `graph_maintenance_runs` row.
type MaintenanceRow = (
    i64,
    String,
    String,
    i64,
    i64,
    i64,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);
/// Raw `graph_query_metrics` row.
type MetricRow = (
    i64,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
    i64,
    DateTime<Utc>,
);
/// Raw `graph_benchmarks` row.
type BenchmarkRow = (String, String, String, i64, i64, i64, DateTime<Utc>);

/// Repository for the RC-8 M4 knowledge graph optimization surfaces.
#[derive(Debug, Clone)]
pub struct KgOptRepository {
    pool: SqlitePool,
}

impl KgOptRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Pagination
    // ------------------------------------------------------------------

    /// Total number of nodes matching the filters (progressive loading
    /// needs the count to render a virtualized list with a total).
    pub async fn nodes_page_count(
        &self,
        node_types: Option<&[GraphNodeType]>,
        workspace_id: Option<Uuid>,
    ) -> Result<u64, DatabaseError> {
        let mut sql = String::from("SELECT COUNT(*) FROM graph_nodes WHERE 1=1");
        if workspace_id.is_some() {
            sql.push_str(" AND workspace_id = ?");
        }
        if let Some(types) = node_types {
            if !types.is_empty() {
                let placeholders: Vec<&str> = types.iter().map(|_| "?").collect();
                sql.push_str(&format!(" AND node_type IN ({})", placeholders.join(",")));
            }
        }
        let mut query = sqlx::query_as::<_, (i64,)>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        if let Some(types) = node_types {
            for t in types {
                query = query.bind(t.as_str());
            }
        }
        let (count,): (i64,) = query.fetch_one(&self.pool).await?;
        Ok(count.max(0) as u64)
    }

    /// One page of graph nodes, newest-first, with the total count.
    pub async fn nodes_page(
        &self,
        node_types: Option<&[GraphNodeType]>,
        workspace_id: Option<Uuid>,
        offset: u64,
        limit: u32,
    ) -> Result<NodePage, DatabaseError> {
        let total = self.nodes_page_count(node_types, workspace_id).await?;

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
        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, KgNodeRow>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        if let Some(types) = node_types {
            for t in types {
                query = query.bind(t.as_str());
            }
        }
        query = query.bind(limit as i64).bind(offset as i64);

        let rows: Vec<KgNodeRow> = query.fetch_all(&self.pool).await?;
        let nodes = rows
            .into_iter()
            .map(KgNode::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(NodePage {
            has_more: (offset + nodes.len() as u64) < total,
            total,
            offset,
            limit,
            nodes,
        })
    }

    /// Total number of graph edges.
    pub async fn edges_page_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM graph_relationships")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    /// One page of graph edges, newest-first, with the total count.
    pub async fn edges_page(&self, offset: u64, limit: u32) -> Result<EdgePage, DatabaseError> {
        let total = self.edges_page_count().await?;
        let rows: Vec<KgEdgeRow> = sqlx::query_as(
            "SELECT * FROM graph_relationships
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;
        let edges = rows
            .into_iter()
            .map(KgEdge::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(EdgePage {
            has_more: (offset + edges.len() as u64) < total,
            total,
            offset,
            limit,
            edges,
        })
    }

    /// One page of a node's direct neighbors: the connecting edge plus
    /// the neighbor node, newest edge first. Edges whose neighbor is
    /// missing (orphans) are skipped by the fetch.
    pub async fn neighbors_page(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        offset: u64,
        limit: u32,
    ) -> Result<NeighborPage, DatabaseError> {
        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM graph_relationships
             WHERE (source_node_type = ? AND source_entity_id = ?)
                OR (target_node_type = ? AND target_entity_id = ?)",
        )
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(node_type.as_str())
        .bind(entity_id)
        .fetch_one(&self.pool)
        .await?;

        let edges: Vec<KgEdge> = sqlx::query_as::<_, KgEdgeRow>(
            "SELECT * FROM graph_relationships
             WHERE (source_node_type = ? AND source_entity_id = ?)
                OR (target_node_type = ? AND target_entity_id = ?)
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(node_type.as_str())
        .bind(entity_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(KgEdge::try_from)
        .collect::<Result<Vec<_>, _>>()?;

        let neighbors = self.resolve_neighbors(node_type, entity_id, &edges).await?;
        let rows: Vec<NeighborRow> = neighbors
            .into_iter()
            .zip(&edges)
            .filter_map(|(neighbor, edge)| {
                neighbor.map(|neighbor| NeighborRow {
                    edge: edge.clone(),
                    neighbor,
                })
            })
            .collect();

        let total = total.0.max(0) as u64;
        Ok(NeighborPage {
            has_more: (offset + rows.len() as u64) < total,
            total,
            offset,
            limit,
            neighbors: rows,
        })
    }

    /// Resolves the neighbor node of each edge touching `entity_id`
    /// (the endpoint that is not the focus node) in one batched query.
    async fn resolve_neighbors(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
        edges: &[KgEdge],
    ) -> Result<Vec<Option<KgNode>>, DatabaseError> {
        if edges.is_empty() {
            return Ok(Vec::new());
        }
        let keys: Vec<(String, Uuid)> = edges
            .iter()
            .map(|edge| {
                if edge.source_entity_id == entity_id && edge.source_node_type == node_type {
                    (
                        edge.target_node_type.as_str().to_string(),
                        edge.target_entity_id,
                    )
                } else {
                    (
                        edge.source_node_type.as_str().to_string(),
                        edge.source_entity_id,
                    )
                }
            })
            .collect();

        let mut sql = String::from("SELECT * FROM graph_nodes WHERE 1=0");
        for _ in &keys {
            sql.push_str(" OR (node_type = ? AND entity_id = ?)");
        }
        let mut query = sqlx::query_as::<_, KgNodeRow>(&sql);
        for (node_type, entity_id) in &keys {
            query = query.bind(node_type).bind(entity_id);
        }
        let rows: Vec<KgNodeRow> = query.fetch_all(&self.pool).await?;
        let by_key: std::collections::HashMap<(String, Uuid), KgNode> = rows
            .into_iter()
            .filter_map(|row| KgNode::try_from(row).ok())
            .map(|node| ((node.node_type.as_str().to_string(), node.entity_id), node))
            .collect();

        Ok(keys.iter().map(|key| by_key.get(key).cloned()).collect())
    }

    // ------------------------------------------------------------------
    // Integrity scans
    // ------------------------------------------------------------------

    /// Edge ids whose source or target node is missing from the
    /// registry (FKs cannot express cross-row presence, so this is
    /// detected by a left join).
    pub async fn orphan_edge_ids(&self) -> Result<Vec<Uuid>, DatabaseError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT r.id FROM graph_relationships r
             LEFT JOIN graph_nodes s
               ON s.node_type = r.source_node_type AND s.entity_id = r.source_entity_id
             LEFT JOIN graph_nodes t
               ON t.node_type = r.target_node_type AND t.entity_id = r.target_entity_id
             WHERE s.entity_id IS NULL OR t.entity_id IS NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Workspace-scoped nodes whose `workspace_id` no longer exists
    /// (workspace rows deleted without a cascade, e.g. pre-migration).
    pub async fn dangling_workspace_nodes(&self) -> Result<Vec<KgNode>, DatabaseError> {
        let rows: Vec<KgNodeRow> = sqlx::query_as(
            "SELECT n.* FROM graph_nodes n
             LEFT JOIN workspaces w ON w.id = n.workspace_id
             WHERE n.workspace_id IS NOT NULL AND w.id IS NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(KgNode::try_from).collect()
    }

    /// Nodes with an empty title/summary or an unknown node type,
    /// returned raw: `(node_type, entity_id, title, summary)`. Unknown
    /// node types cannot be decoded into `KgNode`, so repair needs the
    /// raw type string to delete them.
    pub async fn malformed_nodes(
        &self,
    ) -> Result<Vec<(String, Uuid, String, Option<String>)>, DatabaseError> {
        sqlx::query_as::<_, (String, Uuid, String, Option<String>)>(
            "SELECT node_type, entity_id, title, summary FROM graph_nodes
             WHERE TRIM(COALESCE(title, '')) = ''
                OR TRIM(COALESCE(summary, '')) = ''
                OR node_type NOT IN ('workspace', 'file', 'planner_report',
                                     'execution', 'memory_record', 'autonomous_session')",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(DatabaseError::from)
    }

    /// Edges whose `confidence` or `weight` fell outside `[0, 1]`.
    pub async fn invalid_confidence_edges(&self) -> Result<Vec<KgEdge>, DatabaseError> {
        let rows: Vec<KgEdgeRow> = sqlx::query_as(
            "SELECT * FROM graph_relationships
             WHERE confidence NOT BETWEEN 0.0 AND 1.0
                OR weight NOT BETWEEN 0.0 AND 1.0",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(KgEdge::try_from).collect()
    }

    // ------------------------------------------------------------------
    // Repair helpers
    // ------------------------------------------------------------------

    /// Deletes the given edge ids. Returns rows removed.
    pub async fn delete_edges(&self, ids: &[Uuid]) -> Result<u64, DatabaseError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "DELETE FROM graph_relationships WHERE id IN ({})",
            placeholders.join(",")
        );
        let mut query = sqlx::query(&sql);
        for id in ids {
            query = query.bind(id);
        }
        let result = query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Restores an empty title/summary on one node. Returns whether a
    /// row was actually updated.
    pub async fn fix_malformed_node(
        &self,
        node_type: GraphNodeType,
        entity_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE graph_nodes
             SET title = CASE WHEN TRIM(COALESCE(title, '')) = '' THEN '(untitled)' ELSE title END,
                 summary = CASE WHEN TRIM(COALESCE(summary, '')) = '' THEN '' ELSE summary END,
                 updated_at = ?
             WHERE node_type = ? AND entity_id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(node_type.as_str())
        .bind(entity_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Clamps an edge's `confidence`/`weight` back into `[0, 1]`.
    /// Returns whether a row was updated.
    pub async fn clamp_edge_values(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE graph_relationships
             SET confidence = MIN(MAX(confidence, 0.0), 1.0),
                 weight = MIN(MAX(weight, 0.0), 1.0),
                 updated_at = ?
             WHERE id = ?",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Deletes a node row (unknown-type malformed nodes cannot be
    /// decoded, so repair drops them).
    pub async fn delete_node(
        &self,
        node_type: &str,
        entity_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query("DELETE FROM graph_nodes WHERE node_type = ? AND entity_id = ?")
            .bind(node_type)
            .bind(entity_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ------------------------------------------------------------------
    // Issue persistence
    // ------------------------------------------------------------------

    /// Persists one integrity finding. Returns its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_issue(
        &self,
        issue_type: IssueType,
        severity: IssueSeverity,
        node_type: Option<&str>,
        entity_id: Option<Uuid>,
        detail: String,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO graph_integrity_issues
                 (issue_type, severity, node_type, entity_id, detail, status)
             VALUES (?, ?, ?, ?, ?, 'open')
             RETURNING id",
        )
        .bind(issue_type.as_str())
        .bind(severity.as_str())
        .bind(node_type)
        .bind(entity_id.map(|id| id.to_string()))
        .bind(detail)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Open findings, newest first (the integrity panel's live list).
    pub async fn open_issues(&self, limit: u32) -> Result<Vec<GraphIntegrityIssue>, DatabaseError> {
        let rows: Vec<IssueRow> = sqlx::query_as(
            "SELECT id, issue_type, severity, node_type, entity_id, detail, status, created_at, resolved_at
             FROM graph_integrity_issues
             WHERE status = 'open'
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(decode_issue).collect()
    }

    /// Recent findings of any status (diagnostics panel history).
    pub async fn recent_issues(
        &self,
        limit: u32,
    ) -> Result<Vec<GraphIntegrityIssue>, DatabaseError> {
        let rows: Vec<IssueRow> = sqlx::query_as(
            "SELECT id, issue_type, severity, node_type, entity_id, detail, status, created_at, resolved_at
             FROM graph_integrity_issues
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(decode_issue).collect()
    }

    /// Marks the given issue ids resolved. Returns rows updated.
    pub async fn mark_issues_resolved(&self, ids: &[i64]) -> Result<u64, DatabaseError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE graph_integrity_issues
             SET status = 'resolved', resolved_at = ?
             WHERE id IN ({}) AND status = 'open'",
            placeholders.join(",")
        );
        let mut query = sqlx::query(&sql).bind(Utc::now().to_rfc3339());
        for id in ids {
            query = query.bind(id);
        }
        let result = query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Resolves open issues of one type whose `entity_id` is in the
    /// given list (the repair pass's bookkeeping). Returns rows updated.
    pub async fn resolve_issues(
        &self,
        issue_type: IssueType,
        entity_ids: &[Uuid],
    ) -> Result<u64, DatabaseError> {
        if entity_ids.is_empty() {
            return Ok(0);
        }
        let placeholders: Vec<&str> = entity_ids.iter().map(|_| "?").collect();
        let sql = format!(
            "UPDATE graph_integrity_issues
             SET status = 'resolved', resolved_at = ?
             WHERE status = 'open' AND issue_type = ? AND entity_id IN ({})",
            placeholders.join(",")
        );
        let mut query = sqlx::query(&sql)
            .bind(Utc::now().to_rfc3339())
            .bind(issue_type.as_str());
        for id in entity_ids {
            query = query.bind(id.to_string());
        }
        let result = query.execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Number of open findings.
    pub async fn open_issue_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM graph_integrity_issues WHERE status = 'open'")
                .fetch_one(&self.pool)
                .await?;
        Ok(count.max(0) as u64)
    }

    /// Per-type counts of open findings (integrity panel histogram).
    pub async fn open_issue_type_counts(
        &self,
    ) -> Result<Vec<crate::models::kg::TypeCount>, DatabaseError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT issue_type, COUNT(*) FROM graph_integrity_issues
             WHERE status = 'open' GROUP BY issue_type",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, count)| crate::models::kg::TypeCount {
                name,
                count: count.max(0),
            })
            .collect())
    }

    /// Number of duplicate `(node_type, entity_id)` node rows — the
    /// consistency check's uniqueness probe.
    pub async fn duplicate_node_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM (
                 SELECT node_type, entity_id FROM graph_nodes
                 GROUP BY node_type, entity_id HAVING COUNT(*) > 1
             )",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.max(0) as u64)
    }

    // ------------------------------------------------------------------
    // Maintenance history
    // ------------------------------------------------------------------

    /// Records one maintenance run. Returns its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_maintenance_run(
        &self,
        run_type: &str,
        status: &str,
        issues_found: u64,
        issues_resolved: u64,
        duration_ms: u64,
        summary: serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO graph_maintenance_runs
                 (run_type, status, issues_found, issues_resolved, duration_ms, summary,
                  started_at, finished_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(run_type)
        .bind(status)
        .bind(issues_found as i64)
        .bind(issues_resolved as i64)
        .bind(duration_ms as i64)
        .bind(serde_json::to_string(&summary).unwrap_or_else(|_| "{}".into()))
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Most recent maintenance runs, newest first.
    pub async fn recent_maintenance_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<MaintenanceRun>, DatabaseError> {
        let rows: Vec<MaintenanceRow> = sqlx::query_as(
            "SELECT id, run_type, status, issues_found, issues_resolved, duration_ms, summary,
                    started_at, finished_at
             FROM graph_maintenance_runs
             ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, run_type, status, found, resolved, duration, summary, started, finished)| {
                    MaintenanceRun {
                        id,
                        run_type,
                        status,
                        issues_found: found.max(0) as u64,
                        issues_resolved: resolved.max(0) as u64,
                        duration_ms: duration.max(0) as u64,
                        summary: serde_json::from_str(&summary)
                            .unwrap_or(serde_json::Value::Object(Default::default())),
                        started_at: started,
                        finished_at: finished,
                    }
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------
    // Benchmark persistence
    // ------------------------------------------------------------------

    /// Records one micro-benchmark result. Returns its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_benchmark(
        &self,
        suite_name: &str,
        benchmark_name: &str,
        operation: &str,
        node_count: u64,
        edge_count: u64,
        duration_ms: u64,
        payload: serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO graph_benchmarks
                 (suite_name, benchmark_name, operation, node_count, edge_count, duration_ms, payload)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(suite_name)
        .bind(benchmark_name)
        .bind(operation)
        .bind(node_count as i64)
        .bind(edge_count as i64)
        .bind(duration_ms as i64)
        .bind(serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into()))
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Most recent benchmark results, newest first.
    pub async fn recent_benchmarks(
        &self,
        limit: u32,
    ) -> Result<Vec<GraphBenchmarkResult>, DatabaseError> {
        let rows: Vec<BenchmarkRow> = sqlx::query_as(
            "SELECT suite_name, benchmark_name, operation, node_count, edge_count, duration_ms, created_at
             FROM graph_benchmarks
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(suite, name, operation, nodes, edges, duration, created)| GraphBenchmarkResult {
                    name,
                    operation,
                    node_count: nodes.max(0) as u64,
                    edge_count: edges.max(0) as u64,
                    duration_ms: duration.max(0) as u64,
                    throughput_per_sec: None,
                    suite_name: suite,
                    created_at: created,
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------
    // Query metrics persistence
    // ------------------------------------------------------------------

    /// Records one operation metric. Returns its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_query_metric(
        &self,
        operation: &str,
        scope: Option<&str>,
        query: Option<&str>,
        duration_ms: u64,
        rows_returned: u64,
        hit_cache: bool,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO graph_query_metrics
                 (operation, scope, query, duration_ms, rows_returned, hit_cache)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(operation)
        .bind(scope)
        .bind(query)
        .bind(duration_ms as i64)
        .bind(rows_returned as i64)
        .bind(hit_cache as i64)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Most recent operation metrics, newest first.
    pub async fn recent_query_metrics(
        &self,
        limit: u32,
    ) -> Result<Vec<QueryMetric>, DatabaseError> {
        let rows: Vec<MetricRow> = sqlx::query_as(
            "SELECT id, operation, scope, query, duration_ms, rows_returned, hit_cache, occurred_at
             FROM graph_query_metrics
             ORDER BY occurred_at DESC, id DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, operation, scope, query, duration, rows, cache, occurred)| QueryMetric {
                    id,
                    operation,
                    scope,
                    query,
                    duration_ms: duration.max(0),
                    rows_returned: rows.max(0),
                    hit_cache: cache > 0,
                    occurred_at: occurred,
                },
            )
            .collect())
    }
}

/// Decodes one raw `graph_integrity_issues` row into the public model.
/// Unknown stored enum values degrade to `None`/defaults rather than
/// failing the whole read.
fn decode_issue(row: IssueRow) -> Result<GraphIntegrityIssue, DatabaseError> {
    let (id, issue_type, severity, node_type, entity_id, detail, status, created_at, resolved_at) =
        row;
    let entity_id = entity_id.and_then(|raw| Uuid::parse_str(&raw).ok());
    Ok(GraphIntegrityIssue {
        id,
        issue_type: IssueType::from_stored(&issue_type).unwrap_or(IssueType::MalformedNode),
        severity: match severity.as_str() {
            "info" => IssueSeverity::Info,
            "critical" => IssueSeverity::Critical,
            _ => IssueSeverity::Warning,
        },
        node_type: node_type
            .as_deref()
            .and_then(|raw| crate::models::kg::GraphNodeType::from_str(raw).ok()),
        entity_id,
        detail,
        status,
        created_at,
        resolved_at,
    })
}

#[cfg(test)]
#[path = "kg_opt_repository_tests.rs"]
mod tests;
