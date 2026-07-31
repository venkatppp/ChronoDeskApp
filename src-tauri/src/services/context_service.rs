//! Context Service
//!
//! High-level intelligence API that sits above SessionEngine. This service
//! is the single entry point for all context intelligence features:
//! Smart Resume, session analytics, and future intelligence capabilities.
//!
//! Commands should interact with ContextService, not SessionEngine directly.

use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::repositories::{SettingsRepository, WorkspaceRepository};
use crate::session::detector::DEFAULT_INACTIVITY_THRESHOLD_SECONDS;
use crate::session::engine::SessionEngine;
use crate::session::types::{Session, SessionSummary};

/// Settings key for the session inactivity threshold.
const SETTING_KEY_INACTIVITY_THRESHOLD: &str = "session_inactivity_threshold_seconds";

/// Context Service: high-level intelligence API.
///
/// Provides Smart Resume and other context intelligence features by
/// orchestrating SessionEngine and repository access. This is the layer
/// that commands interact with.
#[derive(Debug, Clone)]
pub struct ContextService {
    session_engine: SessionEngine,
    workspace_repository: WorkspaceRepository,
    settings_repository: SettingsRepository,
}

impl ContextService {
    /// Creates a new ContextService.
    pub fn new(
        session_engine: SessionEngine,
        workspace_repository: WorkspaceRepository,
        settings_repository: SettingsRepository,
    ) -> Self {
        Self {
            session_engine,
            workspace_repository,
            settings_repository,
        }
    }

    /// Gets the configured inactivity threshold from settings.
    ///
    /// Falls back to the default (30 minutes) if no setting exists.
    async fn get_inactivity_threshold(&self) -> i64 {
        match self
            .settings_repository
            .get(SETTING_KEY_INACTIVITY_THRESHOLD)
            .await
        {
            Ok(Some(value)) => value
                .parse()
                .unwrap_or(DEFAULT_INACTIVITY_THRESHOLD_SECONDS),
            _ => DEFAULT_INACTIVITY_THRESHOLD_SECONDS,
        }
    }

