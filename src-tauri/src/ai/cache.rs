//! Caching layers for embeddings and inference results.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// LRU cache for embeddings.
pub struct EmbeddingCache {
    capacity: usize,
    cache: HashMap<u64, (Vec<f32>, u64)>, // hash -> (embedding, access_count)
    access_order: Vec<u64>,
}

impl EmbeddingCache {
    /// Creates a new embedding cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    /// Gets an embedding from the cache.
    pub fn get(&mut self, text: &str) -> Option<Vec<f32>> {
        if self.capacity == 0 {
            return None;
        }

        let hash = Self::hash_text(text);

        if let Some((embedding, access_count)) = self.cache.get_mut(&hash) {
            *access_count += 1;

            // Update access order
            self.access_order.retain(|&h| h != hash);
            self.access_order.push(hash);

            Some(embedding.clone())
        } else {
            None
        }
    }

    /// Puts an embedding into the cache.
    pub fn put(&mut self, text: String, embedding: Vec<f32>) {
        if self.capacity == 0 {
            return;
        }

        let hash = Self::hash_text(&text);

        // Evict if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&hash) {
            if let Some(&lru_hash) = self.access_order.first() {
                self.cache.remove(&lru_hash);
                self.access_order.remove(0);
            }
        }

        self.cache.insert(hash, (embedding, 1));
        self.access_order.push(hash);
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.capacity,
            hit_rate: 0.0, // Tracked separately
        }
    }

    fn hash_text(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

/// Generic inference result cache.
pub struct InferenceCache<T: Clone> {
    capacity: usize,
    cache: HashMap<u64, (T, u64)>, // hash -> (result, access_count)
    access_order: Vec<u64>,
}

impl<T: Clone> InferenceCache<T> {
    /// Creates a new inference cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cache: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    /// Gets a result from the cache.
    pub fn get(&mut self, key: &str) -> Option<T> {
        if self.capacity == 0 {
            return None;
        }

        let hash = Self::hash_key(key);

        if let Some((result, access_count)) = self.cache.get_mut(&hash) {
            *access_count += 1;

            // Update access order
            self.access_order.retain(|&h| h != hash);
            self.access_order.push(hash);

            Some(result.clone())
        } else {
            None
        }
    }

    /// Puts a result into the cache.
    pub fn put(&mut self, key: String, result: T) {
        if self.capacity == 0 {
            return;
        }

        let hash = Self::hash_key(&key);

        // Evict if at capacity
        if self.cache.len() >= self.capacity && !self.cache.contains_key(&hash) {
            if let Some(&lru_hash) = self.access_order.first() {
                self.cache.remove(&lru_hash);
                self.access_order.remove(0);
            }
        }

        self.cache.insert(hash, (result, 1));
        self.access_order.push(hash);
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Returns cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            capacity: self.capacity,
            hit_rate: 0.0, // Tracked separately
        }
    }

    fn hash_key(key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_cache_stores_and_retrieves() {
        let mut cache = EmbeddingCache::new(10);
        let embedding = vec![1.0, 2.0, 3.0];

        cache.put("test".to_string(), embedding.clone());
        let retrieved = cache.get("test");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), embedding);
    }

    #[test]
    fn embedding_cache_evicts_lru() {
        let mut cache = EmbeddingCache::new(2);

        cache.put("test1".to_string(), vec![1.0]);
        cache.put("test2".to_string(), vec![2.0]);
        cache.put("test3".to_string(), vec![3.0]);

        // test1 should be evicted
        assert!(cache.get("test1").is_none());
        assert!(cache.get("test2").is_some());
        assert!(cache.get("test3").is_some());
    }

    #[test]
    fn embedding_cache_updates_access_order() {
        let mut cache = EmbeddingCache::new(2);

        cache.put("test1".to_string(), vec![1.0]);
        cache.put("test2".to_string(), vec![2.0]);

        // Access test1
        let _ = cache.get("test1");

        // Add test3 - test2 should be evicted (not test1)
        cache.put("test3".to_string(), vec![3.0]);

        assert!(cache.get("test1").is_some());
        assert!(cache.get("test2").is_none());
        assert!(cache.get("test3").is_some());
    }

    #[test]
    fn inference_cache_works() {
        let mut cache: InferenceCache<String> = InferenceCache::new(10);

        cache.put("query".to_string(), "result".to_string());
        let retrieved = cache.get("query");

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), "result");
    }

    #[test]
    fn zero_capacity_cache_disabled() {
        let mut cache = EmbeddingCache::new(0);
        cache.put("test".to_string(), vec![1.0]);
        assert!(cache.get("test").is_none());
    }
}
