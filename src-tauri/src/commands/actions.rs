//! Action execution commands.

use crate::actions::models::ExecuteActionRequest;
use crate::actions::service::ActionService;
use crate::runtime::IntelligenceEmitter;
use tauri::State;
use uuid::Uuid;

/// Execute an action.
#[tauri::command]
pub async fn execute_action(
    request: ExecuteActionRequest,
    service: State<'_, ActionService>,
    emitter: State<'_, IntelligenceEmitter>,
) -> Result<crate::actions::models::ActionResult, String> {
    let result = service
        .execute_action(request.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Emit action executed event
    if let Some(workspace_id_str) = &request.workspace_id {
        if let Ok(workspace_id) = Uuid::parse_str(workspace_id_str) {
            emitter.emit_action_executed(
                workspace_id,
                format!("{:?}", request.action_type),
                result.success,
                result.error.clone(),
            );
        }
    }

    Ok(result)
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
    workspace_id: String,
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
    workspace_id: String,
    service: State<'_, ActionService>,
) -> Result<(), String> {
    service
        .clear_workspace_history(workspace_id)
        .await
        .map_err(|e| e.to_string())
}
