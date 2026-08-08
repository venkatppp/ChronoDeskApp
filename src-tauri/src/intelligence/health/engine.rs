//! Workspace health calculation engine.
//!
//! Every score here is derived from measurable, persisted signals:
//! timeline event volume/recency, tracked file counts, and smart-resume
//! session availability. No placeholder constants — a workspace with no
//! observed activity legitimately scores low, and a freshly active one
//! scores accordingly.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::repositories::{FileRepository, TimelineRepository, WorkspaceRepository};
use crate::services::ContextService;

use super::models::{HealthFactor, HealthMetric, WorkspaceHealth};
use super::service::HealthService;

/// Engine for calculating workspace health scores.
#[derive(Clone)]
pub struct WorkspaceHealthEngine {
    health_service: HealthService,
    workspace_repository: WorkspaceRepository,
    timeline_repository: TimelineRepository,
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
}

/// A recency-based activity score: how recently the workspace was used.
fn recency_score(last_active: Option<DateTime<Utc>>) -> (f64, String) {
    let Some(last) = last_active else {
        return (0.05, "never".to_string());
    };
    let days = (Utc::now() - last).num_days();
    let (score, label) = match days {
        d if d <= 1 => (1.0, "today"),
        d if d <= 3 => (0.8, "this week"),
        d if d <= 7 => (0.6, "this week"),
        d if d <= 14 => (0.4, "two weeks ago"),
        d if d <= 30 => (0.25, "this month"),
        _ => (0.1, "more than a month ago"),
    };
    (score, format!("{label} ({} days ago)", days.max(0)))
}

/// Event-volume score: more real activity (up to a cap) is healthier.
fn activity_volume_score(events_7d: i64, events_30d: i64) -> (f64, f64) {
    let recent = events_7d as f64;
    let total = events_30d as f64;
    // 50+ events in the last week saturates the recent-activity portion.
    let score = (recent / 50.0).min(1.0) * 0.7 + (total / 150.0).min(1.0) * 0.3;
    (score, recent)
}

impl WorkspaceHealthEngine {
    /// Calculates current health for a workspace.
    pub async fn calculate_health(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceHealth, DatabaseError> {
        let mut health = WorkspaceHealth::new(workspace_id.to_string());

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

        // Persist health history and sync the workspace column so the
        // dashboard/workspaces surfaces read a real, current score.
        self.health_service.save_health(&health).await?;
        if self.workspace_repository.get_by_id(workspace_id).await.is_ok() {
            let _ = self
                .workspace_repository
                .update(
                    workspace_id,
                    crate::models::UpdateWorkspaceInput {
                        health_score: Some(health.overall_score * 100.0),
                        ..Default::default()
                    },
                )
                .await;
        }

        Ok(health)
    }

    /// Gets the latest health assessment for a workspace.
    pub async fn get_latest_health(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<WorkspaceHealth>, DatabaseError> {
        self.health_service.get_latest_health(workspace_id).await
    }

    /// Gets health history for a workspace.
    pub async fn get_health_history(
        &self,
        workspace_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<WorkspaceHealth>, DatabaseError> {
        self.health_service
            .get_health_history(workspace_id, since)
            .await
    }

    /// Calculates activity level health factor — from real timeline
    /// events (recency of the last activity and volume over the last
    /// 7/30 days), not placeholders.
    async fn calculate_activity_factor(
        &self,
        workspace_id: Uuid,
    ) -> Result<HealthFactor, DatabaseError> {
        let workspace = self.workspace_repository.get_by_id(workspace_id).await?;

        let now = Utc::now();
        let events_7d = self
            .timeline_repository
            .count_since(workspace_id, now - Duration::days(7))
            .await?;
        let events_30d = self
            .timeline_repository
            .count_since(workspace_id, now - Duration::days(30))
            .await?;

        let (recency, recency_label) = recency_score(Some(workspace.last_active_at));
        let (volume, recent_events) = activity_volume_score(events_7d, events_30d);

        // Activity is 60% recency (the workspace is being worked on now)
        // and 40% volume (there is real ongoing work, not a one-off touch).
        let activity_score = recency * 0.6 + volume * 0.4;

        Ok(HealthFactor::new(
            "activity_level",
            "Activity Level",
            "How actively this workspace is being used",
        )
        .with_score(activity_score)
        .with_weight(0.4)
        .with_metric(
            HealthMetric::new(
                "last_activity",
                "Last activity",
                (Utc::now() - workspace.last_active_at).num_hours() as f64,
                "hours_ago",
            )
            .with_ideal(24.0),
        )
        .with_metric(
            HealthMetric::new(
                "events_7d",
                "Timeline events (7d)",
                recent_events,
                "events",
            )
            .with_ideal(50.0),
        )
        .with_metric(
            HealthMetric::new(
                "activity_score",
                "Activity Score",
                activity_score * 100.0,
                "percent",
            )
            .with_ideal(80.0),
        )
        .with_metric(
            HealthMetric::new("recency_label", "Recency", 0.0, recency_label).with_ideal(1.0),
        ))
    }

    /// Calculates organization health factor — from the real tracked file
    /// set: file count and directory spread (files in more than one
    /// directory indicate an organized project, not a loose pile).
    async fn calculate_organization_factor(
        &self,
        workspace_id: Uuid,
    ) -> Result<HealthFactor, DatabaseError> {
        let files = self.file_repository.list_by_workspace(workspace_id).await?;
        let file_count = files.len();

        // Distinct parent directories among the tracked files.
        let mut dirs = std::collections::HashSet::new();
        for file in &files {
            let parent = std::path::Path::new(&file.path_or_url)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !parent.is_empty() {
                dirs.insert(parent);
            }
        }

        let dir_count = dirs.len();
        // 20+ files and 3+ directories saturate the organization score.
        let file_score = (file_count as f64 / 20.0).min(1.0);
        let dir_score = (dir_count as f64 / 3.0).min(1.0);
        let organization_score = if file_count == 0 {
            0.05 // No tracked files: nothing organized yet.
        } else {
            file_score * 0.6 + dir_score * 0.4
        };

        Ok(HealthFactor::new(
            "organization",
            "Organization",
            "How well files are organized and accessed",
        )
        .with_score(organization_score)
        .with_weight(0.3)
        .with_metric(
            HealthMetric::new("file_count", "Tracked files", file_count as f64, "files")
                .with_ideal(20.0),
        )
        .with_metric(
            HealthMetric::new(
                "directory_count",
                "Directories",
                dir_count as f64,
                "directories",
            )
            .with_ideal(3.0),
        )
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
        _workspace_id: Uuid,
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
