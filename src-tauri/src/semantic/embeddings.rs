//! Embedding provider abstraction.
//!
//! Supports multiple embedding backends (local, ONNX, remote APIs).

use async_trait::async_trait;

use crate::errors::DatabaseError;

/// Trait for embedding providers.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generates an embedding vector for the given text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError>;

    /// Returns the dimensionality of embeddings produced by this provider.
    fn dimensions(&self) -> usize;

    /// Returns the name of this provider.
    fn name(&self) -> &str;
}

/// Local placeholder embedding provider.
///
/// Uses a simple hash-based approach for development and testing.
/// Replace with ONNX or remote provider for production use.
#[derive(Clone)]
pub struct LocalEmbeddingProvider {
    dimensions: usize,
}

impl LocalEmbeddingProvider {
    /// Creates a new local embedding provider with specified dimensions.
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl Default for LocalEmbeddingProvider {
    fn default() -> Self {
        Self::new(384) // Default to 384 dimensions (common for MiniLM)
    }
}

#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, DatabaseError> {
        // Simple deterministic embedding based on text hash
        // This is a placeholder - replace with real embeddings in production
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate deterministic pseudo-random embedding
        let mut embedding = Vec::with_capacity(self.dimensions);
        let mut seed = hash;

        for _ in 0..self.dimensions {
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let value = ((seed / 65536) % 32768) as f32 / 32768.0;
            embedding.push(value);
        }

        // Normalize the embedding
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        Ok(embedding)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn name(&self) -> &str {
        "local-hash"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_provider_generates_correct_dimensions() {
        let provider = LocalEmbeddingProvider::new(384);
        let embedding = provider.embed("test").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn local_provider_is_deterministic() {
        let provider = LocalEmbeddingProvider::new(384);
        let embedding1 = provider.embed("test").await.unwrap();
        let embedding2 = provider.embed("test").await.unwrap();
        assert_eq!(embedding1, embedding2);
    }

    #[tokio::test]
    async fn local_provider_normalizes_embeddings() {
        let provider = LocalEmbeddingProvider::new(384);
        let embedding = provider.embed("test").await.unwrap();

        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn different_text_produces_different_embeddings() {
        let provider = LocalEmbeddingProvider::new(384);
        let embedding1 = provider.embed("hello").await.unwrap();
        let embedding2 = provider.embed("world").await.unwrap();
        assert_ne!(embedding1, embedding2);
    }
}
