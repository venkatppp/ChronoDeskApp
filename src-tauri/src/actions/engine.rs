//! Action execution engine.
//!
//! Stateless engine that consumes recommendation output and executes concrete actions.

use crate::actions::executors::{execute_action, ExecutorContext};
use crate::actions::models::{ActionResult, ActionType, ExecuteActionRequest};
use crate::actions::repository::ActionRepository;
use crate::errors::DatabaseError;
use crate::repositories::{FileRepository, WorkspaceRepository};

/// Engine for executing intelligent actions.
pub struct ActionEngine {
    action_repo: ActionRepository,
    workspace_repo: WorkspaceRepository,
    file_repo: FileRepository,
}

impl ActionEngine {
    /// Creates a new action engine.
    pub fn new(
        action_repo: ActionRepository,
        workspace_repo: WorkspaceRepository,
        file_repo: FileRepository,
    ) -> Self {
        Self {
            action_repo,
            workspace_repo,
            file_repo,
        }
    }

    /// Executes an action and records it in history.
    pub async fn execute(
        &self,
        request: ExecuteActionRequest,
    ) -> Result<ActionResult, DatabaseError> {
        // Create executor context
        let ctx = ExecutorContext::new(self.workspace_repo.clone(), self.file_repo.clone());

        // Execute the action
        let result = execute_action(
            &ctx,
            request.action_type.clone(),
            request.workspace_id,
            &request.metadata,
        )
        .await?;

        // Record in history
        let history = self
            .action_repo
            .create(
                request.action_type,
                request.workspace_id,
                request.recommendation_id,
                result.success,
                result.data.clone(),
                result.undo_state,
            )
            .await?;

        Ok(ActionResult {
            success: result.success,
            message: result.message,
            action_id: history.id,
            data: result.data,
        })
    }

    /// Undoes a previously executed action.
    pub async fn undo(&self, action_id: i64) -> Result<ActionResult, DatabaseError> {
        // Get the action from history
        let action = self
            .action_repo
            .get_by_id(action_id)
            .await?
            .ok_or_else(|| DatabaseError::not_found("action", action_id.to_string()))?;

        if !action.can_undo() {
            return Err(DatabaseError::InvalidInput(
                "Action cannot be undone".to_string(),
            ));
        }

        let undo_state = action.undo_state.as_ref().unwrap();

        // Execute the undo based on action type
        let ctx = ExecutorContext::new(self.workspace_repo.clone(), self.file_repo.clone());

        let result = match action.action_type {
            ActionType::ArchiveWorkspace => {
                if let Some(was_archived) = undo_state.was_archived {
                    if !was_archived {
                        // Was active, restore to active
                        let metadata = serde_json::json!({});
                        execute_action(
                            &ctx,
                            ActionType::RestoreWorkspace,
                            action.workspace_id,
                            &metadata,
                        )
                        .await?
                    } else {
                        return Err(DatabaseError::InvalidInput(
                            "Invalid undo state".to_string(),
                        ));
                    }
                } else {
                    return Err(DatabaseError::InvalidInput(
                        "Invalid undo state".to_string(),
                    ));
                }
            }
            ActionType::RestoreWorkspace => {
                if let Some(was_archived) = undo_state.was_archived {
                    if was_archived {
                        // Was archived, restore to archived
                        let metadata = serde_json::json!({});
                        execute_action(
                            &ctx,
                            ActionType::ArchiveWorkspace,
                            action.workspace_id,
                            &metadata,
                        )
                        .await?
                    } else {
                        return Err(DatabaseError::InvalidInput(
                            "Invalid undo state".to_string(),
                        ));
                    }
                } else {
                    return Err(DatabaseError::InvalidInput(
                        "Invalid undo state".to_string(),
                    ));
                }
            }
            ActionType::PinWorkspace => {
                let metadata = serde_json::json!({});
                execute_action(
                    &ctx,
                    ActionType::UnpinWorkspace,
                    action.workspace_id,
                    &metadata,
                )
                .await?
            }
            ActionType::UnpinWorkspace => {
                let metadata = serde_json::json!({});
                execute_action(
                    &ctx,
                    ActionType::PinWorkspace,
                    action.workspace_id,
                    &metadata,
                )
                .await?
            }
            ActionType::CleanDuplicateFiles => {
                // Cannot undo file deletion in this implementation
                return Err(DatabaseError::InvalidInput(
                    "File deletion cannot be undone".to_string(),
                ));
            }
            _ => {
                return Err(DatabaseError::InvalidInput(
                    "Action type does not support undo".to_string(),
                ));
            }
        };

        // Record the undo action
        let undo_action_type = match action.action_type {
            ActionType::ArchiveWorkspace => ActionType::RestoreWorkspace,
            ActionType::RestoreWorkspace => ActionType::ArchiveWorkspace,
            ActionType::PinWorkspace => ActionType::UnpinWorkspace,
            ActionType::UnpinWorkspace => ActionType::PinWorkspace,
            _ => action.action_type,
        };

        let history = self
            .action_repo
            .create(
                undo_action_type,
                action.workspace_id,
                None,
                result.success,
                result.data.clone(),
                None,
            )
            .await?;

        Ok(ActionResult {
            success: result.success,
            message: format!("Undone: {}", result.message),
            action_id: history.id,
            data: result.data,
        })
    }
}
