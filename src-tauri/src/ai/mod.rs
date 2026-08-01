//! AI Module - Local ONNX-based Intelligence
//!
//! Provides model management, embeddings, reranking, and inference caching.

pub mod cache;
pub mod diagnostics;
pub mod manager;
pub mod models;
pub mod onnx_provider;
pub mod reranker;
pub mod settings;
pub mod workers;

pub use cache::{EmbeddingCache, InferenceCache};
pub use diagnostics::AIDiagnostics;
pub use manager::ModelManager;
pub use models::{
    DownloadProgress, InferenceStats, ModelInfo, ModelMetadata, ModelStatus, ModelType,
};
pub use onnx_provider::ONNXEmbeddingProvider;
pub use reranker::Reranker;
pub use settings::AISettings;
pub use workers::EmbeddingWorker;
