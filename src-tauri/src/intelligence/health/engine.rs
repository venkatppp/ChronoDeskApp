//! Workspace health calculation engine.

use chrono::{DateTime, Utc};

use crate::errors::DatabaseError;
use crate::repositories::{FileRepository, TimelineRepository, WorkspaceRepository};
use crate::services::ContextService;

use super::models::{HealthFactor, HealthMetric, WorkspaceHealth};
use super::service::HealthService;

/// Engine for calculating workspace health scores.
#[derive(Clone)]
pub struct WorkspaceHealthEngine {
    health_service: HealthService,
    #[allow(dead_code)]
    workspace_repository: WorkspaceRepository,
    #[allow(dead_code)]
    timeline_repository: TimelineRepository,
    #[allow(dead_code)]
    file_repository: FileRepository,
    context_service: ContextService,
}

impl WorkspaceHealthEngine {
    /// Creates a new workspace health engine.
    pub fn new(
        health_service: HealthService,
        workspace_repository: WorkspaceRepository,
        timeline_repository: TimelineRepository,
        file_repository: FileRepository,
        context_service: ContextService,
    ) -> Self {
        Self {
            health_service,
            workspace_repository,
            timeline_repository,
            file_repository,
            context_service,
        }
    }

    /// Calculates current health for a workspace.
    pub async fn calculate_health(
        &self,
        workspace_id: i64,
    ) -> Result<WorkspaceHealth, DatabaseError> {
        // Convert i64 to Uuid (workspace_id stored as i64 in health table, but Uuid in main tables)
        // For now, we'll work with the data we have
        let mut health = WorkspaceHealth::new(workspace_id);

        // Calculate individual health factors
        let activity_factor = self.calculate_activity_factor(workspace_id).await?;
        let organization_factor = self.calculate_organization_factor(workspace_id).await?;
        let context_factor = self.calculate_context_factor(workspace_id).await?;

        health = health
            .with_factor(activity_factor)
            .with_factor(organization_factor)
            .with_factor(context_factor);

        // Calculate overall score from weighted factors
        health.calculate_overall_score();

        // Calculate trend
        if let Ok(Some(trend)) = self
            .health_service
            .calculate_trend(workspace_id, health.overall_score)
            .await
        {
            health = health.with_trend(trend);
        }

        // Persist health history
        self.health_service.save_health(&health).await?;

        Ok(health)
    }

    /// Gets the latest health assessment for a workspace.
    pub async fn get_latest_health(
        &self,
        workspace_id: i64,
    ) -> Result<Option<WorkspaceHealth>, DatabaseError> {
        self.health_service.get_latest_health(workspace_id).await
    }

    /// Gets health history for a workspace.
    pub async fn get_health_history(
        &self,
        workspace_id: i64,
        since: DateTime<Utc>,
    ) -> Result<Vec<WorkspaceHealth>, DatabaseError> {
        self.health_service
            .get_health_history(workspace_id, since)
            .await
    }

    /// Calculates activity level health factor.
    async fn calculate_activity_factor(
        &self,
        _workspace_id: i64,
    ) -> Result<HealthFactor, DatabaseError> {
        // For now, return a placeholder factor
        // TODO: Implement proper activity tracking based on timeline events
        let activity_score = 0.7; // Placeholder

        Ok(HealthFactor::new(
            "activity_level",
            "Activity Level",
            "How actively this workspace is being used",
        )
        .with_score(activity_score)
        .with_weight(0.4)
        .with_metric(
            HealthMetric::new(
                "activity_score",
                "Activity Score",
                activity_score * 100.0,
                "percent",
            )
            .with_ideal(80.0),
        ))
    }

    /// Calculates organization health factor.
    async fn calculate_organization_factor(
        &self,
        _workspace_id: i64,
    ) -> Result<HealthFactor, DatabaseError> {
        // Placeholder - calculate based on file organization patterns
        let organization_score = 0.75;

        Ok(HealthFactor::new(
            "organization",
            "Organization",
            "How well files are organized and accessed",
        )
        .with_score(organization_score)
        .with_weight(0.3)
        .with_metric(
            HealthMetric::new(
                "organization_score",
                "Organization Score",
                organization_score * 100.0,
                "percent",
            )
            .with_ideal(80.0),
        ))
    }

    /// Calculates context health factor (how well context is maintained).
    async fn calculate_context_factor(
        &self,
        _workspace_id: i64,
    ) -> Result<HealthFactor, DatabaseError> {
        // Get smart resume session (no workspace_id parameter)
        let session = self.context_service.get_smart_resume_session().await?;

        // Score based on whether we have a recent session
        let context_score = if session.is_some() {
            0.8 // Good context if we have a recent session
        } else {
            0.3 // Low context if no recent session
        };

        Ok(HealthFactor::new(
            "context",
            "Context Quality",
            "How well context is maintained between sessions",
        )
        .with_score(context_score)
        .with_weight(0.3)
        .with_metric(
            HealthMetric::new(
                "context_score",
                "Context Score",
                context_score * 100.0,
                "percent",
            )
            .with_ideal(80.0),
        ))
    }
}
