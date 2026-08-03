//! Context Intelligence repository (RC-8 M3).
//!
//! Owns every SQL statement behind the M3 surfaces: persisted
//! cross-workspace relationships (`context_intel_workspace_relations`),
//! graph context snapshots (`context_intel_snapshots`), and goal
//! clusters (`context_intel_clusters`). The graph itself is read
//! through [`KgLiveRepository`](crate::repositories::KgLiveRepository)
//! (`all_nodes`/`all_edges`) and workspace metadata through
//! [`WorkspaceRepository`](crate::repositories::WorkspaceRepository) —
//! no SQL is duplicated here. All scoring, similarity and clustering
//! policy lives in [`ContextIntelService`](crate::services::ContextIntelService).
//!
//! `workspace_id` columns are declared TEXT but — like every UUID column
//! in this codebase — store sqlx's BLOB encoding (raw 16 bytes), so ids
//! are bound and read as `Uuid` directly, never as strings.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg_context::{GoalCluster, SummaryPoint};

/// One persisted cross-workspace relationship (both directions of the
/// unordered pair are returned by [`ContextIntelRepository::list_workspace_similarity`]).
pub struct WorkspaceSimilarityRow {
    pub target_workspace_id: Uuid,
    pub similarity: f64,
    pub confidence: f64,
    /// JSON array of `SignalEvidence`.
    pub signals: String,
    pub last_updated: DateTime<Utc>,
}

/// One persisted graph context snapshot.
pub struct SnapshotRow {
    pub id: i64,
    pub workspace_id: Uuid,
    pub snapshot_type: String,
    pub node_count: i64,
    pub edge_count: i64,
    pub confidence: f64,
    /// JSON array of `SummaryPoint`.
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

/// One persisted goal cluster.
pub struct ClusterRow {
    pub id: i64,
    pub workspace_id: Option<Uuid>,
    pub name: String,
    pub member_count: i64,
    /// JSON array of `ClusterMember`.
    pub members: String,
    /// JSON array of centroid terms.
    pub centroid: String,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

/// Repository for the RC-8 M3 context intelligence surfaces.
#[derive(Debug, Clone)]
pub struct ContextIntelRepository {
    pool: SqlitePool,
}

impl ContextIntelRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Cross-workspace relationships
    // ------------------------------------------------------------------

