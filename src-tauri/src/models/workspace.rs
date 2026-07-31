use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::errors::DatabaseError;

/// Lifecycle state of a [`Workspace`] (blueprint §7.2).
///
/// Stored in SQLite as the lowercase TEXT produced by [`WorkspaceStatus::as_str`]
/// and constrained at the schema level by a `CHECK` clause in
/// `migrations/0001_initial_schema.sql`, so the two representations can
/// never silently drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    Active,
    Archived,
}

impl WorkspaceStatus {
    /// The exact lowercase string stored in the `workspaces.status` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceStatus::Active => "active",
            WorkspaceStatus::Archived => "archived",
        }
    }
}

impl fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkspaceStatus {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(WorkspaceStatus::Active),
            "archived" => Ok(WorkspaceStatus::Archived),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown workspace status '{other}'"
            ))),
        }
    }
}

/// A ChronoDesk workspace: the auto-maintained container linking every
/// artifact a user touches while working on one piece of work (blueprint
/// §1.2). This is the public, strongly typed model returned by
/// [`crate::repositories::workspace_repository::WorkspaceRepository`] and
/// serialized directly across the Tauri IPC boundary to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: WorkspaceStatus,
    /// Composite 0–100 health score (blueprint §12). Enforced at the
    /// schema level by a `CHECK (health_score BETWEEN 0 AND 100)` clause.
    pub health_score: f64,
    /// Filesystem directory this workspace corresponds to, if it was
    /// created (or matched) by the Workspace Engine's detector rather
    /// than created manually from the UI. `None` for manually-created
    /// workspaces with no filesystem association. Unique when present
    /// (enforced by a partial unique index — see
    /// `migrations/0002_workspace_root_path.sql`), so
    /// [`WorkspaceRepository::find_by_root_path`](crate::repositories::workspace_repository::WorkspaceRepository::find_by_root_path)
    /// can answer "does a workspace already exist for this directory?"
    /// with an indexed point lookup.
    pub root_path: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw shape of a `workspaces` row as SQLite/sqlx decode it. `status` stays
/// a `String` here — SQLite has no enum type — and is parsed into
/// [`WorkspaceStatus`] by [`TryFrom<WorkspaceRow>`] below. Never exposed
/// outside `repositories::workspace_repository`.
#[derive(Debug, FromRow)]
pub(crate) struct WorkspaceRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub health_score: f64,
    pub root_path: Option<String>,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<WorkspaceRow> for Workspace {
    type Error = DatabaseError;

    fn try_from(row: WorkspaceRow) -> Result<Self, Self::Error> {
        Ok(Workspace {
            id: row.id,
            name: row.name,
            description: row.description,
            status: WorkspaceStatus::from_str(&row.status)?,
            health_score: row.health_score,
            root_path: row.root_path,
            last_active_at: row.last_active_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Aggregated statistics for a single workspace, returned by
/// [`WorkspaceRepository::get_workspace_stats`].
/// Combines file count, timeline event count, health score, and recency
/// in a single IPC-friendly response so the dashboard doesn't need
/// multiple round-trips to render a workspace card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceStats {
    pub workspace_id: Uuid,
    pub file_count: i64,
    pub timeline_event_count: i64,
    pub last_activity: DateTime<Utc>,
    pub health_score: f64,
}

/// Input for [`WorkspaceRepository::create`](crate::repositories::workspace_repository::WorkspaceRepository::create).
/// Deserialized directly from the JSON payload a Tauri command receives
/// from the frontend.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Set by [`crate::workspace::manager::WorkspaceManager`] when a
    /// workspace is created from a detected filesystem root. Left as
    /// `None` for a manually-created workspace (e.g. the "+ New
    /// workspace" button in the UI).
    #[serde(default)]
    pub root_path: Option<String>,
}

/// Partial update for [`WorkspaceRepository::update`](crate::repositories::workspace_repository::WorkspaceRepository::update).
///
/// Every field is optional and follows PATCH semantics: `None` means
/// "leave this column unchanged". To explicitly clear `description`,
/// pass `Some(String::new())` —
/// [`WorkspaceRepository::update`](crate::repositories::workspace_repository::WorkspaceRepository::update)
/// treats an empty string as "set to NULL" rather than a literal
/// zero-length value, since an empty description is never meaningfully
/// different from no description.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkspaceInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<WorkspaceStatus>,
    pub health_score: Option<f64>,
}
