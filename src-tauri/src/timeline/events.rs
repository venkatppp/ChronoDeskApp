//! Domain-level timeline activities and their mapping onto the stored
//! [`TimelineEventType`] + metadata representation (blueprint §10).
//!
//! [`TimelineActivity`] is the vocabulary the rest of the backend speaks
//! in — "a file was created", "a workspace was opened" — kept separate
//! from [`crate::models::timeline::TimelineEventType`], which is
//! deliberately a smaller, storage-oriented enum matching the database's
//! `CHECK` constraint (`migrations/0003_timeline_event_create_type.sql`).
//! Every [`TimelineActivity`] maps onto exactly one `TimelineEventType`
//! plus a structured metadata payload carrying the detail the storage
//! type alone can't (e.g. *which* file, or *why* a workspace was
//! created).

use crate::models::TimelineEventType;

/// A significant, human-meaningful action worth recording on a
/// workspace's timeline — the exact vocabulary Phase 3 specifies
/// (workspace created/opened/closed/renamed, file created/modified/
/// deleted, project imported).
#[derive(Debug, Clone, PartialEq)]
pub enum TimelineActivity {
    /// A brand-new workspace was detected and created for the first time.
    WorkspaceCreated,
    /// An existing workspace just became the active one — file activity
    /// resumed under its root, or the user explicitly opened it.
    WorkspaceOpened,
    /// A workspace was explicitly closed. Modeled now so the mapping
    /// below is complete, even though nothing in Phase 3's pipeline
    /// emits it yet — there is no "the user switched away" trigger until
    /// multi-workspace session tracking exists (a reasonable Phase 4
    /// addition once the desktop app tracks foreground/background state).
    WorkspaceClosed,
    /// A workspace's name changed.
    WorkspaceRenamed {
        previous_name: String,
        new_name: String,
    },
    /// An existing directory was brought under ChronoDesk's management
    /// (a watch path was added covering it), as opposed to being
    /// discovered incrementally file-by-file.
    ProjectImported,
    /// A new artifact was created under a workspace.
    FileCreated { path: String },
    /// An existing artifact's contents changed.
    FileModified { path: String },
    /// An artifact was removed.
    FileDeleted { path: String },
    /// An artifact was renamed or moved to a new path within the same
    /// workspace.
    FileMoved { from: String, to: String },
}

impl TimelineActivity {
    /// Maps this activity onto the `(event_type, metadata)` pair
    /// [`crate::repositories::timeline_repository::TimelineRepository::create`]
    /// persists. Kept as one exhaustive `match` so adding a new
    /// [`TimelineActivity`] variant without updating this function is a
    /// compile error, not a silent gap.
    pub fn to_event_type_and_metadata(&self) -> (TimelineEventType, Option<serde_json::Value>) {
        match self {
            TimelineActivity::WorkspaceCreated => (
                TimelineEventType::WorkspaceSwitch,
                Some(serde_json::json!({ "activity": "workspace_created" })),
            ),
            TimelineActivity::WorkspaceOpened => (
                TimelineEventType::WorkspaceSwitch,
                Some(serde_json::json!({ "activity": "workspace_opened" })),
            ),
            TimelineActivity::WorkspaceClosed => (
                TimelineEventType::WorkspaceSwitch,
                Some(serde_json::json!({ "activity": "workspace_closed" })),
            ),
            TimelineActivity::WorkspaceRenamed {
                previous_name,
                new_name,
            } => (
                TimelineEventType::WorkspaceSwitch,
                Some(serde_json::json!({
                    "activity": "workspace_renamed",
                    "previous_name": previous_name,
                    "new_name": new_name,
                })),
            ),
            TimelineActivity::ProjectImported => (
                TimelineEventType::WorkspaceSwitch,
                Some(serde_json::json!({ "activity": "project_imported" })),
            ),
            TimelineActivity::FileCreated { path } => (
                TimelineEventType::Create,
                Some(serde_json::json!({ "path": path })),
            ),
            TimelineActivity::FileModified { path } => (
                TimelineEventType::Edit,
                Some(serde_json::json!({ "path": path })),
            ),
            TimelineActivity::FileDeleted { path } => (
                TimelineEventType::Delete,
                Some(serde_json::json!({ "path": path })),
            ),
            TimelineActivity::FileMoved { from, to } => (
                TimelineEventType::Move,
                Some(serde_json::json!({ "from": from, "to": to })),
            ),
        }
    }

    /// The artifact path this activity refers to, if it's a file-level
    /// activity (as opposed to a workspace-level one). Used by
    /// [`super::recorder::TimelineRecorder`] to decide whether it needs
    /// to resolve or create a `files` row before recording the event.
    /// For a move, this is the *destination* path — the row now lives
    /// there.
    pub fn file_path(&self) -> Option<&str> {
        match self {
            TimelineActivity::FileCreated { path }
            | TimelineActivity::FileModified { path }
            | TimelineActivity::FileDeleted { path } => Some(path),
            TimelineActivity::FileMoved { to, .. } => Some(to),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_created_maps_to_create_with_path_metadata() {
        let activity = TimelineActivity::FileCreated {
            path: "/repo/src/main.rs".to_string(),
        };
        let (event_type, metadata) = activity.to_event_type_and_metadata();

        assert_eq!(event_type, TimelineEventType::Create);
        assert_eq!(metadata.unwrap()["path"], "/repo/src/main.rs");
        assert_eq!(activity.file_path(), Some("/repo/src/main.rs"));
    }

    #[test]
    fn file_moved_reports_destination_as_file_path() {
        let activity = TimelineActivity::FileMoved {
            from: "/repo/old.rs".to_string(),
            to: "/repo/new.rs".to_string(),
        };
        assert_eq!(activity.file_path(), Some("/repo/new.rs"));

        let (event_type, _) = activity.to_event_type_and_metadata();
        assert_eq!(event_type, TimelineEventType::Move);
    }

    #[test]
    fn workspace_level_activities_have_no_file_path() {
        assert_eq!(TimelineActivity::WorkspaceCreated.file_path(), None);
        assert_eq!(TimelineActivity::WorkspaceOpened.file_path(), None);
        assert_eq!(TimelineActivity::ProjectImported.file_path(), None);
    }

    #[test]
    fn workspace_renamed_carries_both_names_in_metadata() {
        let activity = TimelineActivity::WorkspaceRenamed {
            previous_name: "old-name".to_string(),
            new_name: "new-name".to_string(),
        };
        let (event_type, metadata) = activity.to_event_type_and_metadata();

        assert_eq!(event_type, TimelineEventType::WorkspaceSwitch);
        let metadata = metadata.unwrap();
        assert_eq!(metadata["previous_name"], "old-name");
        assert_eq!(metadata["new_name"], "new-name");
    }
}
