//! Session IPC commands.
//!
//! Thin handlers that expose ContextService to the frontend. All business
//! logic lives in ContextService; these commands only handle IPC concerns
//! (serialization, error mapping, state extraction).

use tauri::State;
use uuid::Uuid;

use crate::services::ContextService;
use crate::session::types::SessionSummary;

/// Gets the most recent session for Smart Resume.
///
/// Returns the latest active session across all workspaces, or None if
/// no recent sessions exist. Used by the Dashboard to display the
/// "Continue Working" banner.
#[tauri::command]
pub async fn get_smart_resume_session(
    context_service: State<'_, ContextService>,
) -> Result<Option<SessionSummary>, String> {
    context_service
        .get_smart_resume_session()
        .await
        .map_err(|e| e.to_string())
}

/// Gets recent sessions for a specific workspace.
///
/// Used for workspace analytics and session history views.
///
/// # Arguments
/// * `workspace_id` - Workspace UUID as string
/// * `limit` - Maximum number of sessions to return (optional)
#[tauri::command]
pub async fn get_workspace_sessions(
    workspace_id: String,
    limit: Option<usize>,
    context_service: State<'_, ContextService>,
) -> Result<Vec<crate::session::types::Session>, String> {
    let workspace_id =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace_id: {}", e))?;

    context_service
        .get_workspace_sessions(workspace_id, limit)
        .await
        .map_err(|e| e.to_string())
}

/// Gets the latest session for a specific workspace with full details.
///
/// Returns None if the workspace has no timeline events or sessions.
#[tauri::command]
pub async fn get_latest_workspace_session(
    workspace_id: String,
    context_service: State<'_, ContextService>,
) -> Result<Option<SessionSummary>, String> {
    let workspace_id =
        Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace_id: {}", e))?;

    context_service
        .get_latest_workspace_session(workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Updates the session inactivity threshold setting.
///
/// This allows users to customize what constitutes a "break" between
/// sessions (e.g. 15 minutes, 30 minutes, 60 minutes).
///
/// # Arguments
/// * `threshold_seconds` - Inactivity threshold in seconds (60-14400)
#[tauri::command]
pub async fn set_session_inactivity_threshold(
    threshold_seconds: i64,
    context_service: State<'_, ContextService>,
) -> Result<(), String> {
    context_service
        .set_inactivity_threshold(threshold_seconds)
        .await
        .map_err(|e| e.to_string())
}

/// Gets the current session inactivity threshold setting.
///
/// Returns the threshold in seconds.
#[tauri::command]
pub async fn get_session_inactivity_threshold(
    context_service: State<'_, ContextService>,
) -> Result<i64, String> {
    context_service
        .get_inactivity_threshold_setting()
        .await
        .map_err(|e| e.to_string())
}