    /// Upserts one cross-workspace relationship. The service stores each
    /// unordered pair once under a canonical direction (lexicographically
    /// smaller workspace id first), so the lookup below scans both
    /// directions.
    pub async fn upsert_workspace_similarity(
        &self,
        source_workspace_id: Uuid,
        target_workspace_id: Uuid,
        similarity: f64,
        confidence: f64,
        signals: &[serde_json::Value],
    ) -> Result<(), DatabaseError> {
        let signals = serde_json::to_string(signals)?;
        sqlx::query(
            "INSERT INTO context_intel_workspace_relations
                 (source_workspace_id, target_workspace_id, similarity, confidence, signals)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(source_workspace_id, target_workspace_id)
             DO UPDATE SET similarity = excluded.similarity,
                            confidence = excluded.confidence,
                            signals = excluded.signals,
                            last_updated = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        )
        .bind(source_workspace_id)
        .bind(target_workspace_id)
        .bind(similarity)
        .bind(confidence)
        .bind(&signals)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Persisted relationships touching `workspace_id` (either side),
    /// strongest first, optionally above a similarity floor.
    pub async fn list_workspace_similarity(
        &self,
        workspace_id: Uuid,
        min_similarity: Option<f64>,
        limit: Option<usize>,
    ) -> Result<Vec<WorkspaceSimilarityRow>, DatabaseError> {
        let mut sql = String::from(
            "SELECT source_workspace_id, target_workspace_id, similarity, confidence, signals,
                    last_updated
             FROM context_intel_workspace_relations
             WHERE (source_workspace_id = ? OR target_workspace_id = ?)",
        );
        if min_similarity.is_some() {
            sql.push_str(" AND similarity >= ?");
        }
        sql.push_str(" ORDER BY similarity DESC");
        if limit.is_some() {
            sql.push_str(" LIMIT ?");
        }

        let mut query = sqlx::query_as::<_, (Uuid, Uuid, f64, f64, String, String)>(&sql)
            .bind(workspace_id)
            .bind(workspace_id);
        if let Some(floor) = min_similarity {
            query = query.bind(floor);
        }
        if let Some(max) = limit {
            query = query.bind(max as i64);
        }

        let rows = query.fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for (source, target, similarity, confidence, signals, last_updated) in rows {
            let other = if source == workspace_id {
                target
            } else {
                source
            };
            out.push(WorkspaceSimilarityRow {
                target_workspace_id: other,
                similarity,
                confidence,
                signals,
                last_updated: parse_rfc3339(&last_updated)?,
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Graph context snapshots
    // ------------------------------------------------------------------

    /// Stores one graph context snapshot, returning its row id.
    #[allow(clippy::too_many_arguments)] // the snapshot row is the point
    pub async fn snapshot_insert(
        &self,
        workspace_id: Uuid,
        snapshot_type: &str,
        node_count: i64,
        edge_count: i64,
        confidence: f64,
        summary: &[SummaryPoint],
        payload: &serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let summary = serde_json::to_string(summary)?;
        let payload = serde_json::to_string(payload)?;
        // Explicit RFC3339 timestamp: SQLite's `datetime('now')` default
        // is not RFC3339, and already-migrated databases keep the old
        // column default regardless of the migration text.
        let created_at = Utc::now().to_rfc3339();
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO context_intel_snapshots
                 (workspace_id, snapshot_type, node_count, edge_count, confidence, summary,
                  payload, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
        )
        .bind(workspace_id)
        .bind(snapshot_type)
        .bind(node_count)
        .bind(edge_count)
        .bind(confidence)
        .bind(&summary)
        .bind(&payload)
        .bind(&created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// The most recent snapshots for a workspace, newest first.
    pub async fn snapshot_list(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<SnapshotRow>, DatabaseError> {
        let mut sql = String::from(
            "SELECT id, workspace_id, snapshot_type, node_count, edge_count, confidence, summary,
                    created_at
             FROM context_intel_snapshots
             WHERE workspace_id = ?
             ORDER BY created_at DESC",
        );
        if limit.is_some() {
            sql.push_str(" LIMIT ?");
        }
        let mut query =
            sqlx::query_as::<_, (i64, Uuid, String, i64, i64, f64, String, String)>(&sql)
                .bind(workspace_id);
        if let Some(max) = limit {
            query = query.bind(max as i64);
        }
        let rows = query.fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for (
            id,
            workspace_id,
            snapshot_type,
            node_count,
            edge_count,
            confidence,
            summary,
            created_at,
        ) in rows
        {
            out.push(SnapshotRow {
                id,
                workspace_id,
                snapshot_type,
                node_count,
                edge_count,
                confidence,
                summary,
                created_at: parse_rfc3339(&created_at)?,
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Goal clusters
    // ------------------------------------------------------------------

    /// Replaces every persisted cluster for one scope (`None` = whole
    /// graph) with the freshly computed set. Clusters are derived data,
    /// so replace-on-write keeps the table consistent with the graph.
    pub async fn clusters_replace(
        &self,
        workspace_id: Option<Uuid>,
        clusters: &[GoalCluster],
    ) -> Result<(), DatabaseError> {
        if workspace_id.is_some() {
            sqlx::query("DELETE FROM context_intel_clusters WHERE workspace_id = ?")
                .bind(workspace_id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("DELETE FROM context_intel_clusters")
                .execute(&self.pool)
                .await?;
        }
        for cluster in clusters {
            let members = serde_json::to_string(&cluster.members)?;
            let centroid = serde_json::to_string(&cluster.centroid_terms)?;
            // Explicit RFC3339 timestamp (see `snapshot_insert`).
            let created_at = Utc::now().to_rfc3339();
            sqlx::query(
                "INSERT INTO context_intel_clusters
                     (workspace_id, name, member_count, members, centroid, confidence, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(workspace_id)
            .bind(&cluster.name)
            .bind(cluster.member_count as i64)
            .bind(&members)
            .bind(&centroid)
            .bind(cluster.confidence)
            .bind(&created_at)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Persisted clusters for one scope (`None` = whole graph).
    pub async fn clusters_list(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<ClusterRow>, DatabaseError> {
        let mut sql = String::from(
            "SELECT id, workspace_id, name, member_count, members, centroid, confidence
             FROM context_intel_clusters",
        );
        if workspace_id.is_some() {
            sql.push_str(" WHERE workspace_id = ?");
        }
        sql.push_str(" ORDER BY member_count DESC, id ASC");

        let mut query =
            sqlx::query_as::<_, (i64, Option<Uuid>, String, i64, String, String, f64)>(&sql);
        if let Some(ws_id) = workspace_id {
            query = query.bind(ws_id);
        }
        let rows = query.fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for (id, workspace_id, name, member_count, members, centroid, confidence) in rows {
            out.push(ClusterRow {
                id,
                workspace_id,
                name,
                member_count,
                members,
                centroid,
                confidence,
                created_at: Utc::now(),
            });
        }
        Ok(out)
    }
}

fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>, DatabaseError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| DatabaseError::IoError(e.to_string()))
}

#[cfg(test)]
#[path = "context_intel_repository_tests.rs"]
mod tests;
