//! Local Vector Provider - a real, deterministic, dependency-free
//! embedding provider for the execution memory system.
//!
//! Replaces the placeholder whole-string hash provider with a
//! character n-gram hashing embedder (the "hashing trick" used by
//! FastText-style models): a text becomes a sparse bag of its words and
//! character n-grams, each hashed into a fixed-dimension vector with a
//! signed bucket. The vector is term-frequency weighted and L2-normalized.
//!
//! Why this is a *real* embedding: texts that share words or subword
//! n-grams land close together in the vector space, so cosine similarity
//! is meaningful for goal matching (e.g. "resume my focus session" and
//! "resume my last focus session" score high, "organize receipts" does
//! not) — the property the whole-string hash provider lacked.
//!
//! Deterministic by construction: accumulation is order-independent and
//! `DefaultHasher` uses fixed keys, so the same text embeds identically
//! on every process run (safe for the persistent SQLite embedding cache).

use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::copilot::memory::vector::provider::VectorProvider;
use crate::errors::DatabaseError;

/// Lower bound of the character n-grams extracted from each word.
const MIN_NGRAM: usize = 3;
/// Upper bound of the character n-grams extracted from each word.
const MAX_NGRAM: usize = 5;

/// Local n-gram hashing embedding provider.
#[derive(Clone)]
pub struct LocalVectorProvider {
    dimensions: usize,
}

impl LocalVectorProvider {
    /// Creates a provider with the given embedding dimensionality.
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    /// Embeds a text synchronously (the shared core for `embed` and
    /// `embed_batch`).
    fn embed_sync(&self, text: &str) -> Vec<f32> {
        let dims = self.dimensions;
        if dims == 0 {
            return Vec::new();
        }

        // Word tokens: lowercase alphanumeric sequences. Order is kept so
        // the feature stream is reproducible, and counts accumulate into a
        // map whose iteration order does not matter (sums commute).
        let mut tokens: Vec<String> = Vec::new();
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            let word = word.to_lowercase();
            if !word.is_empty() {
                tokens.push(word);
            }
        }

        let mut counts: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for word in &tokens {
            *counts.entry(word.clone()).or_insert(0.0) += 1.0;
            if word.len() > MIN_NGRAM {
                for size in MIN_NGRAM..=MAX_NGRAM.min(word.len()) {
                    for start in 0..=(word.len() - size) {
                        let ngram = word[start..start + size].to_string();
                        *counts.entry(ngram).or_insert(0.0) += 1.0;
                    }
                }
            }
        }

        let mut accum = vec![0.0_f64; dims];
        for (feature, count) in &counts {
            let h1 = hash_seeded(feature, 0);
            let h2 = hash_seeded(feature, 1);
            let bucket = (h1 % dims as u64) as usize;
            let sign = if h2 & 1 == 0 { 1.0 } else { -1.0 };
            // Term-frequency weight; `1 + tf` keeps single occurrences
            // non-trivial while repeated terms dominate.
            let weight = 1.0 + count.sqrt();
            accum[bucket] += sign * weight;
        }

        // L2 normalize so cosine similarity is a direct dot product.
        let magnitude: f64 = accum.iter().map(|v| v * v).sum::<f64>().sqrt();
        if magnitude == 0.0 {
            return vec![0.0; dims];
        }
        accum.into_iter().map(|v| (v / magnitude) as f32).collect()
    }
}

impl Default for LocalVectorProvider {
    fn default() -> Self {
        Self::new(384) // MiniLM-style default dimensionality
    }
}

/// Hashes a feature with a seed, using fixed-key `DefaultHasher` so the
/// result is stable across processes.
fn hash_seeded(feature: &str, seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    feature.hash(&mut hasher);
    hasher.finish()
}

#[async_trait]
impl VectorProvider for LocalVectorProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError> {
        Ok(self.embed_sync(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, DatabaseError> {
        Ok(texts.iter().map(|text| self.embed_sync(text)).collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn name(&self) -> &str {
        "local-ngram"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cosine(a: &[f32], b: &[f32]) -> f64 {
        let dot: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (*x as f64) * (*y as f64))
            .sum();
        let mag_a: f64 = a
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        let mag_b: f64 = b
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }

    #[tokio::test]
    async fn generates_correct_dimensions() {
        let provider = LocalVectorProvider::new(384);
        let embedding = provider.embed("test").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn is_deterministic_across_runs() {
        let provider = LocalVectorProvider::default();
        let a = provider.embed("resume my focus session").await.unwrap();
        let b = provider.embed("resume my focus session").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn embeddings_are_l2_normalized() {
        let provider = LocalVectorProvider::default();
        let embedding = provider.embed("resume my focus session").await.unwrap();
        let magnitude: f64 = embedding
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        assert!((magnitude - 1.0).abs() < 1e-4);
    }

    #[tokio::test]
    async fn similar_goals_embed_close_together() {
        let provider = LocalVectorProvider::default();
        let a = provider.embed("resume my focus session").await.unwrap();
        let b = provider
            .embed("resume my last focus session")
            .await
            .unwrap();
        let c = provider.embed("organize tax receipts").await.unwrap();
        let d = provider.embed("resume my focus session").await.unwrap();

        let near = cosine(&a, &b);
        let far = cosine(&a, &c);
        let same = cosine(&a, &d);
        assert!(near > 0.5, "shared n-grams should embed close: {near}");
        assert!(far < 0.5, "unrelated goals should embed apart: {far}");
        assert!((same - 1.0).abs() < 1e-4, "identical text scores 1.0");
        assert!(near > far, "similar must beat unrelated: {near} vs {far}");
    }

    #[tokio::test]
    async fn batch_matches_single_embeddings() {
        let provider = LocalVectorProvider::default();
        let texts = vec!["resume focus", "organize receipts", "plan vacation"];
        let batch = provider.embed_batch(&texts).await.unwrap();
        for (text, embedded) in texts.iter().zip(batch.iter()) {
            let single = provider.embed(text).await.unwrap();
            assert_eq!(embedded, &single);
        }
    }

    #[tokio::test]
    async fn empty_text_embeds_to_zero_vector() {
        let provider = LocalVectorProvider::default();
        let embedding = provider.embed("   ").await.unwrap();
        assert!(embedding.iter().all(|v| *v == 0.0));
    }
}
