//! Vector memory system (RC-6 M2) — the production-quality replacement
//! for the placeholder semantic retrieval over execution memory.
//!
//! Layering (strict, no duplicated logic):
//!
//! | Layer | File | Owns |
//! |-------|------|------|
//! | Provider abstraction | `provider.rs` | the `VectorProvider` trait + cache-aware decorator |
//! | Local provider | `local.rs` | n-gram hashing embedder (real local embeddings) |
//! | Cache | `cache.rs` | in-memory LRU text->embedding cache + stats |
//! | Index | `index.rs` | in-memory k-NN index (cosine over normalized vectors) |
//! | Repository | `repository.rs` | all SQL: durable index + persistent embedding cache |
//! | Indexer | `indexer.rs` | background worker: incremental, batched, automatic re-indexing |
//!
//! [`MemoryVectorSystem`] composes those layers into the facade the
//! `MemoryEngine` talks to; it never plans or executes anything.

pub mod cache;
pub mod index;
pub mod indexer;
pub mod local;
pub mod provider;
pub mod repository;

pub use cache::{CacheStats, EmbeddingCache};
pub use index::VectorIndex;
pub use indexer::MemoryIndexer;
pub use local::LocalVectorProvider;
pub use provider::{CachedProvider, VectorProvider};
pub use repository::{IndexedVector, MemoryVectorRepository};

use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::copilot::memory::repository::MemoryRepository;
use crate::errors::DatabaseError;

/// Default in-memory cache capacity (texts).
const CACHE_CAPACITY: usize = 512;

/// The vector memory facade: cached provider + k-NN index + durable SQL +
/// background indexer, all behind one cheap-to-clone handle.
#[derive(Clone)]
pub struct MemoryVectorSystem {
    repository: MemoryVectorRepository,
    cache: Arc<EmbeddingCache>,
    provider: Arc<CachedProvider>,
    index: VectorIndex,
    indexer: Arc<MemoryIndexer>,
}

/// Snapshot of the vector index for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorIndexStatus {
    /// Execution memory records total.
    pub total_records: u64,
    /// Records with an embedding in the durable index.
    pub indexed: u64,
    /// Records still waiting for an embedding.
    pub pending: u64,
    /// Embedding provider name.
    pub provider: String,
    /// Embedding dimensionality.
    pub dimensions: usize,
    /// When the last index pass wrote, if any.
    pub last_indexed_at: Option<String>,
    /// In-memory cache occupancy.
    pub cache_size: usize,
    pub cache_capacity: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

/// Outcome of an index pass (dashboard + IPC).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct IndexResult {
    /// Records examined by the pass.
    pub requested: usize,
    /// Records successfully embedded and indexed.
    pub indexed: usize,
    /// Records whose embedding failed.
    pub failed: usize,
    /// Records skipped (e.g. embedding failed but counted separately).
    pub skipped: usize,
}

impl MemoryVectorSystem {
    /// Composes the vector system over a connection pool and provider.
    pub fn new(pool: SqlitePool, provider: Arc<dyn VectorProvider>) -> Self {
        let repository = MemoryVectorRepository::new(pool.clone());
        let cache = Arc::new(EmbeddingCache::new(CACHE_CAPACITY));
        let cached = Arc::new(CachedProvider::new(
            provider,
            cache.clone(),
            repository.clone(),
        ));
        let index = VectorIndex::new();
        let indexer = Arc::new(MemoryIndexer::new(
            MemoryRepository::new(pool),
            repository.clone(),
            cached.clone(),
            index.clone(),
        ));
        Self {
            repository,
            cache,
            provider: cached,
            index,
            indexer,
        }
    }

    /// The background indexer (for wiring the worker and manual passes).
    pub fn indexer(&self) -> &Arc<MemoryIndexer> {
        &self.indexer
    }

    /// Embeds a text through the cache tiers; `None` on provider failure.
    pub async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        self.provider.embed(text).await.ok()
    }

    /// k-NN over the in-memory index: `(memory_id, cosine)` desc.
    pub fn knn(&self, query: &[f32], k: usize) -> Vec<(Uuid, f32)> {
        self.index.knn(query, k)
    }

    /// In-memory embedding cache stats (dashboard storage card).
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Number of records in the in-memory index.
    pub fn index_len(&self) -> usize {
        self.index.len()
    }

    /// Builds the dashboard status payload.
    pub async fn status(&self) -> Result<VectorIndexStatus, DatabaseError> {
        let total = self.repository.count_total_records().await?;
        let indexed = self.repository.count_indexed().await?;
        let pending = self.repository.count_pending().await?;
        let last_indexed_at = self
            .repository
            .last_indexed_at()
            .await?
            .map(|when| when.to_rfc3339());
        let cache_stats = self.cache.stats();
        Ok(VectorIndexStatus {
            total_records: total,
            indexed,
            pending,
            provider: self.provider.name().to_string(),
            dimensions: self.provider.dimensions(),
            last_indexed_at,
            cache_size: cache_stats.size,
            cache_capacity: cache_stats.capacity,
            cache_hits: cache_stats.hits,
            cache_misses: cache_stats.misses,
            cache_hit_rate: cache_stats.hit_rate,
        })
    }

    /// Timestamp used by tests to assert index freshness.
    pub async fn last_indexed_at(&self) -> Result<Option<chrono::DateTime<Utc>>, DatabaseError> {
        self.repository.last_indexed_at().await
    }

    /// Removes a memory from the vector index (duplicate merge, RC-6 M3):
    /// deletes the durable row and drops the in-memory k-NN entry. The
    /// `execution_memory` row itself is deleted by the memory repository.
    pub async fn remove(&self, memory_id: Uuid) -> Result<(), DatabaseError> {
        self.repository.remove_index(memory_id).await?;
        self.index.remove(memory_id);
        Ok(())
    }
}
