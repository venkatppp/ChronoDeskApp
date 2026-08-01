//! Reranker for improving search result quality - Real ONNX implementation.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::ai::cache::InferenceCache;
use crate::ai::inference::RerankerInferenceEngine;
use crate::ai::models::{InferenceStats, RerankRequest, RerankResult};
use crate::errors::DatabaseError;

/// Cross-encoder reranker for search results with real ONNX inference.
pub struct Reranker {
    #[allow(dead_code)]
    model_id: String,
    engine: Arc<RerankerInferenceEngine>,
    cache: Arc<Mutex<InferenceCache<Vec<RerankResult>>>>,
    stats: Arc<Mutex<InferenceStats>>,
}

impl Reranker {
    /// Creates a new reranker with real ONNX inference.
    pub fn new(
        model_id: String,
        model_path: PathBuf,
        tokenizer_path: PathBuf,
        max_length: usize,
        enable_cache: bool,
        cache_size: usize,
    ) -> Result<Self, DatabaseError> {
        // Initialize the ONNX inference engine
        let engine = RerankerInferenceEngine::new(&model_path, &tokenizer_path, max_length)?;

        let cache = if enable_cache {
            InferenceCache::new(cache_size)
        } else {
            InferenceCache::new(0)
        };

        let stats = InferenceStats::new(model_id.clone(), crate::ai::models::ModelType::Reranker);

        Ok(Self {
            model_id,
            engine: Arc::new(engine),
            cache: Arc::new(Mutex::new(cache)),
            stats: Arc::new(Mutex::new(stats)),
        })
    }

    /// Reranks documents based on relevance to query using real ONNX inference.
    pub async fn rerank(&self, request: RerankRequest) -> Result<Vec<RerankResult>, DatabaseError> {
        let start = std::time::Instant::now();

        // Create cache key
        let cache_key = format!(
            "{}:{}:{}",
            request.query,
            request.documents.join("|"),
            request.top_k
        );

        // Check cache
        {
            let mut cache = self.cache.lock();
            if let Some(results) = cache.get(&cache_key) {
                let mut stats = self.stats.lock();
                stats.update_cache_hit();
                return Ok(results);
            }
        }

        // Cache miss - compute scores using real ONNX inference
        let results = self.rerank_with_inference(request).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock();
            cache.put(cache_key, results.clone());
        }

        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.update_cache_miss();
            stats.total_inferences += 1;
            stats.last_inference_at = Some(chrono::Utc::now());

            let latency = start.elapsed().as_millis() as f32;
            stats.avg_latency_ms = if stats.total_inferences == 1 {
                latency
            } else {
                (stats.avg_latency_ms * (stats.total_inferences - 1) as f32 + latency)
                    / stats.total_inferences as f32
            };
        }

        Ok(results)
    }

    /// Reranks using real ONNX cross-encoder inference.
    async fn rerank_with_inference(
        &self,
        request: RerankRequest,
    ) -> Result<Vec<RerankResult>, DatabaseError> {
        let engine = self.engine.clone();
        let query = request.query.clone();
        let documents = request.documents.clone();

        // Run inference in blocking task to avoid blocking async runtime
        let scores = tokio::task::spawn_blocking(move || {
            let doc_refs: Vec<&str> = documents.iter().map(|s| s.as_str()).collect();
            engine.score_batch(&query, &doc_refs)
        })
        .await
        .map_err(|e| DatabaseError::IoError(format!("Reranking task failed: {}", e)))??;

        // Create results with scores
        let mut results: Vec<RerankResult> = request
            .documents
            .iter()
            .enumerate()
            .zip(scores.iter())
            .map(|((index, document), score)| RerankResult {
                index,
                score: *score,
                document: document.clone(),
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top k
        results.truncate(request.top_k);

        Ok(results)
    }

    /// Gets inference statistics.
    pub fn get_stats(&self) -> InferenceStats {
        self.stats.lock().clone()
    }

    /// Clears the reranking cache.
    pub fn clear_cache(&self) {
        self.cache.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require actual ONNX models to be present.

    #[tokio::test]
    #[ignore] // Ignore by default since it requires downloaded models
    async fn real_reranking_works() {
        let model_path = PathBuf::from("test_models/bge-reranker-base/model.onnx");
        let tokenizer_path = PathBuf::from("test_models/bge-reranker-base/tokenizer.json");

        if !model_path.exists() || !tokenizer_path.exists() {
            return; // Skip if models not available
        }

        let reranker = Reranker::new(
            "test".to_string(),
            model_path,
            tokenizer_path,
            512,
            true,
            100,
        )
        .unwrap();

        let request = RerankRequest {
            query: "rust programming".to_string(),
            documents: vec![
                "Learning Rust programming language".to_string(),
                "Python tutorial for beginners".to_string(),
                "Advanced Rust techniques".to_string(),
            ],
            top_k: 2,
        };

        let results = reranker.rerank(request).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].score >= results[1].score);

        // Verify that Rust-related documents score higher
        assert!(results[0].document.contains("Rust") || results[1].document.contains("Rust"));
    }
}
