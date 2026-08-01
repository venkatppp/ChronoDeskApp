//! Semantic search commands for frontend integration.

use tauri::State;

use crate::semantic::models::ExplainablePrediction;
use crate::semantic::models::{SemanticSearchRequest, SemanticSearchResult};
use crate::semantic::reasoning::ContextReasoningEngine;
use crate::semantic::search::SemanticSearchEngine;

/// Performs semantic search across all indexed documents.
#[tauri::command]
pub async fn semantic_search(
    request: SemanticSearchRequest,
    engine: State<'_, SemanticSearchEngine>,
) -> Result<Vec<SemanticSearchResult>, String> {
    engine.search(request).await.map_err(|e| e.to_string())
}

/// Finds similar documents to a given document.
#[tauri::command]
pub async fn find_similar_documents(
    document_id: String,
    limit: usize,
    min_confidence: f32,
    engine: State<'_, SemanticSearchEngine>,
) -> Result<Vec<SemanticSearchResult>, String> {
    engine
        .find_similar(&document_id, limit, min_confidence)
        .await
        .map_err(|e| e.to_string())
}

/// Infers related work for a workspace.
#[tauri::command]
pub async fn infer_related_work(
    workspace_id: String,
    engine: State<'_, ContextReasoningEngine>,
) -> Result<Vec<String>, String> {
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace ID: {}", e))?;

    engine
        .infer_related_work(workspace_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Detects recurring workflows in a workspace.
#[tauri::command]
pub async fn detect_recurring_workflows(
    workspace_id: String,
    engine: State<'_, ContextReasoningEngine>,
) -> Result<Vec<String>, String> {
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace ID: {}", e))?;

    engine
        .detect_recurring_workflows(workspace_uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Finds similar sessions based on semantic content.
#[tauri::command]
pub async fn find_similar_sessions(
    workspace_id: String,
    limit: usize,
    engine: State<'_, ContextReasoningEngine>,
) -> Result<Vec<String>, String> {
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace ID: {}", e))?;

    engine
        .find_similar_sessions(workspace_uuid, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Explains a recommendation with supporting evidence.
#[tauri::command]
pub async fn explain_recommendation(
    workspace_id: String,
    recommendation_id: String,
    engine: State<'_, ContextReasoningEngine>,
) -> Result<ExplainablePrediction, String> {
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace ID: {}", e))?;

    engine
        .explain_recommendation(workspace_uuid, recommendation_id)
        .await
        .map_err(|e| e.to_string())
}

/// Infers missing context for a workspace.
#[tauri::command]
pub async fn infer_missing_context(
    workspace_id: String,
    engine: State<'_, ContextReasoningEngine>,
) -> Result<Vec<String>, String> {
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace ID: {}", e))?;

    engine
        .infer_missing_context(workspace_uuid)
        .await
        .map_err(|e| e.to_string())
}
