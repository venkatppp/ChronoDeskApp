//! Vector Index - in-memory k-NN index over execution memory
//! embeddings.
//!
//! Holds one L2-normalized embedding per memory record and answers
//! approximate-NN queries by exact dot product (cosine) over the resident
//! set. Memory sizes are small (tens to low thousands of records), so a
//! flat scan is both fast and exact; the index's real job is keeping
//! similarity search *off* the SQLite row-decode path (records are only
//! loaded from SQL once the top-k candidate ids are known).
//!
//! The index is durable through `MemoryVectorRepository`: the background
//! indexer persists every mutation here and rebuilds it at startup via
//! `load_vectors` (see `MemoryIndexer::warm_up`).

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use uuid::Uuid;

/// In-memory k-NN index over memory embeddings. Cheap to clone; state
/// lives behind an arc-shared read-write lock.
#[derive(Debug, Default)]
pub struct VectorIndex {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    /// memory_id -> (L2-normalized embedding, text that was embedded).
    vectors: HashMap<Uuid, (Vec<f32>, String)>,
}

impl Clone for VectorIndex {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl VectorIndex {
    /// Creates an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Upserts a memory record's embedding (normalized on insert).
    pub fn upsert(&self, memory_id: Uuid, text: &str, embedding: Vec<f32>) {
        let normalized = normalize(&embedding);
        self.inner
            .write()
            .vectors
            .insert(memory_id, (normalized, text.to_string()));
    }

    /// Removes a memory record from the index.
    pub fn remove(&self, memory_id: Uuid) -> bool {
        self.inner.write().vectors.remove(&memory_id).is_some()
    }

    /// Removes every entry.
    pub fn clear(&self) {
        self.inner.write().vectors.clear();
    }

    /// Number of indexed records.
    pub fn len(&self) -> usize {
        self.inner.read().vectors.len()
    }

    /// Whether the index holds no records.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether a memory record is currently indexed.
    pub fn contains(&self, memory_id: Uuid) -> bool {
        self.inner.read().vectors.contains_key(&memory_id)
    }

    /// The k nearest neighbors of the query embedding, as `(memory_id,
    /// cosine)` pairs ordered by descending similarity. The query vector
    /// is normalized on the way in, so the score is a true cosine in
    /// [0, 1] for non-negative-mean inputs (zero-centered cosine remains
    /// the caller's `retrieval::cosine_similarity`).
    pub fn knn(&self, query: &[f32], k: usize) -> Vec<(Uuid, f32)> {
        let inner = self.inner.read();
        if inner.vectors.is_empty() || query.is_empty() {
            return Vec::new();
        }
        let normalized_query = normalize(query);
        let mut scored: Vec<(Uuid, f32)> = inner
            .vectors
            .iter()
            .map(|(id, (embedding, _))| {
                let cosine = dot(&normalized_query, embedding);
                (*id, cosine)
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored.truncate(k);
        scored
    }
}

/// L2-normalizes a vector in place of magnitude 0 (returns a zero vector).
fn normalize(vector: &[f32]) -> Vec<f32> {
    let magnitude: f64 = vector
        .iter()
        .map(|v| (*v as f64) * (*v as f64))
        .sum::<f64>()
        .sqrt();
    if magnitude == 0.0 {
        return vec![0.0; vector.len()];
    }
    vector.iter().map(|v| *v / magnitude as f32).collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_and_knn_rank_by_cosine() {
        let index = VectorIndex::new();
        let near = Uuid::new_v4();
        let far = Uuid::new_v4();
        // Unit-ish vector pointing mostly in the query direction.
        index.upsert(near, "resume focus", vec![1.0, 1.0, 0.0]);
        index.upsert(far, "tax receipts", vec![-1.0, 1.0, 0.0]);

        let hits = index.knn(&[1.0, 1.0, 0.0], 2);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, near);
        assert!(hits[0].1 > hits[1].1);
    }

    #[test]
    fn identical_vectors_score_one() {
        let index = VectorIndex::new();
        let id = Uuid::new_v4();
        index.upsert(id, "resume focus", vec![0.5, 0.5, 0.5]);
        let hits = index.knn(&[0.5, 0.5, 0.5], 1);
        assert!((hits[0].1 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn knn_respects_k_and_empty_state() {
        let index = VectorIndex::new();
        assert!(index.is_empty());
        assert!(index.knn(&[1.0, 0.0], 5).is_empty());

        for i in 0..5 {
            index.upsert(Uuid::new_v4(), &format!("g{i}"), vec![i as f32, 0.0]);
        }
        assert_eq!(index.len(), 5);
        assert_eq!(index.knn(&[10.0, 0.0], 3).len(), 3);
        assert_eq!(index.knn(&[], 3).len(), 0);
    }

    #[test]
    fn remove_and_clear_keep_index_consistent() {
        let index = VectorIndex::new();
        let id = Uuid::new_v4();
        index.upsert(id, "g", vec![1.0, 0.0]);
        assert!(index.contains(id));
        assert!(index.remove(id));
        assert!(!index.contains(id));
        index.upsert(id, "g", vec![1.0, 0.0]);
        index.clear();
        assert!(index.is_empty());
    }

    #[test]
    fn upsert_overwrites_existing_entry() {
        let index = VectorIndex::new();
        let id = Uuid::new_v4();
        index.upsert(id, "g", vec![1.0, 0.0]);
        index.upsert(id, "g", vec![0.0, 1.0]);
        assert_eq!(index.len(), 1, "no duplicate entries");
        let hits = index.knn(&[0.0, 1.0], 1);
        assert_eq!(hits[0].0, id);
        assert!((hits[0].1 - 1.0).abs() < 1e-4);
    }
}
