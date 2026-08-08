//! Workspace Engine orchestration (blueprint §4.2): ties the
//! boundary-detection heuristics in [`super::detector`] to persistence via
//! [`WorkspaceService`]. This is the module the file watcher pipeline
//! calls on every relevant filesystem event.

use std::path::Path;

use crate::errors::DatabaseError;
use crate::models::{CreateWorkspaceInput, Workspace};
use crate::services::WorkspaceService;

use super::detector::{self, DetectedWorkspaceRoot};

/// Automatic workspace lifecycle management: given filesystem activity,
/// finds the workspace it belongs to (creating one if this is the first
/// activity ever seen under that root) and marks it as just-opened.
///
/// Holds a [`WorkspaceService`] rather than a raw repository — every
/// workspace this manager touches should also get the timeline side
/// effects (`workspace_switch` events) that only the service layer
/// knows to record; a manager that reached past the service into
/// `WorkspaceRepository` directly would silently lose that trail.
#[derive(Debug, Clone)]
pub struct WorkspaceManager {
    workspace_service: WorkspaceService,
}

impl WorkspaceManager {
    pub fn new(workspace_service: WorkspaceService) -> Self {
        Self { workspace_service }
    }

    /// Given a file path that just had activity under `watch_root`,
    /// finds or creates the workspace it belongs to and marks it opened.
    ///
    /// Returns `Ok(None)` — not an error — if no ancestor directory
    /// within `watch_root` clears the detection threshold (blueprint
    /// §2.2's heuristics in [`super::heuristics`]), e.g. a stray file
    /// sitting directly in a watched root with no project markers
    /// anywhere above it. The watcher pipeline should simply skip
    /// recording a timeline event in that case rather than inventing a
    /// workspace for every loose file.
    pub async fn resolve_workspace_for_path(
        &self,
        file_path: &Path,
        watch_root: &Path,
    ) -> Result<Option<Workspace>, DatabaseError> {
        let Some(detected) = detector::detect_workspace_root(file_path, watch_root) else {
            return Ok(None);
        };

        self.find_or_create_workspace(&detected).await.map(Some)
    }

    /// Finds the existing workspace for an already-detected root, or
    /// creates one. Either way, the returned workspace has just been
    /// "opened" (`last_active_at` bumped — see
    /// [`WorkspaceService::open_workspace`]): any file activity within a
    /// workspace's root is evidence the workspace is currently being
    /// worked on. Only the first detection of a brand-new root appends a
    /// timeline `workspace_switch` (its creation event); every later file
    /// touch records a file event, not a switch — switches are
    /// user-driven, so only [`WorkspaceService::switch_workspace`] records
    /// them.
    pub async fn find_or_create_workspace(
        &self,
        detected: &DetectedWorkspaceRoot,
    ) -> Result<Workspace, DatabaseError> {
        let root_path = detected.path.to_string_lossy().into_owned();

        if let Some(existing) = self.workspace_service.find_by_root_path(&root_path).await? {
            tracing::debug!(workspace_id = %existing.id, root_path, "matched existing workspace");
            return self.workspace_service.open_workspace(existing.id).await;
        }

        tracing::info!(
            root_path,
            markers = ?detected.markers,
            suggested_name = detected.suggested_name,
            "detected new workspace root"
        );

        let workspace = self
            .workspace_service
            .create_workspace(CreateWorkspaceInput {
                name: detected.suggested_name.clone(),
                description: None,
                root_path: Some(root_path),
            })
            .await?;

        tracing::info!(workspace_id = %workspace.id, "workspace auto-created from detected root");

        self.workspace_service.open_workspace(workspace.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::repositories::{TimelineRepository, WorkspaceRepository};
    use std::fs;
    use tempfile::tempdir;

    async fn manager() -> (WorkspaceManager, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let service = WorkspaceService::new(
            WorkspaceRepository::new(database.pool().clone()),
            TimelineRepository::new(database.pool().clone()),
        );
        (WorkspaceManager::new(service), temp_dir)
    }

    #[tokio::test]
    async fn creates_a_workspace_on_first_detection() {
        let (manager, _db_guard) = manager().await;
        let watch_root = tempdir().unwrap();
        fs::create_dir(watch_root.path().join(".git")).unwrap();
        let file_path = watch_root.path().join("src/main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "").unwrap();

        let workspace = manager
            .resolve_workspace_for_path(&file_path, watch_root.path())
            .await
            .expect("resolve should succeed")
            .expect("a git repo root should be detected");

        assert_eq!(
            workspace.root_path.as_deref(),
            Some(watch_root.path().to_string_lossy().as_ref())
        );
    }

    #[tokio::test]
    async fn reuses_the_same_workspace_for_repeated_activity() {
        let (manager, _db_guard) = manager().await;
        let watch_root = tempdir().unwrap();
        fs::write(watch_root.path().join("package.json"), "{}").unwrap();
        let file_a = watch_root.path().join("src/a.js");
        let file_b = watch_root.path().join("src/b.js");
        fs::create_dir_all(file_a.parent().unwrap()).unwrap();
        fs::write(&file_a, "").unwrap();
        fs::write(&file_b, "").unwrap();

        let first = manager
            .resolve_workspace_for_path(&file_a, watch_root.path())
            .await
            .unwrap()
            .unwrap();
        let second = manager
            .resolve_workspace_for_path(&file_b, watch_root.path())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            first.id, second.id,
            "two files under the same detected root must resolve to the same workspace"
        );
    }

    #[tokio::test]
    async fn returns_none_for_a_loose_file_with_no_project_markers() {
        let (manager, _db_guard) = manager().await;
        let watch_root = tempdir().unwrap();
        let loose_file = watch_root.path().join("notes.txt");
        fs::write(&loose_file, "").unwrap();

        let result = manager
            .resolve_workspace_for_path(&loose_file, watch_root.path())
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
