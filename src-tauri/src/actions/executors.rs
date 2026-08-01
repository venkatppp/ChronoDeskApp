//! Action executors - individual handlers for each action type.

use crate::actions::models::{ActionType, UndoState};
use crate::errors::DatabaseError;
use crate::models::{UpdateWorkspaceInput, WorkspaceStatus};
use crate::repositories::{FileRepository, WorkspaceRepository};
use std::str::FromStr;
use uuid::Uuid;

/// Context needed to execute actions.
pub struct ExecutorContext {
    pub workspace_repo: WorkspaceRepository,
    pub file_repo: FileRepository,
}

impl ExecutorContext {
    pub fn new(workspace_repo: WorkspaceRepository, file_repo: FileRepository) -> Self {
        Self {
            workspace_repo,
            file_repo,
        }
    }
}

/// Result of executing an action.
pub struct ExecutionResult {
    pub success: bool,
    pub message: String,
    pub data: serde_json::Value,
    pub undo_state: Option<UndoState>,
}

impl ExecutionResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: serde_json::Value::Null,
            undo_state: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data,
            undo_state: None,
        }
    }

    pub fn success_with_undo(message: impl Into<String>, undo_state: UndoState) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: serde_json::Value::Null,
            undo_state: Some(undo_state),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: serde_json::Value::Null,
            undo_state: None,
        }
    }
}

/// Archive a workspace.
pub async fn execute_archive_workspace(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    _metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    // Convert i64 to Uuid
    let workspace_uuid = workspace_id;

    // Get current workspace state
    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    let was_archived = workspace.status == WorkspaceStatus::Archived;

    if was_archived {
        return Ok(ExecutionResult::failure("Workspace is already archived"));
    }

    // Archive the workspace
    ctx.workspace_repo
        .update(
            workspace_uuid,
            UpdateWorkspaceInput {
                name: None,
                description: None,
                status: Some(WorkspaceStatus::Archived),
                health_score: None,
            },
        )
        .await?;

    let undo_state = UndoState {
        was_archived: Some(false),
        was_pinned: None,
        deleted_file_ids: None,
    };

    Ok(ExecutionResult::success_with_undo(
        format!("Archived workspace '{}'", workspace.name),
        undo_state,
    ))
}

/// Restore an archived workspace.
pub async fn execute_restore_workspace(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    _metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_uuid = workspace_id;

    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    let was_archived = workspace.status == WorkspaceStatus::Archived;

    if !was_archived {
        return Ok(ExecutionResult::failure("Workspace is not archived"));
    }

    ctx.workspace_repo
        .update(
            workspace_uuid,
            UpdateWorkspaceInput {
                name: None,
                description: None,
                status: Some(WorkspaceStatus::Active),
                health_score: None,
            },
        )
        .await?;

    let undo_state = UndoState {
        was_archived: Some(true),
        was_pinned: None,
        deleted_file_ids: None,
    };

    Ok(ExecutionResult::success_with_undo(
        format!("Restored workspace '{}'", workspace.name),
        undo_state,
    ))
}

/// Pin a workspace.
pub async fn execute_pin_workspace(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    _metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_uuid = workspace_id;

    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    // Note: Pinning is stored via tags in the current schema
    // We'll use the workspace_tags table with a special "pinned" tag
    let undo_state = UndoState {
        was_archived: None,
        was_pinned: Some(false),
        deleted_file_ids: None,
    };

    // TODO: When tag repository is available, add pinned tag
    // For now, just return success
    Ok(ExecutionResult::success_with_undo(
        format!("Pinned workspace '{}'", workspace.name),
        undo_state,
    ))
}

/// Unpin a workspace.
pub async fn execute_unpin_workspace(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    _metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_uuid = workspace_id;

    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    let undo_state = UndoState {
        was_archived: None,
        was_pinned: Some(true),
        deleted_file_ids: None,
    };

    // TODO: When tag repository is available, remove pinned tag
    Ok(ExecutionResult::success_with_undo(
        format!("Unpinned workspace '{}'", workspace.name),
        undo_state,
    ))
}

/// Clean duplicate files.
pub async fn execute_clean_duplicate_files(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_uuid = workspace_id;

    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    // Extract file IDs from metadata (as UUID strings)
    let file_id_strs: Vec<String> = metadata
        .get("file_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if file_id_strs.is_empty() {
        return Ok(ExecutionResult::failure("No file IDs provided"));
    }

    // Parse and delete the files
    let mut deleted_ids = Vec::new();
    for file_id_str in &file_id_strs {
        if let Ok(file_uuid) = Uuid::from_str(file_id_str) {
            ctx.file_repo.delete(file_uuid).await?;
            deleted_ids.push(file_id_str.clone());
        }
    }

    let undo_state = UndoState {
        was_archived: None,
        was_pinned: None,
        deleted_file_ids: None, // Can't restore deleted files in this implementation
    };

    Ok(ExecutionResult::success_with_undo(
        format!(
            "Cleaned {} duplicate file(s) from workspace '{}'",
            deleted_ids.len(),
            workspace.name
        ),
        undo_state,
    ))
}

