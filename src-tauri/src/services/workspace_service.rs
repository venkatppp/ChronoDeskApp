use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::{
    CreateWorkspaceInput, NewTimelineEvent, TimelineEventType, UpdateWorkspaceInput, Workspace,
    WorkspaceStats, WorkspaceStatus,
};
use crate::repositories::{TimelineRepository, WorkspaceRepository};

/// Business logic for workspaces. The only thing `commands::workspace`
/// talks to — command handlers never import a repository directly.
///
/// Holds both [`WorkspaceRepository`] and [`TimelineRepository`] because
/// several workspace operations have a timeline side effect (creating or
/// opening a workspace is itself an event worth recording); coordinating
/// two repositories for one user-facing action is exactly what belongs
/// in a service rather than in either repository.
#[derive(Debug, Clone)]
pub struct WorkspaceService {
    workspace_repository: WorkspaceRepository,
    timeline_repository: TimelineRepository,
}

impl WorkspaceService {
    pub fn new(
        workspace_repository: WorkspaceRepository,
        timeline_repository: TimelineRepository,
    ) -> Self {
        Self {
            workspace_repository,
            timeline_repository,
        }
    }

    /// Lists every active workspace, most recently active first — the
    /// dashboard's "Active workspaces" grid (blueprint §3.2).
    pub async fn list_active_workspaces(&self) -> Result<Vec<Workspace>, DatabaseError> {
        self.workspace_repository.list_active_workspaces().await
    }

    /// Lists every archived workspace, most recently active first —
    /// backs the "Archived" filter tab on the Workspaces screen.
    pub async fn list_archived_workspaces(&self) -> Result<Vec<Workspace>, DatabaseError> {
        self.workspace_repository.list_archived_workspaces().await
    }

    /// Fetches a single workspace by id.
    pub async fn get_workspace(&self, id: Uuid) -> Result<Workspace, DatabaseError> {
        self.workspace_repository.get_by_id(id).await
    }

