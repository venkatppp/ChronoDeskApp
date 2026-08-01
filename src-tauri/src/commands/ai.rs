//! AI model management commands.

use tauri::{Emitter, State};

use crate::ai::models::{RerankRequest, RerankResult};
use crate::ai::{AIDiagnostics, DownloadProgress, InferenceStats, ModelInfo};

/// AI state manager (to be added to lib.rs).
pub struct AIState {
    pub manager: crate::ai::ModelManager,
    pub reranker: Option<std::sync::Arc<crate::ai::Reranker>>,
    pub embedding_provider: Option<std::sync::Arc<crate::ai::ONNXEmbeddingProvider>>,
}

/// Lists all available AI models.
#[tauri::command]
pub async fn list_models(state: State<'_, AIState>) -> Result<Vec<ModelInfo>, String> {
    Ok(state.manager.list_models())
}

/// Gets information about a specific model.
#[tauri::command]
pub async fn get_model(
    model_id: String,
    state: State<'_, AIState>,
) -> Result<Option<ModelInfo>, String> {
    Ok(state.manager.get_model(&model_id))
}

/// Downloads a model.
#[tauri::command]
pub async fn download_model(
    model_id: String,
    state: State<'_, AIState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    state
        .manager
        .download_model(&model_id, move |progress: DownloadProgress| {
            let _ = app.emit("model:download_progress", &progress);
        })
        .await
        .map_err(|e| e.to_string())
}

/// Loads a model into memory.
#[tauri::command]
pub async fn load_model(model_id: String, state: State<'_, AIState>) -> Result<(), String> {
    // Get model metadata
    let model = state
        .manager
        .get_model(&model_id)
        .ok_or_else(|| format!("Model not found: {}", model_id))?;

    if model.status != crate::ai::models::ModelStatus::Downloaded {
        return Err("Model must be downloaded first".to_string());
    }

    let model_path = state
        .manager
        .get_model_path(&model_id)
        .ok_or_else(|| "Model path not found".to_string())?;

    let model_file = model_path.join("model.onnx");
    let tokenizer_file = model_path.join("tokenizer.json");

    // Load based on model type
    match model.metadata.model_type {
        crate::ai::models::ModelType::Embedding => {
            let _provider = crate::ai::ONNXEmbeddingProvider::new(
                model_id.clone(),
                model_file,
                tokenizer_file,
                model.metadata.dimensions,
                model.metadata.max_sequence_length,
                true,  // enable_cache
                10000, // cache_size
            )
            .map_err(|e| e.to_string())?;

            // TODO: Store provider in state
            // For now, mark as loaded
            state
                .manager
                .mark_loaded(&model_id, model.metadata.file_size_bytes)
                .map_err(|e| e.to_string())?;

            state
                .manager
                .set_active_embedding_model(model_id.clone())
                .map_err(|e| e.to_string())?;
        }
        crate::ai::models::ModelType::Reranker => {
            let _reranker = crate::ai::Reranker::new(
                model_id.clone(),
                model_file,
                tokenizer_file,
                model.metadata.max_sequence_length,
                true, // enable_cache
                1000, // cache_size
            )
            .map_err(|e| e.to_string())?;

            state
                .manager
                .mark_loaded(&model_id, model.metadata.file_size_bytes)
                .map_err(|e| e.to_string())?;

            state
                .manager
                .set_active_reranker_model(model_id.clone())
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Unloads a model from memory.
#[tauri::command]
pub async fn unload_model(model_id: String, state: State<'_, AIState>) -> Result<(), String> {
    state
        .manager
        .mark_unloaded(&model_id)
        .map_err(|e| e.to_string())
}

/// Gets the active embedding model ID.
#[tauri::command]
pub async fn get_active_embedding_model(
    state: State<'_, AIState>,
) -> Result<Option<String>, String> {
    Ok(state.manager.active_embedding_model())
}

/// Gets the active reranker model ID.
#[tauri::command]
pub async fn get_active_reranker_model(
    state: State<'_, AIState>,
) -> Result<Option<String>, String> {
    Ok(state.manager.active_reranker_model())
}

/// Gets model status.
#[tauri::command]
pub async fn get_model_status(
    model_id: String,
    state: State<'_, AIState>,
) -> Result<Option<crate::ai::models::ModelStatus>, String> {
    Ok(state.manager.get_model(&model_id).map(|m| m.status))
}

/// Gets inference statistics for all models.
#[tauri::command]
pub async fn get_inference_statistics(
    state: State<'_, AIState>,
) -> Result<Vec<InferenceStats>, String> {
    let mut stats = Vec::new();

    // Get embedding provider stats
    if let Some(provider) = &state.embedding_provider {
        stats.push(provider.get_stats());
    }

    // Get reranker stats
    if let Some(reranker) = &state.reranker {
        stats.push(reranker.get_stats());
    }

    Ok(stats)
}

/// Gets AI diagnostics.
#[tauri::command]
pub async fn get_ai_diagnostics(state: State<'_, AIState>) -> Result<AIDiagnostics, String> {
    let models = state.manager.list_models();
    let stats_vec = get_inference_statistics(state).await?;

    let mut stats_map = std::collections::HashMap::new();
    for stat in stats_vec {
        stats_map.insert(stat.model_id.clone(), stat);
    }

    Ok(AIDiagnostics::new(models, stats_map))
}

/// Reranks documents using the active reranker model.
#[tauri::command]
pub async fn rerank_documents(
    request: RerankRequest,
    state: State<'_, AIState>,
) -> Result<Vec<RerankResult>, String> {
    let reranker = state
        .reranker
        .as_ref()
        .ok_or_else(|| "No reranker model loaded".to_string())?;

    reranker.rerank(request).await.map_err(|e| e.to_string())
}
