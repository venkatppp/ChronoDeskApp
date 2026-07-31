//! Action service layer.
//!
//! Service layer that orchestrates action execution and history management.

use crate::actions::engine::ActionEngine;
use crate::actions::models::{ActionHistory, ActionResult, ExecuteActionRequest};
use crate::actions::repository::ActionRepository;
use crate::errors::DatabaseError;

/// Service for managing actions.
pub struct ActionService {
    action_repo: ActionRepository,
    engine: ActionEngine,
}

impl ActionService {
    /// Creates a new action service.
    pub fn new(action_repo: ActionRepository, engine: ActionEngine) -> Self {
        Self {
            action_repo,
            engine,
        }
    }

    /// Executes an action.
    pub async fn execute_action(
        &self,
        request: ExecuteActionRequest,
    ) -> Result<ActionResult, DatabaseError> {
        self.engine.execute(request).await
    }

    /// Undoes an action.
    pub async fn undo_action(&self, action_id: i64) -> Result<ActionResult, DatabaseError> {
        self.engine.undo(action_id).await
    }

    /// Gets action history for a workspace.
    pub async fn get_workspace_history(
        &self,
        workspace_id: i64,
        limit: i64,
    ) -> Result<Vec<ActionHistory>, DatabaseError> {
        self.action_repo.get_by_workspace(workspace_id, limit).await
    }

    /// Gets all action history.
    pub async fn get_all_history(&self, limit: i64) -> Result<Vec<ActionHistory>, DatabaseError> {
        self.action_repo.get_all(limit).await
    }

    /// Clears all action history.
    pub async fn clear_all_history(&self) -> Result<(), DatabaseError> {
        self.action_repo.clear_all().await
    }

    /// Clears action history for a workspace.
    pub async fn clear_workspace_history(&self, workspace_id: i64) -> Result<(), DatabaseError> {
        self.action_repo.clear_by_workspace(workspace_id).await
    }
}
