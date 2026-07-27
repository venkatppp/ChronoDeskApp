use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::errors::DatabaseError;

/// The kind of activity a [`TimelineEvent`] records (blueprint §10).
/// Mirrors the `CHECK` constraint on `timeline_events.event_type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    Create,
    Open,
    Close,
    Edit,
    Move,
    Delete,
    Commit,
    Visit,
    Screenshot,
    WorkspaceSwitch,
}

impl TimelineEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimelineEventType::Create => "create",
            TimelineEventType::Open => "open",
            TimelineEventType::Close => "close",
            TimelineEventType::Edit => "edit",
            TimelineEventType::Move => "move",
            TimelineEventType::Delete => "delete",
            TimelineEventType::Commit => "commit",
            TimelineEventType::Visit => "visit",
            TimelineEventType::Screenshot => "screenshot",
            TimelineEventType::WorkspaceSwitch => "workspace_switch",
        }
    }
}

impl fmt::Display for TimelineEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TimelineEventType {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create" => Ok(TimelineEventType::Create),
            "open" => Ok(TimelineEventType::Open),
            "close" => Ok(TimelineEventType::Close),
            "edit" => Ok(TimelineEventType::Edit),
            "move" => Ok(TimelineEventType::Move),
            "delete" => Ok(TimelineEventType::Delete),
            "commit" => Ok(TimelineEventType::Commit),
            "visit" => Ok(TimelineEventType::Visit),
            "screenshot" => Ok(TimelineEventType::Screenshot),
            "workspace_switch" => Ok(TimelineEventType::WorkspaceSwitch),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown timeline event type '{other}'"
            ))),
        }
    }
}

/// One entry in a workspace's append-only activity log (blueprint §10).
/// Never mutated after insertion — there is no `update` operation on
/// [`crate::repositories::timeline_repository::TimelineRepository`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEvent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub file_id: Option<Uuid>,
    pub event_type: TimelineEventType,
    pub occurred_at: DateTime<Utc>,
    /// Free-form, event-type-specific JSON payload (e.g. `{"diff_lines": 12}`
    /// for an `edit` event). Stored as TEXT in SQLite and left as a raw
    /// [`serde_json::Value`] here rather than a fixed struct, since each
    /// `event_type` has a different, evolving shape.
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Raw shape of a `timeline_events` row; see
/// [`crate::models::workspace::WorkspaceRow`] for why enum columns are
/// decoded as `String` first.
#[derive(Debug, FromRow)]
pub(crate) struct TimelineEventRow {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub file_id: Option<Uuid>,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<TimelineEventRow> for TimelineEvent {
    type Error = DatabaseError;

    fn try_from(row: TimelineEventRow) -> Result<Self, Self::Error> {
        let metadata = row
            .metadata
            .map(|raw| {
                serde_json::from_str(&raw).map_err(|e| {
                    DatabaseError::InvalidInput(format!("corrupt timeline event metadata: {e}"))
                })
            })
            .transpose()?;

        Ok(TimelineEvent {
            id: row.id,
            workspace_id: row.workspace_id,
            file_id: row.file_id,
            event_type: TimelineEventType::from_str(&row.event_type)?,
            occurred_at: row.occurred_at,
            metadata,
            created_at: row.created_at,
        })
    }
}

/// Input for [`TimelineRepository::create`](crate::repositories::timeline_repository::TimelineRepository::create).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTimelineEvent {
    pub workspace_id: Uuid,
    #[serde(default)]
    pub file_id: Option<Uuid>,
    pub event_type: TimelineEventType,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}