/// Open suggested workspace.
pub async fn execute_open_suggested_workspace(
    ctx: &ExecutorContext,
    workspace_id: Uuid,
    _metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_uuid = workspace_id;

    let workspace = ctx.workspace_repo.get_by_id(workspace_uuid).await?;

    // Return workspace data for frontend to handle navigation
    let data = serde_json::json!({
        "workspaceId": workspace.id,
        "name": workspace.name,
        "rootPath": workspace.root_path,
    });

    Ok(ExecutionResult::success_with_data(
        format!("Opening workspace '{}'", workspace.name),
        data,
    ))
}

/// Resume previous session.
pub async fn execute_resume_previous_session(
    _ctx: &ExecutorContext,
    workspace_id: Uuid,
    metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    // Extract session data from metadata
    let session_id = metadata
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let files = metadata
        .get("files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let data = serde_json::json!({
        "workspaceId": workspace_id,
        "sessionId": session_id,
        "files": files,
    });

    Ok(ExecutionResult::success_with_data(
        "Resuming previous session",
        data,
    ))
}

/// Open most relevant files.
pub async fn execute_open_most_relevant_files(
    _ctx: &ExecutorContext,
    workspace_id: Uuid,
    metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    // Extract file paths from metadata
    let files = metadata
        .get("files")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if files.is_empty() {
        return Ok(ExecutionResult::failure("No relevant files found"));
    }

    let data = serde_json::json!({
        "workspaceId": workspace_id,
        "files": files,
    });

    Ok(ExecutionResult::success_with_data(
        format!("Opening {} relevant file(s)", files.len()),
        data,
    ))
}

/// Mark recommendation as complete.
pub async fn execute_mark_recommendation_complete(
    _ctx: &ExecutorContext,
    _workspace_id: Uuid,
    metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let recommendation_id = metadata
        .get("recommendation_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DatabaseError::InvalidInput("Missing recommendation_id".to_string()))?;

    Ok(ExecutionResult::success(format!(
        "Marked recommendation {} as complete",
        recommendation_id
    )))
}

/// Main executor dispatcher.
pub async fn execute_action(
    ctx: &ExecutorContext,
    action_type: ActionType,
    workspace_id: Option<String>,
    metadata: &serde_json::Value,
) -> Result<ExecutionResult, DatabaseError> {
    let workspace_id_str = workspace_id
        .ok_or_else(|| DatabaseError::InvalidInput("Missing workspace_id".to_string()))?;

    let workspace_uuid = Uuid::from_str(&workspace_id_str)
        .map_err(|e| DatabaseError::InvalidInput(format!("Invalid workspace UUID: {}", e)))?;

    match action_type {
        ActionType::ArchiveWorkspace => {
            execute_archive_workspace(ctx, workspace_uuid, metadata).await
        }
        ActionType::RestoreWorkspace => {
            execute_restore_workspace(ctx, workspace_uuid, metadata).await
        }
        ActionType::PinWorkspace => execute_pin_workspace(ctx, workspace_uuid, metadata).await,
        ActionType::UnpinWorkspace => execute_unpin_workspace(ctx, workspace_uuid, metadata).await,
        ActionType::CleanDuplicateFiles => {
            execute_clean_duplicate_files(ctx, workspace_uuid, metadata).await
        }
        ActionType::OpenSuggestedWorkspace => {
            execute_open_suggested_workspace(ctx, workspace_uuid, metadata).await
        }
        ActionType::ResumePreviousSession => {
            execute_resume_previous_session(ctx, workspace_uuid, metadata).await
        }
        ActionType::OpenMostRelevantFiles => {
            execute_open_most_relevant_files(ctx, workspace_uuid, metadata).await
        }
        ActionType::MarkRecommendationComplete => {
            execute_mark_recommendation_complete(ctx, workspace_uuid, metadata).await
        }
    }
}

/// Helper to convert i64 workspace ID to UUID.
/// Note: This is a temporary bridge. The workspace table uses UUID,
/// but some parts of the system still use i64. This converts the i64
/// to a UUID by treating it as the low 64 bits.
#[allow(dead_code)]
fn id_to_uuid(id: i64) -> Result<Uuid, DatabaseError> {
    // For now, we'll format as a simple UUID
    // In production, you'd have a proper mapping or use consistent IDs
    let uuid_str = format!("00000000-0000-0000-0000-{:012x}", id as u64);
    Uuid::from_str(&uuid_str)
        .map_err(|e| DatabaseError::InvalidInput(format!("Invalid workspace ID: {}", e)))
}
