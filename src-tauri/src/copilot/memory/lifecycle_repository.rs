//! Memory Lifecycle Repository — all SQL for the RC-6 M4 lifecycle
//! system: retention transitions, versioning lookups, lineage edges,
//! compression archive, snapshots, storage statistics, and orphaned
//! vector cleanup.
//!
//! Owns the SQL; the pure rules live in `memory/lifecycle/` and the
//! orchestration lives in the `MemoryEngine` facade.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::copilot::memory::lifecycle::LineageEdge;
use crate::copilot::memory::models::{goal_fingerprint, RetentionPolicy};
use crate::errors::DatabaseError;

/// Repository for memory lifecycle persistence.
#[derive(Clone)]
pub struct LifecycleRepository {
    pool: SqlitePool,
}

/// A stored snapshot row (before record counts are computed).
pub struct SnapshotRow {
    pub id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub data: String,
}

impl LifecycleRepository {
    /// Creates a new lifecycle repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// The underlying connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ------------------------------------------------------------------
    // Retention
    // ------------------------------------------------------------------

    /// Applies a retention transition to a record, stamping the
    /// relevant timestamps and clearing the others (a record can only
    /// be in one lifecycle state).
    #[allow(clippy::too_many_arguments)] // one call site; the transition is the point
    pub async fn update_retention(
        &self,
        id: Uuid,
        retention: &RetentionPolicy,
        retention_until: Option<DateTime<Utc>>,
        archived_at: Option<DateTime<Utc>>,
        expired_at: Option<DateTime<Utc>>,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE execution_memory
            SET retention = ?, retention_until = ?, archived_at = ?, expired_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(retention.to_string())
        .bind(retention_until.map(|t| t.to_rfc3339()))
        .bind(archived_at.map(|t| t.to_rfc3339()))
        .bind(expired_at.map(|t| t.to_rfc3339()))
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks a record `Expired` (cleanup worker, after its retention
    /// deadline passed).
    pub async fn mark_expired(&self, id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE execution_memory
            SET retention = 'expired', expired_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Ids of temporary records past their retention deadline.
    pub async fn list_due_temporary(&self, limit: usize) -> Result<Vec<Uuid>, DatabaseError> {
        self.list_ids(
            "WHERE retention = 'temporary' AND retention_until IS NOT NULL AND retention_until <= ?",
            limit,
            Some(&Utc::now().to_rfc3339()),
        )
        .await
    }

    /// Ids of records marked `Expired` (deleted by the cleanup pass).
    pub async fn list_expired(&self, limit: usize) -> Result<Vec<Uuid>, DatabaseError> {
        self.list_ids("WHERE retention = 'expired'", limit, None)
            .await
    }

    /// Counts records under a retention policy.
    pub async fn count_by_retention(&self, policy: &RetentionPolicy) -> Result<u64, DatabaseError> {
        type Row = (i64,);
        let row: Option<Row> =
            sqlx::query_as("SELECT COUNT(*) FROM execution_memory WHERE retention = ?")
                .bind(policy.to_string())
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|(count,)| count.max(0) as u64).unwrap_or(0))
    }

    async fn list_ids(
        &self,
        predicate: &str,
        limit: usize,
        bind: Option<&str>,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        let sql = format!("SELECT id FROM execution_memory {predicate} LIMIT ?");
        let mut query = sqlx::query(&sql);
        if let Some(value) = bind {
            query = query.bind(value);
        }
        query = query.bind(limit as i64);
        let rows = query.fetch_all(&self.pool).await?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id = row
                .get::<String, _>("id")
                .parse::<Uuid>()
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
            ids.push(id);
        }
        Ok(ids)
    }

    // ------------------------------------------------------------------
    // Versioning
    // ------------------------------------------------------------------

