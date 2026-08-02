//! Embedding Cache - in-memory LRU cache for text embeddings plus
//! hit/miss counters for the dashboard.
//!
//! The cache is keyed by the stable hash of the text (see
//! `models::text_hash`) so it stays consistent with the persistent
//! SQLite cache. It is pure in-memory state guarded by a `parking_lot`
//! mutex; persistence belongs to `MemoryVectorRepository` and composition
//! to `CachedProvider`.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

/// LRU cache of text -> embedding vectors.
#[derive(Debug)]
pub struct EmbeddingCache {
    capacity: usize,
    entries: Mutex<Entries>,
    hits: AtomicU64,
    misses: AtomicU64,
}

#[derive(Debug, Default)]
struct Entries {
    /// hash -> embedding.
    map: HashMap<u64, Vec<f32>>,
    /// LRU order: the front is the least recently used entry.
    order: VecDeque<u64>,
}

/// Snapshot of cache statistics for the dashboard.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    /// Hits / (hits + misses); 1.0 when there is no activity yet.
    pub hit_rate: f64,
}

impl EmbeddingCache {
    /// Creates a cache with the given capacity (0 disables the cache).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Mutex::new(Entries::default()),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Looks up an embedding, recording a hit or a miss and refreshing
    /// the entry's recency.
    pub fn get(&self, text: &str) -> Option<Vec<f32>> {
        if self.capacity == 0 {
            self.misses.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let hash = hash_text(text);
        let mut entries = self.entries.lock();
        match entries.map.get(&hash).cloned() {
            Some(embedding) => {
                entries.touch(hash);
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(embedding)
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Stores an embedding, evicting the least recently used entry when at
    /// capacity.
    pub fn put(&self, text: String, embedding: Vec<f32>) {
        if self.capacity == 0 {
            return;
        }
        let hash = hash_text(&text);
        let mut entries = self.entries.lock();
        if !entries.map.contains_key(&hash) {
            if entries.map.len() >= self.capacity {
                entries.evict_lru();
            }
            entries.map.insert(hash, embedding);
        } else if let Some(entry) = entries.map.get_mut(&hash) {
            *entry = embedding;
        }
        entries.touch(hash);
    }

    /// Removes every entry and resets the counters.
    pub fn clear(&self) {
        self.entries.lock().clear_all();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Current statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        CacheStats {
            size: self.entries.lock().map.len(),
            capacity: self.capacity,
            hits,
            misses,
            hit_rate: if total == 0 {
                1.0
            } else {
                hits as f64 / total as f64
            },
        }
    }
}

impl Entries {
    /// Marks an entry as most recently used.
    fn touch(&mut self, hash: u64) {
        self.order.retain(|&h| h != hash);
        self.order.push_back(hash);
    }

    /// Removes the least recently used entry.
    fn evict_lru(&mut self) {
        if let Some(lru_hash) = self.order.pop_front() {
            self.map.remove(&lru_hash);
        }
    }

    fn clear_all(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

/// Stable hash used as the in-memory cache key (the `u64` behind
/// `models::text_hash`, which renders it as a string for SQLite).
fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_retrieves() {
        let cache = EmbeddingCache::new(10);
        cache.put("resume focus".to_string(), vec![1.0, 2.0]);
        assert_eq!(cache.get("resume focus"), Some(vec![1.0, 2.0]));
    }

    #[test]
    fn evicts_least_recently_used() {
        let cache = EmbeddingCache::new(2);
        cache.put("a".to_string(), vec![1.0]);
        cache.put("b".to_string(), vec![2.0]);
        let _ = cache.get("a"); // a becomes most recent
        cache.put("c".to_string(), vec![3.0]); // evicts b
        assert!(cache.get("a").is_some());
        assert!(cache.get("b").is_none());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn tracks_hits_misses_and_hit_rate() {
        let cache = EmbeddingCache::new(10);
        cache.put("a".to_string(), vec![1.0]);
        let _ = cache.get("a");
        let _ = cache.get("a");
        let _ = cache.get("missing");
        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn zero_capacity_disables_cache() {
        let cache = EmbeddingCache::new(0);
        cache.put("a".to_string(), vec![1.0]);
        assert!(cache.get("a").is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn clear_resets_entries_and_counters() {
        let cache = EmbeddingCache::new(4);
        cache.put("a".to_string(), vec![1.0]);
        let _ = cache.get("a");
        cache.clear();
        let stats = cache.stats();
        assert_eq!(stats.size, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        // After the reset, a lookup is a fresh miss.
        assert!(cache.get("a").is_none());
        assert_eq!(cache.stats().misses, 1);
    }
}
