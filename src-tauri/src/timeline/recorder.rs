//! [`TimelineRecorder`]: the single choke point for turning a
//! [`super::events::TimelineActivity`] into persisted rows. Resolves (or
//! creates) the `files` row a file-level activity refers to, then writes
//! the `timeline_events` row — so file-row bookkeeping never has to be
//! duplicated at every call site that wants to record something.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::{ArtifactType, NewFile, NewTimelineEvent, TimelineEvent};
use crate::repositories::{FileRepository, TimelineRepository};

use super::events::TimelineActivity;

/// Records [`TimelineActivity`] values onto a workspace's timeline.
#[derive(Debug, Clone)]
pub struct TimelineRecorder {
    file_repository: FileRepository,
    timeline_repository: TimelineRepository,
}

impl TimelineRecorder {
    pub fn new(file_repository: FileRepository, timeline_repository: TimelineRepository) -> Self {
        Self {
            file_repository,
            timeline_repository,
        }
    }

    /// Records `activity` against `workspace_id`, occurring at
    /// `occurred_at`. For a file-level activity, finds the existing
    /// `files` row for that path within the workspace or creates one —
    /// callers never manage file rows themselves.
    ///
    /// # Errors
    /// Propagates any [`DatabaseError`] from the underlying repositories,
    /// most commonly [`DatabaseError::Constraint`] if `workspace_id`
    /// doesn't reference an existing workspace.
    pub async fn record(
        &self,
        workspace_id: Uuid,
        activity: TimelineActivity,
        occurred_at: DateTime<Utc>,
    ) -> Result<TimelineEvent, DatabaseError> {
        // Defense-in-depth: never ingest generated/dependency/build
        // paths, even if a caller bypasses the watcher's ignore filter.
        // `is_ignored` is the shared exclusion source of truth.
        if let Some(path) = activity.file_path() {
            if crate::watcher::event_handler::is_ignored(std::path::Path::new(&path)) {
                tracing::debug!(path = %path, "skipping ignored path in timeline recorder");
                return Err(DatabaseError::InvalidInput(format!(
                    "path is inside an excluded directory: {path}"
                )));
            }
        }

        let file_id = match activity.file_path() {
            Some(path) => Some(self.resolve_file(workspace_id, path).await?),
            None => None,
        };

        let (event_type, metadata) = activity.to_event_type_and_metadata();

        let event = self
            .timeline_repository
            .create(NewTimelineEvent {
                workspace_id,
                file_id,
                event_type,
                occurred_at,
                metadata,
            })
            .await?;

        tracing::info!(
            workspace_id = %workspace_id,
            event_type = event.event_type.as_str(),
            file_id = ?file_id,
            "timeline event recorded"
        );

        Ok(event)
    }

    /// Finds the `files` row for `path` within `workspace_id`, creating
    /// one — as a generic `file` artifact; the watcher pipeline doesn't
    /// yet distinguish tabs/notes/screenshots from real filesystem files,
    /// only [`crate::watcher`] does that distinction, and Phase 3 only
    /// watches the filesystem — the first time this path is seen.
    async fn resolve_file(&self, workspace_id: Uuid, path: &str) -> Result<Uuid, DatabaseError> {
        if let Some(existing) = self
            .file_repository
            .find_by_workspace_and_path(workspace_id, path)
            .await?
        {
            return Ok(existing.id);
        }

        let created = self
            .file_repository
            .create(NewFile {
                workspace_id,
                artifact_type: ArtifactType::File,
                path_or_url: path.to_string(),
                content_hash: None,
            })
            .await?;

        Ok(created.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::CreateWorkspaceInput;
    use crate::repositories::WorkspaceRepository;

    async fn recorder_with_workspace() -> (TimelineRecorder, FileRepository, Uuid, tempfile::TempDir)
    {
        let (database, temp_dir) = test_database().await;
        let workspace_repo = WorkspaceRepository::new(database.pool().clone());
        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        let file_repository = FileRepository::new(database.pool().clone());
        let recorder = TimelineRecorder::new(
            file_repository.clone(),
            TimelineRepository::new(database.pool().clone()),
        );

        (recorder, file_repository, workspace.id, temp_dir)
    }

    #[tokio::test]
    async fn recording_file_created_creates_a_file_row() {
        let (recorder, file_repository, workspace_id, _guard) = recorder_with_workspace().await;

        recorder
            .record(
                workspace_id,
                TimelineActivity::FileCreated {
                    path: "/repo/src/main.rs".to_string(),
                },
                Utc::now(),
            )
            .await
            .expect("record should succeed");

        let files = file_repository
            .list_by_workspace(workspace_id)
            .await
            .unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path_or_url, "/repo/src/main.rs");
    }

    #[tokio::test]
    async fn recording_activity_for_the_same_path_twice_reuses_the_file_row() {
        let (recorder, file_repository, workspace_id, _guard) = recorder_with_workspace().await;

        recorder
            .record(
                workspace_id,
                TimelineActivity::FileCreated {
                    path: "/repo/src/main.rs".to_string(),
                },
                Utc::now(),
            )
            .await
            .unwrap();
        recorder
            .record(
                workspace_id,
                TimelineActivity::FileModified {
                    path: "/repo/src/main.rs".to_string(),
                },
                Utc::now(),
            )
            .await
            .unwrap();

        let files = file_repository
            .list_by_workspace(workspace_id)
            .await
            .unwrap();
        assert_eq!(
            files.len(),
            1,
            "the second event must not create a duplicate file row"
        );
    }

    #[tokio::test]
    async fn workspace_level_activity_records_with_no_file_id() {
        let (recorder, _file_repository, workspace_id, _guard) = recorder_with_workspace().await;

        let event = recorder
            .record(workspace_id, TimelineActivity::WorkspaceOpened, Utc::now())
            .await
            .unwrap();

        assert_eq!(event.file_id, None);
    }

    #[tokio::test]
    async fn recording_against_unknown_workspace_fails() {
        let (database, _guard) = test_database().await;
        let recorder = TimelineRecorder::new(
            FileRepository::new(database.pool().clone()),
            TimelineRepository::new(database.pool().clone()),
        );

        let result = recorder
            .record(
                Uuid::new_v4(),
                TimelineActivity::WorkspaceOpened,
                Utc::now(),
            )
            .await;

        assert!(matches!(result, Err(DatabaseError::Constraint(_))));
    }
}
