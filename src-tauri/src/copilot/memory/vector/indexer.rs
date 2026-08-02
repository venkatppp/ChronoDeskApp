//! Memory Indexer - the background worker that keeps the vector index in
//! sync with execution memory (RC-6 M2).
//!
//! Responsibilities:
//! - **Incremental embedding generation** — every pass asks the vector
//!   repository which records still need an embedding (new records or
//!   records whose goal changed since their last index write) and embeds
//!   only those, in batches.
//! - **Batch embedding** — uncached texts reach the provider through
//!   [`VectorProvider::embed_batch`] so providers with real batch paths
//!   (e.g. ONNX) can amortize; the cache-aware provider already filters
//!   out cached texts.
//! - **Automatic re-indexing** — captures notify the worker
//!   (`notify()`), which debounces and runs a pass; a periodic interval
//!   is the safety net. `reindex_all()` drops and rebuilds the whole
//!   index.
//! - **Durable consistency** — every index write lands in the in-memory
//!   k-NN index *and* the SQLite index, and back-fills the
//!   `execution_memory.goal_embedding` column so retrieval's ranking
//!   blend can use embeddings.
//!
//! The worker never plans, schedules, or executes: it only indexes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use uuid::Uuid;

use crate::copilot::memory::repository::MemoryRepository;
use crate::copilot::memory::vector::index::VectorIndex;
use crate::copilot::memory::vector::provider::VectorProvider;
use crate::copilot::memory::vector::repository::MemoryVectorRepository;
use crate::copilot::memory::vector::IndexResult;
use crate::errors::DatabaseError;

/// Debounce after a capture notification before running an index pass
/// (collapses bursts of captures into one batch).
const DEBOUNCE: Duration = Duration::from_millis(150);
/// Safety-net interval: a pass runs at least this often even without
/// notifications.
const INDEX_INTERVAL: Duration = Duration::from_secs(60);
/// How many pending records a single pass processes at most.
const DEFAULT_BATCH_SIZE: usize = 64;

/// The background vector index worker.
#[derive(Clone)]
pub struct MemoryIndexer {
    repository: MemoryRepository,
    vector_repository: MemoryVectorRepository,
    provider: Arc<dyn VectorProvider>,
    index: VectorIndex,
    notify: Arc<Notify>,
    shutdown: Arc<AtomicBool>,
    batch_size: usize,
}