    /// Finds the best reusable workflow ancestor for a goal: the
    /// successful, non-expired record with the same goal fingerprint
    /// that was replayed most often. Returns its id and version.
    pub async fn best_reusable_ancestor(
        &self,
        fingerprint: &str,
    ) -> Result<Option<(Uuid, u64)>, DatabaseError> {
        type Row = (String, String, i64, i64);
        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT id, goal, version, replay_count
            FROM execution_memory
            WHERE status = 'success' AND retention != 'expired'
            ORDER BY replay_count DESC, created_at DESC
            LIMIT 200
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter(|(_, goal, _, _)| goal_fingerprint(goal) == fingerprint)
            .map(|(id, _, version, _)| {
                let id = Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?;
                Ok((id, version.max(1) as u64))
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?
            .into_iter()
            .next())
    }

    // ------------------------------------------------------------------
    // Lineage
    // ------------------------------------------------------------------

    /// Records a lineage edge (version derivation or duplicate merge).
    pub async fn insert_lineage(
        &self,
        memory_id: Uuid,
        parent_id: Uuid,
        relation: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO memory_lineage (id, memory_id, parent_id, relation, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(memory_id.to_string())
        .bind(parent_id.to_string())
        .bind(relation)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Loads every lineage edge (lineage graphs are built in pure code).
    pub async fn load_lineage_edges(&self) -> Result<Vec<LineageEdge>, DatabaseError> {
        type Row = (String, String, String);
        let rows: Vec<Row> =
            sqlx::query_as("SELECT memory_id, parent_id, relation FROM memory_lineage")
                .fetch_all(&self.pool)
                .await?;
        let mut edges = Vec::with_capacity(rows.len());
        for (memory_id, parent_id, relation) in rows {
            let relation = match relation.as_str() {
                "merged" => crate::copilot::memory::models::LineageRelation::Merged,
                _ => crate::copilot::memory::models::LineageRelation::Parent,
            };
            let memory_id =
                Uuid::parse_str(&memory_id).map_err(|e| DatabaseError::IoError(e.to_string()))?;
            let parent_id =
                Uuid::parse_str(&parent_id).map_err(|e| DatabaseError::IoError(e.to_string()))?;
            edges.push(LineageEdge {
                memory_id,
                parent_id,
                relation,
            });
        }
        Ok(edges)
    }

    // ------------------------------------------------------------------
    // Compression
    // ------------------------------------------------------------------

    /// Ids of records that are compressible: not yet compressed and with
    /// a reasoning or step history at/above the thresholds (JSON arrays
    /// are stored as text, so `json_array_length` counts entries).
    pub async fn list_compressible(
        &self,
        limit: usize,
        reasoning_threshold: usize,
        steps_threshold: usize,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        let sql = "SELECT id FROM execution_memory WHERE compressed_at IS NULL \
             AND (json_array_length(reasoning) >= ? OR json_array_length(steps) >= ?) LIMIT ?";
        let rows = sqlx::query(sql)
            .bind(reasoning_threshold as i64)
            .bind(steps_threshold as i64)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id = row
                .get::<String, _>("id")
                .parse::<Uuid>()
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Compresses a record in place: replaces its reasoning history with
    /// the summary entry and stamps `summary`/`compressed_at`.
    pub async fn set_compressed(&self, id: Uuid, summary: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE execution_memory
            SET reasoning = ?, summary = ?, compressed_at = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(serde_json::json!([summary]).to_string())
        .bind(summary)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Preserves the original reasoning/steps before compression so the
    /// history can be restored on demand.
    pub async fn save_compression_archive(
        &self,
        id: Uuid,
        reasoning_json: &str,
        steps_json: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO memory_compression_archive
                (memory_id, original_reasoning, original_steps, compressed_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(id.to_string())
        .bind(reasoning_json)
        .bind(steps_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Restores a compressed record from its archive: the original
    /// reasoning/steps return, the summary fields clear, and the archive
    /// row is removed. Returns `None` when the record was not compressed.
    pub async fn restore_compressed(
        &self,
        id: Uuid,
    ) -> Result<Option<(String, String)>, DatabaseError> {
        let mut tx = self.pool.begin().await?;
        type Row = (String, String);
        let row: Option<Row> = sqlx::query_as(
            "SELECT original_reasoning, original_steps FROM memory_compression_archive WHERE memory_id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        let Some((reasoning, steps)) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        sqlx::query(
            "UPDATE execution_memory SET reasoning = ?, steps = ?, summary = NULL, compressed_at = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&reasoning)
        .bind(&steps)
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM memory_compression_archive WHERE memory_id = ?")
            .bind(id.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Some((reasoning, steps)))
    }

    /// Number of compressed records and preserved archive entries.
    pub async fn compressed_counts(&self) -> Result<(u64, u64), DatabaseError> {
        type Row = (i64, i64);
        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM execution_memory WHERE compressed_at IS NOT NULL),
                (SELECT COUNT(*) FROM memory_compression_archive)
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(match row {
            Some((compressed, archived)) => (compressed.max(0) as u64, archived.max(0) as u64),
            None => (0, 0),
        })
    }

    // ------------------------------------------------------------------
    // Snapshots
    // ------------------------------------------------------------------

    /// Stores a snapshot payload (the export JSON).
    pub async fn insert_snapshot(
        &self,
        id: Uuid,
        label: &str,
        data: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT OR REPLACE INTO memory_snapshots (id, label, data, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(label)
        .bind(data)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Metadata + payload of every snapshot, newest first.
    pub async fn list_snapshots(&self) -> Result<Vec<SnapshotRow>, DatabaseError> {
        type Row = (String, String, String, String);
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, label, data, created_at FROM memory_snapshots ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut snapshots = Vec::with_capacity(rows.len());
        for (id, label, data, created_at) in rows {
            snapshots.push(SnapshotRow {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                label,
                created_at: DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                data,
            });
        }
        Ok(snapshots)
    }

    /// Payload (export JSON) of one snapshot, if any.
    pub async fn snapshot_data(&self, id: Uuid) -> Result<Option<String>, DatabaseError> {
        type Row = (String,);
        let row: Option<Row> = sqlx::query_as("SELECT data FROM memory_snapshots WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(data,)| data))
    }

    /// Keeps only the `keep` newest snapshots (prunes the rest).
    pub async fn prune_snapshots(&self, keep: usize) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            DELETE FROM memory_snapshots WHERE id NOT IN (
                SELECT id FROM memory_snapshots ORDER BY created_at DESC LIMIT ?
            )
            "#,
        )
        .bind(keep as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Deletes every memory row — used by snapshot restore (cascades
    /// acceptance, lineage, compression archive, and vector index rows).
    pub async fn clear_store(&self) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM execution_memory")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Storage statistics
    // ------------------------------------------------------------------

    /// Size of the SQLite database file, bytes (`page_count * page_size`).
    pub async fn database_size(&self) -> Result<u64, DatabaseError> {
        type Row = (i64, i64);
        let row: Option<Row> = sqlx::query_as(
            "SELECT page_count, page_size FROM pragma_page_count(), pragma_page_size()",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(match row {
            Some((pages, size)) => (pages.max(0) as u64).saturating_mul(size.max(0) as u64),
            None => 0,
        })
    }

    /// Bytes occupied by stored vectors (embeddings + goal text).
    pub async fn vector_index_size(&self) -> Result<u64, DatabaseError> {
        self.sum_size(
            "SELECT COALESCE(SUM(LENGTH(embedding)), 0) + COALESCE(SUM(LENGTH(text)), 0) AS total FROM memory_vector_index",
        )
        .await
    }

    /// Persistent embedding cache: entry count + byte size.
    pub async fn cache_storage(&self) -> Result<(u64, u64), DatabaseError> {
        type Row = (i64, i64);
        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(LENGTH(embedding)), 0) + COALESCE(SUM(LENGTH(text)), 0)
            FROM memory_embedding_cache
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(match row {
            Some((count, size)) => (count.max(0) as u64, size.max(0) as u64),
            None => (0, 0),
        })
    }

    /// Snapshots: count + total payload bytes.
    pub async fn snapshot_storage(&self) -> Result<(u64, u64), DatabaseError> {
        type Row = (i64, i64);
        let row: Option<Row> =
            sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(LENGTH(data)), 0) FROM memory_snapshots")
                .fetch_optional(&self.pool)
                .await?;
        Ok(match row {
            Some((count, size)) => (count.max(0) as u64, size.max(0) as u64),
            None => (0, 0),
        })
    }

    /// Orphaned vector index rows: index entries whose memory row is
    /// gone (normally prevented by the cascade, but a safety net for
    /// hand-edited stores).
    pub async fn orphaned_vector_ids(&self) -> Result<Vec<Uuid>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT mvi.memory_id
            FROM memory_vector_index mvi
            LEFT JOIN execution_memory em ON em.id = mvi.memory_id
            WHERE em.id IS NULL
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id = row
                .get::<String, _>("memory_id")
                .parse::<Uuid>()
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn sum_size(&self, sql: &str) -> Result<u64, DatabaseError> {
        type Row = (i64,);
        let row: Option<Row> = sqlx::query_as(sql).fetch_optional(&self.pool).await?;
        Ok(row.map(|(size,)| size.max(0) as u64).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{
        ExecutionMemoryRecord, MemoryKind, MemoryOutcome, MemoryStatus,
    };
    use crate::database::test_database;
    use chrono::Duration;

    async fn seed_record(
        repo: &LifecycleRepository,
        goal: &str,
        status: MemoryStatus,
    ) -> ExecutionMemoryRecord {
        let record = ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.to_string(),
            status,
            plan: None,
            steps: vec![],
            reasoning: vec![],
            tools_used: vec![],
            failed_steps: vec![],
            error: None,
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            retention: RetentionPolicy::Permanent,
            retention_until: None,
            archived_at: None,
            expired_at: None,
            summary: None,
            compressed_at: None,
            version: 1,
            parent_id: None,
        };
        sqlx::query(
            r#"
            INSERT INTO execution_memory (
                id, kind, source_id, workspace_id, goal, status, plan, steps,
                reasoning, tools_used, failed_steps, error, outcome,
                goal_embedding, goal_embedding_dim, replay_count, created_at, updated_at,
                retention, retention_until, archived_at, expired_at, summary,
                compressed_at, version, parent_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.id.to_string())
        .bind(record.kind.to_string())
        .bind(record.source_id.to_string())
        .bind::<Option<String>>(None)
        .bind(&record.goal)
        .bind(record.status.to_string())
        .bind::<Option<String>>(None)
        .bind(serde_json::to_string(&record.steps).unwrap())
        .bind(serde_json::to_string(&record.reasoning).unwrap())
        .bind(serde_json::to_string(&record.tools_used).unwrap())
        .bind(serde_json::to_string(&record.failed_steps).unwrap())
        .bind::<Option<String>>(None)
        .bind(serde_json::to_string(&record.outcome).unwrap())
        .bind::<Option<Vec<u8>>>(None)
        .bind::<Option<i64>>(None)
        .bind(record.replay_count as i64)
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .bind(record.retention.to_string())
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind(record.version as i64)
        .bind::<Option<String>>(None)
        .execute(repo.pool())
        .await
        .expect("seed record");
        record
    }

    #[tokio::test]
    async fn retention_transitions_and_due_lists() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        let record = seed_record(&repo, "g", MemoryStatus::Success).await;

        // Temporary with a future deadline: not due yet.
        repo.update_retention(
            record.id,
            &RetentionPolicy::Temporary,
            Some(Utc::now() + Duration::days(1)),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(repo.list_due_temporary(10).await.unwrap().is_empty());

        // Past the deadline: due, and mark_expired flips it.
        repo.update_retention(
            record.id,
            &RetentionPolicy::Temporary,
            Some(Utc::now() - Duration::minutes(1)),
            None,
            None,
        )
        .await
        .unwrap();
        let due = repo.list_due_temporary(10).await.unwrap();
        assert_eq!(due, vec![record.id]);

        repo.mark_expired(record.id).await.unwrap();
        assert_eq!(repo.list_expired(10).await.unwrap(), vec![record.id]);
        assert_eq!(
            repo.count_by_retention(&RetentionPolicy::Expired)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            repo.count_by_retention(&RetentionPolicy::Permanent)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn best_reusable_ancestor_prefers_replayed_successes() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        let replayed = seed_record(&repo, "resume my focus session", MemoryStatus::Success).await;
        let other = seed_record(&repo, "resume my focus session", MemoryStatus::Success).await;
        seed_record(&repo, "unrelated goal", MemoryStatus::Success).await;
        seed_record(&repo, "resume my focus session", MemoryStatus::Failed).await;

        sqlx::query("UPDATE execution_memory SET replay_count = 5, version = 2 WHERE id = ?")
            .bind(replayed.id.to_string())
            .execute(repo.pool())
            .await
            .unwrap();
        let mut expired = other.clone();
        expired.id = Uuid::new_v4();
        expired.goal = "resume my focus session".into();
        sqlx::query(
            "INSERT INTO execution_memory (id, kind, source_id, goal, status, steps, reasoning, tools_used, failed_steps, outcome, replay_count, created_at, updated_at, retention, version) VALUES (?, 'execution', ?, ?, 'success', '[]', '[]', '[]', '[]', '{}', 9, ?, ?, 'expired', 1)",
        )
        .bind(expired.id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(&expired.goal)
        .bind(Utc::now().to_rfc3339())
        .bind(Utc::now().to_rfc3339())
        .execute(repo.pool())
        .await
        .unwrap();

        let ancestor = repo
            .best_reusable_ancestor(&goal_fingerprint("RESUME my focus session"))
            .await
            .unwrap()
            .expect("reused ancestor exists");
        assert_eq!(ancestor.0, replayed.id, "most-replayed success wins");
        assert_eq!(ancestor.1, 2, "ancestor version inherited");

        // No successful match for an unknown fingerprint.
        assert!(repo
            .best_reusable_ancestor(&goal_fingerprint("nothing similar"))
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn lineage_edges_round_trip_and_cascade() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        let parent = seed_record(&repo, "parent", MemoryStatus::Success).await;
        let child = seed_record(&repo, "child", MemoryStatus::Success).await;

        repo.insert_lineage(child.id, parent.id, "parent")
            .await
            .unwrap();
        repo.insert_lineage(child.id, parent.id, "parent")
            .await
            .unwrap();
        let edges = repo.load_lineage_edges().await.unwrap();
        assert_eq!(edges.len(), 1, "INSERT OR IGNORE dedupes edges");
        assert_eq!(edges[0].memory_id, child.id);
        assert_eq!(edges[0].parent_id, parent.id);

        sqlx::query("DELETE FROM execution_memory WHERE id = ?")
            .bind(parent.id.to_string())
            .execute(repo.pool())
            .await
            .unwrap();
        assert!(
            repo.load_lineage_edges().await.unwrap().is_empty(),
            "edges cascade when either end dies"
        );
    }

    #[tokio::test]
    async fn compression_archive_restores_originals() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        let record = seed_record(&repo, "g", MemoryStatus::Success).await;
        let reasoning = serde_json::Value::Array(vec![serde_json::json!("a"); 90]).to_string();
        let steps = serde_json::Value::Array(vec![serde_json::json!("s"); 10]).to_string();
        sqlx::query("UPDATE execution_memory SET reasoning = ?, steps = ? WHERE id = ?")
            .bind(&reasoning)
            .bind(&steps)
            .bind(record.id.to_string())
            .execute(repo.pool())
            .await
            .unwrap();

        let compressible = repo.list_compressible(10, 80, 150).await.unwrap();
        assert_eq!(compressible, vec![record.id]);

        repo.save_compression_archive(record.id, &reasoning, &steps)
            .await
            .unwrap();
        repo.set_compressed(record.id, "90 reasoning events")
            .await
            .unwrap();

        let restored = repo
            .restore_compressed(record.id)
            .await
            .unwrap()
            .expect("archive exists");
        assert_eq!(restored.0, reasoning);
        assert_eq!(restored.1, steps);
        assert!(
            repo.restore_compressed(record.id).await.unwrap().is_none(),
            "archive consumed by restore"
        );
        assert_eq!(repo.compressed_counts().await.unwrap(), (0, 0));
    }

    #[tokio::test]
    async fn snapshots_insert_list_prune_and_clear() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        let now = Utc::now();
        for i in 0..3 {
            repo.insert_snapshot(Uuid::new_v4(), &format!("auto-{i}"), &format!("data {i}"))
                .await
                .unwrap();
        }
        let rows = repo.list_snapshots().await.unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[0].created_at >= rows[2].created_at, "newest first");
        assert!(rows
            .iter()
            .all(|r| r.created_at >= now - Duration::seconds(5)));

        repo.prune_snapshots(1).await.unwrap();
        assert_eq!(repo.list_snapshots().await.unwrap().len(), 1);

        let (count, _) = repo.snapshot_storage().await.unwrap();
        assert_eq!(count, 1);

        let first = seed_record(&repo, "g", MemoryStatus::Success).await;
        repo.clear_store().await.unwrap();
        assert_eq!(
            repo.count_by_retention(&RetentionPolicy::Permanent)
                .await
                .unwrap(),
            0
        );
        let _ = first;
    }

    #[tokio::test]
    async fn storage_stats_reflect_the_store() {
        let (database, _guard) = test_database().await;
        let repo = LifecycleRepository::new(database.pool().clone());
        assert!(repo.database_size().await.unwrap() > 0);
        assert_eq!(repo.vector_index_size().await.unwrap(), 0);

        let record = seed_record(&repo, "g", MemoryStatus::Success).await;
        sqlx::query(
            "INSERT INTO memory_vector_index (memory_id, text_hash, text, embedding, dim, indexed_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(record.id.to_string())
        .bind("hash")
        .bind("g")
        .bind([0.0f32, 1.0].iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>())
        .bind(2i64)
        .bind(Utc::now().to_rfc3339())
        .execute(repo.pool())
        .await
        .unwrap();
        assert_eq!(
            repo.vector_index_size().await.unwrap(),
            9,
            "8 bytes embedding + 1 text"
        );

        let (count, size) = repo.cache_storage().await.unwrap();
        assert_eq!(count, 0);
        assert_eq!(size, 0);
        assert!(repo.orphaned_vector_ids().await.unwrap().is_empty());

        sqlx::query("DELETE FROM execution_memory WHERE id = ?")
            .bind(record.id.to_string())
            .execute(repo.pool())
            .await
            .unwrap();
        assert!(
            repo.orphaned_vector_ids().await.unwrap().is_empty(),
            "the FK cascade removes index rows with their memory"
        );
    }
}
