//! AI settings and configuration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// AI settings for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AISettings {
    /// Directory where models are stored.
    pub models_dir: PathBuf,

    /// Active embedding model ID.
    pub active_embedding_model: Option<String>,

    /// Active reranker model ID.
    pub active_reranker_model: Option<String>,

    /// Maximum number of models to keep loaded in memory.
    pub max_loaded_models: usize,

    /// Enable embedding cache.
    pub enable_embedding_cache: bool,

    /// Maximum embedding cache size in entries.
    pub embedding_cache_size: usize,

    /// Enable inference cache.
    pub enable_inference_cache: bool,

    /// Maximum inference cache size in entries.
    pub inference_cache_size: usize,

    /// Number of background embedding workers.
    pub embedding_workers: usize,

    /// Batch size for background embedding generation.
    pub embedding_batch_size: usize,

    /// Enable automatic model updates.
    pub auto_update_models: bool,

    /// Persist loaded models across restarts.
    pub persist_loaded_models: bool,

    /// Loaded model IDs (persisted state).
    #[serde(default)]
    pub loaded_model_ids: Vec<String>,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            models_dir: PathBuf::from("models"),
            active_embedding_model: None,
            active_reranker_model: None,
            max_loaded_models: 2,
            enable_embedding_cache: true,
            embedding_cache_size: 10000,
            enable_inference_cache: true,
            inference_cache_size: 1000,
            embedding_workers: 2,
            embedding_batch_size: 32,
            auto_update_models: false,
            persist_loaded_models: true,
            loaded_model_ids: Vec::new(),
        }
    }
}

impl AISettings {
    /// Creates settings with a custom models directory.
    pub fn with_models_dir(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            ..Default::default()
        }
    }

    /// Sets the active embedding model.
    pub fn with_embedding_model(mut self, model_id: String) -> Self {
        self.active_embedding_model = Some(model_id);
        self
    }

    /// Sets the active reranker model.
    pub fn with_reranker_model(mut self, model_id: String) -> Self {
        self.active_reranker_model = Some(model_id);
        self
    }

    /// Saves settings to a file.
    pub async fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, json).await
    }

    /// Loads settings from a file.
    pub async fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = tokio::fs::read_to_string(path).await?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