impl MemoryIndexer {
    /// Creates an indexer over the given repositories/provider/index.
    pub fn new(
        repository: MemoryRepository,
        vector_repository: MemoryVectorRepository,
        provider: Arc<dyn VectorProvider>,
        index: VectorIndex,
    ) -> Self {
        Self {
            repository,
            vector_repository,
            provider,
            index,
            notify: Arc::new(Notify::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Sets the number of records embedded per provider call (test hook).
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Wakes the worker so it runs an index pass (called by captures).
    pub fn notify(&self) {
        self.notify.notify_one();
    }

    /// Requests the worker loop to stop (used by tests; the app-lifetime
    /// worker is simply dropped).
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.notify.notify_one();
    }

    /// Rebuilds the in-memory index from the durable SQLite index (startup
    /// warm-up). Returns the number of vectors loaded.
    pub async fn warm_up(&self) -> Result<usize, DatabaseError> {
        let vectors = self.vector_repository.load_vectors().await?;
        let count = vectors.len();
        for vector in vectors {
            self.index
                .upsert(vector.memory_id, &vector.text, vector.embedding);
        }
        Ok(count)
    }

    /// One index pass: embeds every pending record (new or changed) in
    /// batches and persists the results everywhere.
    pub async fn index_pending(&self, limit: usize) -> Result<IndexResult, DatabaseError> {
        let pending = self.vector_repository.list_pending(limit).await?;
        if pending.is_empty() {
            return Ok(IndexResult::default());
        }
        let total = pending.len();
        let mut indexed = 0usize;
        let mut failed = 0usize;

        for chunk in pending.chunks(self.batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|(_, goal)| goal.as_str()).collect();
            let embeddings = match self.provider.embed_batch(&texts).await {
                Ok(embeddings) => embeddings,
                Err(error) => {
                    tracing::warn!(error = %error, "batch embedding failed; skipping chunk");
                    failed += chunk.len();
                    continue;
                }
            };
            for ((memory_id, goal), embedding) in chunk.iter().zip(embeddings.iter()) {
                self.write_index(*memory_id, goal, embedding).await;
            }
            indexed += chunk.len();
        }

        Ok(IndexResult {
            requested: total,
            indexed,
            failed,
            skipped: total - indexed - failed,
        })
    }

    /// Drops the whole index (SQL + memory) and rebuilds it from scratch.
    pub async fn reindex_all(&self) -> Result<IndexResult, DatabaseError> {
        self.vector_repository.clear_index().await?;
        self.index.clear();
        self.index_pending(usize::MAX).await
    }

    /// The worker loop: waits for capture notifications (debounced) and
    /// the safety-net interval, running an index pass each time, until
    /// [`Self::shutdown`].
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(INDEX_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = self.notify.notified() => {
                    tokio::time::sleep(DEBOUNCE).await;
                    self.run_pass().await;
                }
                _ = interval.tick() => self.run_pass().await,
                _ = self.wait_shutdown() => break,
            }
        }
    }

    async fn run_pass(&self) {
        match self.index_pending(usize::MAX).await {
            Ok(result) if result.indexed > 0 => {
                tracing::info!(
                    indexed = result.indexed,
                    "memory vector index pass complete"
                );
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(error = %error, "memory vector index pass failed"),
        }
    }

    async fn wait_shutdown(&self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Persists one record's embedding: execution_memory column, the
    /// SQLite index row, and the in-memory k-NN index. Best-effort per
    /// record — a failure is logged and never aborts the pass.
    async fn write_index(&self, memory_id: Uuid, goal: &str, embedding: &[f32]) {
        if let Err(error) = self
            .repository
            .update_goal_embedding(memory_id, Some(embedding))
            .await
        {
            tracing::warn!(error = %error, %memory_id, "failed to back-fill goal embedding");
        }
        if let Err(error) = self
            .vector_repository
            .upsert_index(memory_id, goal, embedding)
            .await
        {
            tracing::warn!(error = %error, %memory_id, "failed to persist index row");
        }
        self.index.upsert(memory_id, goal, embedding.to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{
        ExecutionMemoryRecord, MemoryKind, MemoryOutcome, MemoryStatus, RetentionPolicy,
    };
    use crate::copilot::memory::vector::local::LocalVectorProvider;
    use crate::database::test_database;
    use chrono::Utc;

    fn record(goal: &str) -> ExecutionMemoryRecord {
        ExecutionMemoryRecord {
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
        }
    }

    async fn setup() -> (
        tempfile::TempDir,
        MemoryIndexer,
        MemoryRepository,
        MemoryVectorRepository,
        VectorIndex,
    ) {
        let (database, guard) = test_database().await;
        let repository = MemoryRepository::new(database.pool().clone());
        let vector_repository = MemoryVectorRepository::new(database.pool().clone());
        let provider: Arc<dyn VectorProvider> = Arc::new(LocalVectorProvider::default());
        let index = VectorIndex::new();
        let indexer = MemoryIndexer::new(
            repository.clone(),
            vector_repository.clone(),
            provider,
            index.clone(),
        );
        (guard, indexer, repository, vector_repository, index)
    }

    #[tokio::test]
    async fn index_pending_embeds_new_records_everywhere() {
        let (_guard, indexer, repository, vector_repository, index) = setup().await;
        let captured = record("resume my focus session");
        repository.upsert(&captured).await.unwrap();

        let result = indexer.index_pending(10).await.unwrap();
        assert_eq!(result.requested, 1);
        assert_eq!(result.indexed, 1);
        assert_eq!(result.failed, 0);

        // Durable: SQLite index row + execution_memory column.
        assert_eq!(vector_repository.count_indexed().await.unwrap(), 1);
        assert_eq!(vector_repository.count_pending().await.unwrap(), 0);
        let loaded = repository.get(captured.id).await.unwrap().unwrap();
        let embedding = loaded.goal_embedding.expect("back-filled embedding");
        assert_eq!(embedding.len(), 384);
        // In-memory: the k-NN index can find it.
        assert_eq!(index.len(), 1);
        let hits = index.knn(&embedding, 1);
        assert_eq!(hits[0].0, captured.id);
        assert!((hits[0].1 - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn index_pending_is_incremental() {
        let (_guard, indexer, repository, _vector_repository, _index) = setup().await;
        let first = record("resume my focus session");
        repository.upsert(&first).await.unwrap();
        let second = record("organize tax receipts");
        repository.upsert(&second).await.unwrap();

        indexer.index_pending(10).await.unwrap();
        // Nothing new: a second pass embeds nothing.
        let result = indexer.index_pending(10).await.unwrap();
        assert_eq!(result.requested, 0);
        assert_eq!(result.indexed, 0);

        // A changed goal re-pends the record (automatic re-indexing).
        let mut changed = first.clone();
        changed.goal = "resume my focus session now".to_string();
        changed.updated_at = Utc::now();
        repository.upsert(&changed).await.unwrap();
        let result = indexer.index_pending(10).await.unwrap();
        assert_eq!(result.requested, 1);
        assert_eq!(result.indexed, 1);
    }

    #[tokio::test]
    async fn reindex_all_rebuilds_from_scratch() {
        let (_guard, indexer, repository, vector_repository, index) = setup().await;
        for goal in ["resume focus", "organize receipts", "plan vacation"] {
            repository.upsert(&record(goal)).await.unwrap();
        }
        indexer.index_pending(10).await.unwrap();
        assert_eq!(vector_repository.count_indexed().await.unwrap(), 3);
        assert_eq!(index.len(), 3);

        indexer.reindex_all().await.unwrap();
        assert_eq!(vector_repository.count_indexed().await.unwrap(), 3);
        assert_eq!(vector_repository.count_pending().await.unwrap(), 0);
        assert_eq!(index.len(), 3);
    }

    #[tokio::test]
    async fn warm_up_restores_in_memory_index() {
        let (_guard, indexer, repository, vector_repository, index) = setup().await;
        repository.upsert(&record("resume focus")).await.unwrap();
        indexer.index_pending(10).await.unwrap();
        assert_eq!(index.len(), 1);

        // Simulate a restart: a fresh in-memory index rebuilt from SQL.
        let rebuilt = VectorIndex::new();
        let restarted = MemoryIndexer::new(
            repository,
            vector_repository,
            Arc::new(LocalVectorProvider::default()),
            rebuilt.clone(),
        );
        let warmed = restarted.warm_up().await.unwrap();
        assert_eq!(warmed, 1);
        assert_eq!(rebuilt.len(), 1);
    }

    #[tokio::test]
    async fn run_loop_indexes_notified_captures() {
        let (_guard, indexer, repository, vector_repository, _index) = setup().await;
        let task = tokio::spawn({
            let indexer = indexer.clone();
            async move { indexer.run().await }
        });

        repository
            .upsert(&record("resume my focus session"))
            .await
            .unwrap();
        indexer.notify();

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if vector_repository.count_indexed().await.unwrap() == 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("indexer should index the notified capture within 5s");

        indexer.shutdown();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("worker loop should stop after shutdown")
            .unwrap();
    }

    #[tokio::test]
    async fn batch_size_chunks_pending_work() {
        let (_guard, indexer, repository, _vector_repository, _index) = setup().await;
        for i in 0..5 {
            repository
                .upsert(&record(&format!("goal number {i}")))
                .await
                .unwrap();
        }
        let result = indexer.with_batch_size(2).index_pending(10).await.unwrap();
        assert_eq!(result.requested, 5);
        assert_eq!(result.indexed, 5);
    }
}
