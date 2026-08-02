//! Vector Provider abstraction for the execution memory system.
//!
//! [`VectorProvider`] is the single interface the memory system embeds
//! through: one text at a time or in batches. Implementations include the
//! local n-gram provider ([`crate::copilot::memory::vector::LocalVectorProvider`])
//! and, over time, ONNX/remote backends behind the same trait.
//!
//! [`CachedProvider`] decorates any provider with two cache tiers: the
//! in-memory LRU ([`EmbeddingCache`]) and the durable SQLite cache
//! (`memory_embedding_cache`, owned by `MemoryVectorRepository`). A text
//! is embedded only once per process run and once ever across restarts —
//! this is what makes incremental and batch indexing cheap.

use async_trait::async_trait;

use crate::copilot::memory::models::text_hash;
use crate::copilot::memory::vector::cache::EmbeddingCache;
use crate::copilot::memory::vector::repository::MemoryVectorRepository;
use crate::errors::DatabaseError;

/// Text -> vector provider abstraction.
#[async_trait]
pub trait VectorProvider: Send + Sync {
    /// Embeds a single text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError>;

    /// Embeds many texts in one call. The default loops over [`embed`];
    /// providers with real batch paths (e.g. ONNX) override it.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, DatabaseError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    /// Dimensionality of the produced vectors.
    fn dimensions(&self) -> usize;

    /// Provider name, surfaced in the dashboard.
    fn name(&self) -> &str;
}

/// Cache-aware decorator over any [`VectorProvider`]: in-memory LRU
/// first, then the persistent SQLite cache, then the wrapped provider.
#[derive(Clone)]
pub struct CachedProvider {
    inner: Arc<dyn VectorProvider>,
    cache: Arc<EmbeddingCache>,
    repository: MemoryVectorRepository,
}

use std::sync::Arc;

impl CachedProvider {
    /// Creates a cached provider over `inner`.
    pub fn new(
        inner: Arc<dyn VectorProvider>,
        cache: Arc<EmbeddingCache>,
        repository: MemoryVectorRepository,
    ) -> Self {
        Self {
            inner,
            cache,
            repository,
        }
    }

    /// The wrapped provider (name/dimensions passthrough).
    pub fn inner(&self) -> &Arc<dyn VectorProvider> {
        &self.inner
    }

    /// Cache statistics for the dashboard.
    pub fn cache_stats(&self) -> crate::copilot::memory::vector::cache::CacheStats {
        self.cache.stats()
    }

    /// Loads an embedding through the cache tiers.
    async fn get_cached(&self, text: &str) -> Result<Option<Vec<f32>>, DatabaseError> {
        if let Some(embedding) = self.cache.get(text) {
            return Ok(Some(embedding));
        }
        let hash = text_hash(text);
        if let Some((stored_text, embedding)) = self.repository.cache_get(&hash).await? {
            // The hash is the key; the stored text guards against
            // collisions before trusting the cached vector.
            if stored_text == text {
                self.cache.put(text.to_string(), embedding.clone());
                return Ok(Some(embedding));
            }
        }
        Ok(None)
    }

    /// Stores an embedding in both cache tiers.
    async fn store(&self, text: &str, embedding: Vec<f32>) -> Result<(), DatabaseError> {
        self.cache.put(text.to_string(), embedding.clone());
        self.repository
            .cache_put(&text_hash(text), text, &embedding)
            .await
    }
}

#[async_trait]
impl VectorProvider for CachedProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError> {
        if let Some(embedding) = self.get_cached(text).await? {
            return Ok(embedding);
        }
        let embedding = self.inner.embed(text).await?;
        let _ = self.store(text, embedding.clone()).await;
        Ok(embedding)
    }

    /// Cache-aware batch embedding: only the texts missing from both
    /// cache tiers reach the wrapped provider, in one batch call.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, DatabaseError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        let mut uncached: Vec<&str> = Vec::new();
        let mut uncached_indexes: Vec<usize> = Vec::new();

        for (index, text) in texts.iter().enumerate() {
            match self.get_cached(text).await? {
                Some(embedding) => embeddings.push((index, embedding)),
                None => {
                    uncached.push(text);
                    uncached_indexes.push(index);
                }
            }
        }

        if !uncached.is_empty() {
            let generated = self.inner.embed_batch(&uncached).await?;
            for (index, embedding) in uncached_indexes.into_iter().zip(generated) {
                let text = texts[index];
                let _ = self.store(text, embedding.clone()).await;
                embeddings.push((index, embedding));
            }
        }

        embeddings.sort_by_key(|(index, _)| *index);
        Ok(embeddings
            .into_iter()
            .map(|(_, embedding)| embedding)
            .collect())
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::vector::cache::EmbeddingCache;
    use crate::copilot::memory::vector::repository::MemoryVectorRepository;
    use crate::database::test_database;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Counting provider: records how many times the wrapped provider is
    /// actually asked to embed.
    struct CountingProvider {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl VectorProvider for CountingProvider {
        async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(vec![text.len() as f32, 0.0])
        }
        fn dimensions(&self) -> usize {
            2
        }
        fn name(&self) -> &str {
            "counting"
        }
    }

    async fn setup() -> (CachedProvider, Arc<AtomicUsize>, tempfile::TempDir) {
        let (database, guard) = test_database().await;
        let repository = MemoryVectorRepository::new(database.pool().clone());
        let calls = Arc::new(AtomicUsize::new(0));
        let inner: Arc<dyn VectorProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let provider = CachedProvider::new(inner, Arc::new(EmbeddingCache::new(16)), repository);
        (provider, calls, guard)
    }

    #[tokio::test]
    async fn second_embed_served_from_memory_cache() {
        let (provider, calls, _guard) = setup().await;
        let first = provider.embed("resume focus").await.unwrap();
        let second = provider.embed("resume focus").await.unwrap();
        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::Relaxed), 1, "wrapped once only");
    }

    #[tokio::test]
    async fn persistent_cache_serves_after_cache_clear() {
        let (database, _guard) = test_database().await;
        let repository = MemoryVectorRepository::new(database.pool().clone());
        let calls = Arc::new(AtomicUsize::new(0));
        let inner: Arc<dyn VectorProvider> = Arc::new(CountingProvider {
            calls: calls.clone(),
        });
        let provider =
            CachedProvider::new(inner, Arc::new(EmbeddingCache::new(16)), repository.clone());

        let first = provider.embed("organize receipts").await.unwrap();
        provider.cache.clear();
        let second = provider.embed("organize receipts").await.unwrap();
        assert_eq!(first, second);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "SQLite cache survives an in-memory eviction"
        );
    }

    #[tokio::test]
    async fn batch_only_embeds_misses() {
        let (provider, calls, _guard) = setup().await;
        let _ = provider.embed("a b c").await;
        let batch = provider
            .embed_batch(&["a b c", "x y z", "w q r"])
            .await
            .unwrap();
        assert_eq!(batch.len(), 3);
        assert_eq!(calls.load(Ordering::Relaxed), 3, "one cached + two new");
    }
}
