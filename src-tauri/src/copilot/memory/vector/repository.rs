//! Memory Vector Repository - SQLite persistence for the vector memory
//! system (RC-6 M2).
//!
//! Owns the SQL for two tables:
//! - `memory_vector_index` — the durable vector index: one row per
//!   indexed memory record, with the embedded goal text, the embedding
//!   BLOB, and `indexed_at` (used for incremental indexing).
//! - `memory_embedding_cache` — the persistent text -> embedding cache.
//!
//! Reads `execution_memory` (read-only) to detect which records need
//! indexing: those with no index row yet, or whose goal changed since
//! their last `indexed_at` (automatic re-indexing when memories change).

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::copilot::memory::models::embedding_from_blob;
use crate::errors::DatabaseError;

/// Repository for the memory vector index and embedding cache.
#[derive(Clone)]
pub struct MemoryVectorRepository {
    pool: SqlitePool,
}

/// A persisted index row, decoded for the in-memory warm-up.
#[derive(Debug, Clone)]
pub struct IndexedVector {
    pub memory_id: Uuid,
    pub text: String,
    pub embedding: Vec<f32>,
}

impl MemoryVectorRepository {
    /// Creates a new vector repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Vector index
    // ------------------------------------------------------------------

    /// Upserts a memory record's index row, stamping `indexed_at`.
    pub async fn upsert_index(
        &self,
        memory_id: Uuid,
        text: &str,
        embedding: &[f32],
    ) -> Result<(), DatabaseError> {
        let blob = crate::copilot::memory::models::embedding_to_blob(embedding);
        sqlx::query(
            r#"
            INSERT INTO memory_vector_index (memory_id, text_hash, text, embedding, dim, indexed_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(memory_id) DO UPDATE SET
                text_hash = excluded.text_hash,
                text = excluded.text,
                embedding = excluded.embedding,
                dim = excluded.dim,
                indexed_at = excluded.indexed_at
            "#,
        )
        .bind(memory_id.to_string())
        .bind(crate::copilot::memory::models::text_hash(text))
        .bind(text)
        .bind(blob)
        .bind(embedding.len() as i64)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Removes one memory's index row (duplicate merge, RC-6 M3). The
    /// caller is responsible for the in-memory index.
    pub async fn remove_index(&self, memory_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM memory_vector_index WHERE memory_id = ?")
            .bind(memory_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Loads every indexed vector (used to warm up the in-memory index at
    /// startup).
    pub async fn load_vectors(&self) -> Result<Vec<IndexedVector>, DatabaseError> {
        let rows = sqlx::query(
            "SELECT memory_id, text, embedding FROM memory_vector_index WHERE embedding IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut vectors = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let memory_id = row
                .get::<String, _>("memory_id")
                .parse::<Uuid>()
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
            let text = row.get::<String, _>("text");
            let blob = row
                .get::<Option<Vec<u8>>, _>("embedding")
                .unwrap_or_default();
            vectors.push(IndexedVector {
                memory_id,
                text,
                embedding: embedding_from_blob(&blob),
            });
        }
        Ok(vectors)
    }

    /// Removes every index row (full re-index).
    pub async fn clear_index(&self) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM memory_vector_index")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Number of memory records with an index row.
    pub async fn count_indexed(&self) -> Result<u64, DatabaseError> {
        type Row = (i64,);
        let row: Option<Row> =
            sqlx::query_as("SELECT COUNT(*) FROM memory_vector_index WHERE embedding IS NOT NULL")
                .fetch_optional(&self.pool)
                .await?;
        Ok(match row {
            Some((count,)) => count.max(0) as u64,
            None => 0,
        })
    }

    /// Total execution memory records.
    pub async fn count_total_records(&self) -> Result<u64, DatabaseError> {
        type Row = (i64,);
        let row: Option<Row> = sqlx::query_as("SELECT COUNT(*) FROM execution_memory")
            .fetch_optional(&self.pool)
            .await?;
        Ok(match row {
            Some((count,)) => count.max(0) as u64,
            None => 0,
        })
    }

    /// Memory records that still need embedding: no index row yet, an
    /// index row without an embedding, or a goal newer than the last
    /// index pass (`updated_at > indexed_at`). Oldest first, capped at
    /// `limit` so a single pass never floods a slow provider.
    pub async fn list_pending(&self, limit: usize) -> Result<Vec<(Uuid, String)>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT em.id, em.goal
            FROM execution_memory em
            LEFT JOIN memory_vector_index mvi ON mvi.memory_id = em.id
            WHERE mvi.memory_id IS NULL
               OR mvi.embedding IS NULL
               OR em.updated_at > mvi.indexed_at
            ORDER BY em.created_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        let mut pending = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            let id = row
                .get::<String, _>("id")
                .parse::<Uuid>()
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
            pending.push((id, row.get::<String, _>("goal")));
        }
        Ok(pending)
    }

    /// Count of records still needing embedding (same predicate as
    /// [`Self::list_pending`]).
    pub async fn count_pending(&self) -> Result<u64, DatabaseError> {
        type Row = (i64,);
        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM execution_memory em
            LEFT JOIN memory_vector_index mvi ON mvi.memory_id = em.id
            WHERE mvi.memory_id IS NULL
               OR mvi.embedding IS NULL
               OR em.updated_at > mvi.indexed_at
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(match row {
            Some((count,)) => count.max(0) as u64,
            None => 0,
        })
    }

    /// Timestamp of the most recent index write, if any.
    pub async fn last_indexed_at(&self) -> Result<Option<DateTime<Utc>>, DatabaseError> {
        type Row = (Option<String>,);
        let row: Option<Row> = sqlx::query_as("SELECT MAX(indexed_at) FROM memory_vector_index")
            .fetch_optional(&self.pool)
            .await?;
        match row {
            Some((Some(stamp),)) => Ok(Some(
                DateTime::parse_from_rfc3339(&stamp)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
            )),
            _ => Ok(None),
        }
    }

    // ------------------------------------------------------------------
    // Embedding cache
    // ------------------------------------------------------------------

    /// Looks up a cached embedding, returning the stored text alongside it
    /// so the caller can verify the hash key actually maps to this text.
    pub async fn cache_get(
        &self,
        text_hash: &str,
    ) -> Result<Option<(String, Vec<f32>)>, DatabaseError> {
        let rows =
            sqlx::query("SELECT text, embedding FROM memory_embedding_cache WHERE text_hash = ?")
                .bind(text_hash)
                .fetch_all(&self.pool)
                .await?;
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            results.push((
                row.get::<String, _>("text"),
                embedding_from_blob(&row.get::<Vec<u8>, _>("embedding")),
            ));
        }
        Ok(results.into_iter().next())
    }

    /// Stores an embedding in the persistent cache.
    pub async fn cache_put(
        &self,
        text_hash: &str,
        text: &str,
        embedding: &[f32],
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO memory_embedding_cache (text_hash, text, embedding, dim, created_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(text_hash) DO UPDATE SET
                text = excluded.text,
                embedding = excluded.embedding,
                dim = excluded.dim
            "#,
        )
        .bind(text_hash)
        .bind(text)
        .bind(crate::copilot::memory::models::embedding_to_blob(embedding))
        .bind(embedding.len() as i64)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Removes every persistent cache entry.
    pub async fn cache_clear(&self) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM memory_embedding_cache")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{
        embedding_to_blob, ExecutionMemoryRecord, MemoryKind, MemoryOutcome, MemoryStatus,
    };
    use crate::database::test_database;

    async fn sample_record(
        repository: &MemoryVectorRepository,
        goal: &str,
        updated_at: DateTime<Utc>,
    ) -> Uuid {
        let pool: SqlitePool = repository.pool.clone();
        let record = ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.to_string(),
            status: MemoryStatus::Success,
            plan: None,
            steps: vec![],
            reasoning: vec![],
            tools_used: vec![],
            failed_steps: vec![],
            error: None,
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
            created_at: updated_at,
            updated_at,
        };
        // Insert directly (the memory repository has its own tests; this
        // helper keeps vector tests focused on the index SQL).
        sqlx::query(
            r#"
            INSERT INTO execution_memory (
                id, kind, source_id, workspace_id, goal, status, plan, steps,
                reasoning, tools_used, failed_steps, error, outcome,
                goal_embedding, goal_embedding_dim, replay_count, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.id.to_string())
        .bind(record.kind.to_string())
        .bind(record.source_id.to_string())
        .bind::<Option<String>>(None)
        .bind(&record.goal)
        .bind(record.status.to_string())
        .bind::<Option<String>>(None)
        .bind("[]")
        .bind("[]")
        .bind("[]")
        .bind("[]")
        .bind::<Option<String>>(None)
        .bind("{}")
        .bind::<Option<Vec<u8>>>(None)
        .bind::<Option<i64>>(None)
        .bind(0i64)
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .execute(&pool)
        .await
        .expect("seed record");
        record.id
    }

    #[tokio::test]
    async fn index_upsert_round_trips_and_counts() {
        let (database, _guard) = test_database().await;
        let repo = MemoryVectorRepository::new(database.pool().clone());
        let id = sample_record(&repo, "resume focus", Utc::now()).await;
        assert_eq!(repo.count_indexed().await.unwrap(), 0);
        assert_eq!(repo.count_total_records().await.unwrap(), 1);

        repo.upsert_index(id, "resume focus", &[0.1, 0.2, 0.3])
            .await
            .unwrap();
        assert_eq!(repo.count_indexed().await.unwrap(), 1);

        let vectors = repo.load_vectors().await.unwrap();
        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].memory_id, id);
        assert_eq!(vectors[0].text, "resume focus");
        assert_eq!(vectors[0].embedding, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn pending_tracks_new_and_changed_records() {
        let (database, _guard) = test_database().await;
        let repo = MemoryVectorRepository::new(database.pool().clone());
        let old = sample_record(
            &repo,
            "resume focus",
            Utc::now() - chrono::Duration::days(1),
        )
        .await;
        let recent = sample_record(&repo, "organize receipts", Utc::now()).await;

        // Both are pending before indexing.
        let pending = repo.list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 2);

        // Indexing both removes them from the pending set.
        repo.upsert_index(old, "resume focus", &[1.0, 0.0])
            .await
            .unwrap();
        repo.upsert_index(recent, "organize receipts", &[1.0, 0.0])
            .await
            .unwrap();
        let pending = repo.list_pending(10).await.unwrap();
        assert!(pending.is_empty());
        assert_eq!(repo.count_pending().await.unwrap(), 0);

        // A goal change (updated_at newer than indexed_at) re-pends it.
        let changed_at = Utc::now() + chrono::Duration::minutes(1);
        sqlx::query("UPDATE execution_memory SET goal = ?, updated_at = ? WHERE id = ?")
            .bind("resume focus changed")
            .bind(changed_at.to_rfc3339())
            .bind(old.to_string())
            .execute(database.pool())
            .await
            .unwrap();
        let pending = repo.list_pending(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, old);
    }

    #[tokio::test]
    async fn last_indexed_at_reflects_latest_write() {
        let (database, _guard) = test_database().await;
        let repo = MemoryVectorRepository::new(database.pool().clone());
        assert!(repo.last_indexed_at().await.unwrap().is_none());
        let id = sample_record(&repo, "g", Utc::now()).await;
        repo.upsert_index(id, "g", &[1.0]).await.unwrap();
        assert!(repo.last_indexed_at().await.unwrap().is_some());
    }

    #[tokio::test]
    async fn remove_index_drops_only_the_requested_memory() {
        let (database, _guard) = test_database().await;
        let repo = MemoryVectorRepository::new(database.pool().clone());
        let a = sample_record(&repo, "goal a", Utc::now()).await;
        let b = sample_record(&repo, "goal b", Utc::now()).await;
        repo.upsert_index(a, "goal a", &[1.0, 0.0]).await.unwrap();
        repo.upsert_index(b, "goal b", &[0.0, 1.0]).await.unwrap();
        assert_eq!(repo.count_indexed().await.unwrap(), 2);

        repo.remove_index(a).await.unwrap();
        assert_eq!(repo.count_indexed().await.unwrap(), 1);
        let vectors = repo.load_vectors().await.unwrap();
        assert_eq!(vectors.len(), 1);
        assert_eq!(
            vectors[0].memory_id, b,
            "the other memory's vector survives"
        );

        // The durable cascade also covers the record: deleting the memory
        // row removes the (already removed) index row safely.
        sqlx::query("DELETE FROM execution_memory WHERE id = ?")
            .bind(b.to_string())
            .execute(database.pool())
            .await
            .unwrap();
        assert_eq!(repo.count_indexed().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn cache_get_put_round_trips_with_text_guard() {
        let (database, _guard) = test_database().await;
        let repo = MemoryVectorRepository::new(database.pool().clone());
        let hash = crate::copilot::memory::models::text_hash("resume focus");
        assert!(repo.cache_get(&hash).await.unwrap().is_none());

        repo.cache_put(&hash, "resume focus", &[0.5, 0.25])
            .await
            .unwrap();
        let (stored_text, embedding) = repo.cache_get(&hash).await.unwrap().unwrap();
        assert_eq!(stored_text, "resume focus");
        assert_eq!(embedding, vec![0.5, 0.25]);

        let _ = embedding_to_blob(&[]);
        repo.cache_clear().await.unwrap();
        assert!(repo.cache_get(&hash).await.unwrap().is_none());
    }
}
