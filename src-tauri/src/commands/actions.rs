//! Action execution commands.

use crate::actions::models::ExecuteActionRequest;
use crate::actions::service::ActionService;
use tauri::State;

/// Execute an action.
#[tauri::command]
pub async fn execute_action(
    request: ExecuteActionRequest,
    service: State<'_, ActionService>,
) -> Result<crate::actions::models::ActionResult, String> {
    service
        .execute_action(request)
        .await
        .map_err(|e| e.to_string())
}

/// Undo an action.
#[tauri::command]
pub async fn undo_action(
    action_id: i64,
    service: State<'_, ActionService>,
) -> Result<crate::actions::models::ActionResult, String> {
    service
        .undo_action(action_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get action history for a workspace.
#[tauri::command]
pub async fn get_action_history(
    workspace_id: i64,
    limit: Option<i64>,
    service: State<'_, ActionService>,
) -> Result<Vec<crate::actions::models::ActionHistory>, String> {
    service
        .get_workspace_history(workspace_id, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

/// Get all action history.
#[tauri::command]
pub async fn get_all_action_history(
    limit: Option<i64>,
    service: State<'_, ActionService>,
) -> Result<Vec<crate::actions::models::ActionHistory>, String> {
    service
        .get_all_history(limit.unwrap_or(100))
        .await
        .map_err(|e| e.to_string())
}

/// Clear all action history.
#[tauri::command]
pub async fn clear_action_history(service: State<'_, ActionService>) -> Result<(), String> {
    service.clear_all_history().await.map_err(|e| e.to_string())
}

/// Clear action history for a workspace.
#[tauri::command]
pub async fn clear_workspace_action_history(
    workspace_id: i64,
    service: State<'_, ActionService>,
) -> Result<(), String> {
    service
        .clear_workspace_history(workspace_id)
        .await
        .map_err(|e| e.to_string())
}
