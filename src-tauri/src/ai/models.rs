//! AI data models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Type of AI model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    Embedding,
    Reranker,
}

impl ModelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::Reranker => "reranker",
        }
    }
}

/// Status of a model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Loading,
    Loaded,
    Unloading,
    Error,
}

/// Metadata about an AI model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelMetadata {
    pub id: String,
    pub name: String,
    pub model_type: ModelType,
    pub version: String,
    pub dimensions: usize,
    pub max_sequence_length: usize,
    pub file_size_bytes: u64,
    pub download_url: String,
    pub tokenizer_url: Option<String>,
    pub description: String,
}

/// Information about a model including its status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub metadata: ModelMetadata,
    pub status: ModelStatus,
    pub local_path: Option<PathBuf>,
    pub downloaded_at: Option<DateTime<Utc>>,
    pub loaded_at: Option<DateTime<Utc>>,
    pub memory_usage_bytes: Option<u64>,
    pub error_message: Option<String>,
}

/// Progress of a model download.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    pub speed_bytes_per_sec: u64,
}

/// Statistics about inference performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceStats {
    pub model_id: String,
    pub model_type: ModelType,
    pub total_inferences: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f32,
    pub avg_latency_ms: f32,
    pub p50_latency_ms: f32,
    pub p95_latency_ms: f32,
    pub p99_latency_ms: f32,
    pub last_inference_at: Option<DateTime<Utc>>,
}

impl InferenceStats {
    pub fn new(model_id: String, model_type: ModelType) -> Self {
        Self {
            model_id,
            model_type,
            total_inferences: 0,
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
            avg_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            last_inference_at: None,
        }
    }

    pub fn update_cache_hit(&mut self) {
        self.cache_hits += 1;
        self.update_hit_rate();
    }

    pub fn update_cache_miss(&mut self) {
        self.cache_misses += 1;
        self.update_hit_rate();
    }

    fn update_hit_rate(&mut self) {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hit_rate = self.cache_hits as f32 / total as f32;
        }
    }
}

/// Reranking request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<String>,
    pub top_k: usize,
}

/// Reranking result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankResult {
    pub index: usize,
    pub score: f32,
    pub document: String,
}