    /// Gets the most recent session for Smart Resume.
    ///
    /// Returns the most recently active session across all workspaces,
    /// enriched with workspace metadata for display. Returns None if
    /// no recent sessions exist.
    pub async fn get_smart_resume_session(&self) -> Result<Option<SessionSummary>, DatabaseError> {
        let threshold = self.get_inactivity_threshold().await;

        let session = self
            .session_engine
            .get_most_recent_active_session(Some(threshold))
            .await?;

        if let Some(session) = session {
            // Fetch workspace name
            let workspace = self
                .workspace_repository
                .get_by_id(session.workspace_id)
                .await?;

            let summary = self
                .session_engine
                .get_session_summary(&session, workspace.name)
                .await?;

            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    /// Gets recent sessions for a specific workspace.
    ///
    /// Used for workspace analytics and session history views.
    pub async fn get_workspace_sessions(
        &self,
        workspace_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<Session>, DatabaseError> {
        let threshold = self.get_inactivity_threshold().await;

        self.session_engine
            .detect_sessions(workspace_id, Some(threshold), limit)
            .await
    }

    /// Gets the latest session for a specific workspace with full details.
    pub async fn get_latest_workspace_session(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<SessionSummary>, DatabaseError> {
        let threshold = self.get_inactivity_threshold().await;

        let session = self
            .session_engine
            .get_latest_session(workspace_id, Some(threshold))
            .await?;

        if let Some(session) = session {
            let workspace = self.workspace_repository.get_by_id(workspace_id).await?;

            let summary = self
                .session_engine
                .get_session_summary(&session, workspace.name)
                .await?;

            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    /// Updates the session inactivity threshold setting.
    ///
    /// This allows users to customize what constitutes a "break" between
    /// sessions (e.g. 15 minutes, 30 minutes, 60 minutes).
    pub async fn set_inactivity_threshold(
        &self,
        threshold_seconds: i64,
    ) -> Result<(), DatabaseError> {
        if !(60..=3600 * 4).contains(&threshold_seconds) {
            return Err(DatabaseError::InvalidInput(
                "Inactivity threshold must be between 1 minute and 4 hours".to_string(),
            ));
        }

        self.settings_repository
            .set(
                SETTING_KEY_INACTIVITY_THRESHOLD,
                &threshold_seconds.to_string(),
            )
            .await
    }

    /// Gets the current inactivity threshold setting.
    pub async fn get_inactivity_threshold_setting(&self) -> Result<i64, DatabaseError> {
        Ok(self.get_inactivity_threshold().await)
    }

    /// Gets all files for a workspace.
    pub async fn get_workspace_files(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<crate::models::FileArtifact>, DatabaseError> {
        self.session_engine
            .file_repository
            .list_by_workspace(workspace_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::{CreateWorkspaceInput, NewTimelineEvent, TimelineEventType};
    use crate::repositories::{FileRepository, TimelineRepository};
    use crate::session::engine::SessionEngine;
    use chrono::Utc;

    async fn setup() -> (ContextService, Uuid, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();

        let workspace_repo = WorkspaceRepository::new(pool.clone());
        let timeline_repo = TimelineRepository::new(pool.clone());
        let file_repo = FileRepository::new(pool.clone());
        let settings_repo = SettingsRepository::new(pool.clone());

        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        let session_engine = SessionEngine::new(timeline_repo.clone(), file_repo);
        let service = ContextService::new(session_engine, workspace_repo, settings_repo);

        (service, workspace.id, temp_dir)
    }

    #[tokio::test]
    async fn get_smart_resume_session_returns_none_when_no_events() {
        let (service, _workspace_id, _guard) = setup().await;

        let result = service.get_smart_resume_session().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_smart_resume_session_returns_latest_session() {
        let (_service, _workspace_id, _guard) = setup().await;

        // Get timeline repository from the same database
        let (database, _temp_dir) = test_database().await;
        let workspace_repo = WorkspaceRepository::new(database.pool().clone());
        let timeline_repo = TimelineRepository::new(database.pool().clone());

        // Create workspace in the new database instance
        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace 2".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        // Create timeline event
        timeline_repo
            .create(NewTimelineEvent {
                workspace_id: workspace.id,
                file_id: None,
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: None,
            })
            .await
            .unwrap();

        // Create new service with the same database
        let file_repo = FileRepository::new(database.pool().clone());
        let settings_repo = SettingsRepository::new(database.pool().clone());
        let session_engine = SessionEngine::new(timeline_repo.clone(), file_repo);
        let test_service = ContextService::new(session_engine, workspace_repo, settings_repo);

        let result = test_service.get_smart_resume_session().await.unwrap();
        assert!(result.is_some());

        let summary = result.unwrap();
        assert_eq!(summary.workspace_id, workspace.id);
        assert_eq!(summary.workspace_name, "Test Workspace 2");
        assert!(summary.productivity_score >= 0.0);
    }

    #[tokio::test]
    async fn set_inactivity_threshold_persists_setting() {
        let (service, _workspace_id, _guard) = setup().await;

        service.set_inactivity_threshold(1800).await.unwrap(); // 30 minutes

        let threshold = service.get_inactivity_threshold_setting().await.unwrap();
        assert_eq!(threshold, 1800);
    }

    #[tokio::test]
    async fn set_inactivity_threshold_rejects_invalid_values() {
        let (service, _workspace_id, _guard) = setup().await;

        // Too short
        let result = service.set_inactivity_threshold(30).await;
        assert!(result.is_err());

        // Too long
        let result = service.set_inactivity_threshold(20000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn default_threshold_is_used_when_no_setting_exists() {
        let (service, _workspace_id, _guard) = setup().await;

        let threshold = service.get_inactivity_threshold().await;
        assert_eq!(threshold, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);
    }
}
