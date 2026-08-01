//! Action models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Action type that can be executed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Archive a workspace.
    ArchiveWorkspace,

    /// Restore an archived workspace.
    RestoreWorkspace,

    /// Pin a workspace.
    PinWorkspace,

    /// Unpin a workspace.
    UnpinWorkspace,

    /// Clean duplicate files.
    CleanDuplicateFiles,

    /// Open suggested workspace.
    OpenSuggestedWorkspace,

    /// Resume previous session.
    ResumePreviousSession,

    /// Open most relevant files.
    OpenMostRelevantFiles,

    /// Mark recommendation as complete.
    MarkRecommendationComplete,
}

/// Request to execute an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteActionRequest {
    /// Type of action to execute.
    pub action_type: ActionType,

    /// Target workspace ID (UUID string).
    pub workspace_id: Option<String>,

    /// Associated recommendation ID.
    pub recommendation_id: Option<String>,

    /// Additional metadata for the action.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Result of an action execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded.
    pub success: bool,

    /// Human-readable message.
    pub message: String,

    /// The action history record ID.
    pub action_id: i64,

    /// Data returned by the action.
    #[serde(default)]
    pub data: serde_json::Value,

    /// Optional error message.
    pub error: Option<String>,
}

/// Stored action history record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistory {
    /// Unique identifier.
    pub id: i64,

    /// Type of action executed.
    pub action_type: ActionType,

    /// Target workspace ID (UUID string).
    pub workspace_id: Option<String>,

    /// Associated recommendation ID.
    pub recommendation_id: Option<String>,

    /// When the action was executed.
    pub executed_at: DateTime<Utc>,

    /// Whether the action succeeded.
    pub success: bool,

    /// Additional metadata.
    pub metadata: serde_json::Value,

    /// State needed for undo (if applicable).
    pub undo_state: Option<UndoState>,
}

/// State needed to undo an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoState {
    /// Previous archived state (for archive/restore).
    pub was_archived: Option<bool>,

    /// Previous pinned state (for pin/unpin).
    pub was_pinned: Option<bool>,

    /// Files that were deleted (for clean duplicates).
    pub deleted_file_ids: Option<Vec<i64>>,
}

impl ActionHistory {
    /// Checks if this action can be undone.
    pub fn can_undo(&self) -> bool {
        self.success && self.undo_state.is_some()
    }
}
