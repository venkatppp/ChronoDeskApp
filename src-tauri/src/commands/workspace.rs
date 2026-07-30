//! Workspace IPC commands.
//!
//! Every handler here does exactly two things: pull the managed
//! [`WorkspaceService`] out of Tauri state, and call one method on it.
//! No SQL, no business rules, no validation logic lives in this file —
//! see [`crate::services::workspace_service`] for that. Keeping commands
//! this thin means the same business logic is exercised whether it's
//! invoked from the frontend or from a `#[tokio::test]`.
//!
//! Note on naming: Tauri registers commands globally by their bare
//! function name (not module-qualified), so these are named
//! `list_active_workspaces` / `get_workspace` / etc. rather than the
//! shorter `list_active` / `get` — that avoids a same-name collision with
//! a future `commands::file`/`commands::timeline` module that will
//! plausibly also want a `get` or `create` command. The `workspace::`
//! grouping the spec asks for is expressed by this file living in
//! `commands::workspace`, which is how the Rust side organizes and
//! documents them even though the JS side calls them by bare name.

use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::app_events::{
    self, EVENT_WORKSPACE_CREATED, EVENT_WORKSPACE_DELETED, EVENT_WORKSPACE_SWITCHED,
    EVENT_WORKSPACE_UPDATED,
};
use crate::errors::DatabaseError;
use crate::models::{CreateWorkspaceInput, UpdateWorkspaceInput, Workspace};
use crate::services::WorkspaceService;

/// Lists every active workspace, most recently active first.
/// Backs the dashboard's "Active workspaces" grid (blueprint §3.2).
#[tauri::command]
pub async fn list_active_workspaces(
    service: State<'_, WorkspaceService>,
) -> Result<Vec<Workspace>, DatabaseError> {
    service.list_active_workspaces().await
}

/// Lists every archived workspace, most recently active first.
/// Backs the "Archived" filter tab on the Workspaces screen.
#[tauri::command]
pub async fn list_archived_workspaces(
    service: State<'_, WorkspaceService>,
) -> Result<Vec<Workspace>, DatabaseError> {
    service.list_archived_workspaces().await
}

/// Fetches a single workspace by id.
///
/// # Errors
/// Returns [`DatabaseError::NotFound`] (serialized to a plain string
/// across IPC) if `id` doesn't exist.
#[tauri::command]
pub async fn get_workspace(
    service: State<'_, WorkspaceService>,
    id: Uuid,
) -> Result<Workspace, DatabaseError> {
    service.get_workspace(id).await
}

/// Creates a new workspace and emits [`EVENT_WORKSPACE_CREATED`] so every
/// listening window updates without a manual refresh.
///
/// # Errors
/// [`DatabaseError::InvalidInput`] if `input.name` is empty.
#[tauri::command]
pub async fn create_workspace(
    app: AppHandle,
    service: State<'_, WorkspaceService>,
    input: CreateWorkspaceInput,
) -> Result<Workspace, DatabaseError> {
    let workspace = service.create_workspace(input).await?;
    app_events::emit(&app, EVENT_WORKSPACE_CREATED, &workspace);
    Ok(workspace)
}

/// Applies a partial update to a workspace and emits
/// [`EVENT_WORKSPACE_UPDATED`]. See [`UpdateWorkspaceInput`] for PATCH
/// semantics (a field left as `None` is left unchanged).
///
/// # Errors
/// [`DatabaseError::NotFound`] if `id` doesn't exist;
/// [`DatabaseError::InvalidInput`] for an invalid `name`/`health_score`.
#[tauri::command]
pub async fn update_workspace(
    app: AppHandle,
    service: State<'_, WorkspaceService>,
    id: Uuid,
    input: UpdateWorkspaceInput,
) -> Result<Workspace, DatabaseError> {
    let workspace = service.update_workspace(id, input).await?;
    app_events::emit(&app, EVENT_WORKSPACE_UPDATED, &workspace);
    Ok(workspace)
}

/// Permanently deletes a workspace — and, via cascading foreign keys,
/// every file/timeline/tag/relationship row that referenced it — then
/// emits [`EVENT_WORKSPACE_DELETED`] with `{ "id": <uuid> }`.
///
/// # Errors
/// [`DatabaseError::NotFound`] if `id` doesn't exist.
#[tauri::command]
pub async fn delete_workspace(
    app: AppHandle,
    service: State<'_, WorkspaceService>,
    id: Uuid,
) -> Result<(), DatabaseError> {
    service.delete_workspace(id).await?;
    app_events::emit(
        &app,
        EVENT_WORKSPACE_DELETED,
        &serde_json::json!({ "id": id }),
    );
    Ok(())
}

/// Switches the active workspace and broadcasts the change.
#[tauri::command]
pub async fn switch_workspace(
    app: AppHandle,
    service: State<'_, WorkspaceService>,
    id: Uuid,
) -> Result<(), DatabaseError> {
    service.switch_workspace(id).await?;

    app_events::emit(
        &app,
        EVENT_WORKSPACE_SWITCHED,
        &serde_json::json!({ "id": id }),
    );

    Ok(())
}