    /// Aggregated statistics for a workspace — file count, timeline event
    /// count, recency, and health score — in one round-trip.
    pub async fn get_workspace_stats(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceStats, DatabaseError> {
        self.workspace_repository
            .get_workspace_stats(workspace_id)
            .await
    }

    /// Looks up the workspace matching a filesystem root path, if any.
    /// Used by [`crate::workspace::manager::WorkspaceManager`] to decide
    /// "existing workspace or new one" for a directory the detector just
    /// identified.
    pub async fn find_by_root_path(
        &self,
        root_path: &str,
    ) -> Result<Option<Workspace>, DatabaseError> {
        self.workspace_repository.find_by_root_path(root_path).await
    }

    /// Creates a workspace and records its creation as the first entry in
    /// its own timeline, so the Timeline screen's history (blueprint §10)
    /// starts on day one instead of on whatever the first *file* event
    /// happens to be.
    pub async fn create_workspace(
        &self,
        input: CreateWorkspaceInput,
    ) -> Result<Workspace, DatabaseError> {
        let workspace = self.workspace_repository.create(input).await?;

        self.timeline_repository
            .create(NewTimelineEvent {
                workspace_id: workspace.id,
                file_id: None,
                event_type: TimelineEventType::WorkspaceSwitch,
                occurred_at: workspace.created_at,
                metadata: Some(serde_json::json!({ "reason": "workspace_created" })),
            })
            .await?;

        Ok(workspace)
    }

    /// Applies a partial edit (name/description/status/health_score).
    /// For simply "the user opened this workspace" (no field changes),
    /// use [`WorkspaceService::open_workspace`] instead — it carries a
    /// different business rule (implicit reactivation) that a generic
    /// field edit should not.
    pub async fn update_workspace(
        &self,
        id: Uuid,
        input: UpdateWorkspaceInput,
    ) -> Result<Workspace, DatabaseError> {
        self.workspace_repository.update(id, input).await
    }

    /// Permanently deletes a workspace and, via the schema's cascading
    /// foreign keys, every file/timeline/tag/relationship row that
    /// referenced it.
    pub async fn delete_workspace(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.workspace_repository.delete(id).await
    }

    /// Marks a workspace as just-opened.
    ///
    /// Business rule: opening an **archived** workspace implicitly moves
    /// it back to `active` — a user restoring an old project is exactly
    /// the "resume" signal that should bring it back into the active set,
    /// without a separate "unarchive" action. A repository, which only
    /// knows how to write the columns it's told to, has no basis for that
    /// decision; it belongs here, one level up.
    ///
    /// Also records a `workspace_switch` timeline event — but only when
    /// the workspace actually changed from the previously active one
    /// (i.e. the latest recorded switch targeted a different workspace).
    /// The file-watcher pipeline calls `open_workspace` on every file
    /// touch, so unconditionally recording a switch there would drown the
    /// Timeline in duplicate "Switched workspace" rows for one continuous
    /// working session.
    pub async fn open_workspace(&self, id: Uuid) -> Result<Workspace, DatabaseError> {
        let current = self.workspace_repository.get_by_id(id).await?;

        if current.status == WorkspaceStatus::Archived {
            self.workspace_repository
                .update(
                    id,
                    UpdateWorkspaceInput {
                        status: Some(WorkspaceStatus::Active),
                        ..Default::default()
                    },
                )
                .await?;
        }

        let workspace = self.workspace_repository.touch_last_active(id).await?;

        let record_switch = match self.timeline_repository.latest_workspace_switch().await? {
            Some(latest) => latest.workspace_id != id,
            None => true,
        };

        if record_switch {
            self.timeline_repository
                .create(NewTimelineEvent {
                    workspace_id: id,
                    file_id: None,
                    event_type: TimelineEventType::WorkspaceSwitch,
                    occurred_at: workspace.last_active_at,
                    metadata: None,
                })
                .await?;
        }

        Ok(workspace)
    }

    pub async fn switch_workspace(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.open_workspace(id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    async fn service() -> (WorkspaceService, TimelineRepository, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let timeline_repository = TimelineRepository::new(database.pool().clone());
        let service = WorkspaceService::new(
            WorkspaceRepository::new(database.pool().clone()),
            timeline_repository.clone(),
        );
        (service, timeline_repository, temp_dir)
    }

    #[tokio::test]
    async fn create_workspace_records_a_timeline_event() {
        let (service, timeline_repository, _guard) = service().await;

        let workspace = service
            .create_workspace(CreateWorkspaceInput {
                name: "Pricing Model Q3".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .expect("create should succeed");

        let events = timeline_repository
            .list_by_workspace(workspace.id, None)
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TimelineEventType::WorkspaceSwitch);
    }

    #[tokio::test]
    async fn open_workspace_reactivates_an_archived_workspace() {
        let (service, timeline_repository, _guard) = service().await;

        let workspace = service
            .create_workspace(CreateWorkspaceInput {
                name: "Thesis Draft".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        service
            .update_workspace(
                workspace.id,
                UpdateWorkspaceInput {
                    status: Some(WorkspaceStatus::Archived),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let reopened = service.open_workspace(workspace.id).await.unwrap();

        assert_eq!(reopened.status, WorkspaceStatus::Active);

        // create_workspace recorded the first switch; re-opening the same
        // workspace must NOT append a duplicate switch event — the
        // workspace didn't change, so the timeline stays noise-free.
        let events = timeline_repository
            .list_by_workspace(workspace.id, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn open_workspace_leaves_an_already_active_workspace_active() {
        let (service, _timeline_repository, _guard) = service().await;

        let workspace = service
            .create_workspace(CreateWorkspaceInput {
                name: "ChronoDesk Dev".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        let reopened = service.open_workspace(workspace.id).await.unwrap();
        assert_eq!(reopened.status, WorkspaceStatus::Active);
    }
}
