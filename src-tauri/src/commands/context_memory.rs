//! Context Memory and Knowledge commands.

use crate::context_memory::{
    ContextMemoryEngine, CreateSnapshotRequest, KnowledgeQuery, KnowledgeSearchResult,
    RelatedWorkspace,
};
use crate::runtime::IntelligenceEmitter;
use tauri::State;
use uuid::Uuid;

/// Create a context snapshot.
#[tauri::command]
pub async fn create_context_snapshot(
    request: CreateSnapshotRequest,
    engine: State<'_, ContextMemoryEngine>,
    emitter: State<'_, IntelligenceEmitter>,
) -> Result<crate::context_memory::ContextSnapshot, String> {
    let snapshot = engine
        .create_snapshot(request.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Emit snapshot created event
    if let Ok(workspace_id) = Uuid::parse_str(&request.workspace_id) {
        emitter.emit_snapshot_created(workspace_id, snapshot.id);
    }

    Ok(snapshot)
}

/// Get context snapshots for a workspace.
#[tauri::command]
pub async fn get_workspace_snapshots(
    workspace_id: String,
    limit: Option<usize>,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<Vec<crate::context_memory::ContextSnapshot>, String> {
    engine
        .get_workspace_snapshots(&workspace_id, limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}

/// Get the latest context snapshot for a workspace.
#[tauri::command]
pub async fn get_latest_snapshot(
    workspace_id: String,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<Option<crate::context_memory::ContextSnapshot>, String> {
    engine
        .get_latest_snapshot(&workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Detect and store workspace relationships.
#[tauri::command]
pub async fn detect_workspace_relationships(
    workspace_id: String,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<(), String> {
    engine
        .detect_workspace_relationships(&workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get related workspaces.
#[tauri::command]
pub async fn get_related_workspaces(
    workspace_id: String,
    min_strength: Option<f64>,
    limit: Option<usize>,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<Vec<RelatedWorkspace>, String> {
    engine
        .get_related_workspaces(
            &workspace_id,
            min_strength.unwrap_or(0.1),
            limit.unwrap_or(10),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Search knowledge base.
#[tauri::command]
pub async fn search_knowledge(
    query: KnowledgeQuery,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<KnowledgeSearchResult, String> {
    engine
        .search_knowledge(query)
        .await
        .map_err(|e| e.to_string())
}

/// Create a milestone snapshot.
#[tauri::command]
pub async fn snapshot_milestone(
    workspace_id: String,
    active_files: Vec<String>,
    metadata: serde_json::Value,
    engine: State<'_, ContextMemoryEngine>,
) -> Result<crate::context_memory::ContextSnapshot, String> {
    engine
        .snapshot_milestone(&workspace_id, active_files, metadata)
        .await
        .map_err(|e| e.to_string())
}
