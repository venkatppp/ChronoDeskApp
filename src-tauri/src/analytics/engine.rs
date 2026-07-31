//! Analytics Engine
//!
//! High-level analytics API that provides daily briefings, activity summaries,
//! and workspace insights. This is the facade that commands interact with.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use uuid::Uuid;

use crate::analytics::models::{
    ActivitySummary, DailyBriefing, DailySummary, MonthlySummary, WeeklySummary, WorkspaceInsight,
};
use crate::analytics::service::AnalyticsService;
use crate::errors::DatabaseError;

/// Analytics Engine: high-level analytics API.
///
/// Provides daily briefings, summaries, and insights for the dashboard
/// and analytics views.
#[derive(Debug, Clone)]
pub struct AnalyticsEngine {
    analytics_service: AnalyticsService,
}

impl AnalyticsEngine {
    pub fn new(analytics_service: AnalyticsService) -> Self {
        Self { analytics_service }
    }

    /// Gets daily briefing for dashboard display.
    ///
    /// Includes greeting, today's summary, most active workspace,
    /// longest focus session, and intelligent suggestions.
    pub async fn get_daily_briefing(&self) -> Result<DailyBriefing, DatabaseError> {
        let now = Utc::now();
        let daily_summary = self.analytics_service.get_daily_summary(now).await?;

        // Generate greeting based on time of day
        let hour = now.time().hour();
        let greeting = match hour {
            5..=11 => "Good morning",
            12..=16 => "Good afternoon",
            17..=21 => "Good evening",
            _ => "Good night",
        }
        .to_string();

        // Build activity summary
        let summary = ActivitySummary {
            time_range: "Today".to_string(),
            duration_seconds: daily_summary.total_duration_seconds,
            session_count: daily_summary.session_count,
            workspace_count: daily_summary.workspace_count,
            file_count: daily_summary.file_count,
            edit_count: daily_summary.edit_count,
            commit_count: daily_summary.commit_count,
            primary_language: daily_summary.languages.first().map(|l| l.language.clone()),
        };

        // Generate insights
        let mut insights = Vec::new();

        if daily_summary.total_duration_seconds > 0 {
            let hours = daily_summary.total_duration_seconds / 3600;
            let minutes = (daily_summary.total_duration_seconds % 3600) / 60;
            if hours > 0 {
                insights.push(format!("{}h {}m of focused work today", hours, minutes));
            } else {
                insights.push(format!("{}m of focused work today", minutes));
            }
        }

        if let Some(longest) = daily_summary.longest_session_duration {
            let minutes = longest / 60;
            if minutes > 45 {
                insights.push(format!("Excellent focus: {} minute session", minutes));
            }
        }

        if daily_summary.commit_count > 0 {
            insights.push(format!(
                "{} commit{} completed",
                daily_summary.commit_count,
                if daily_summary.commit_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        // Generate suggestions
        let mut suggestions = Vec::new();

        if let Some(ref workspace) = daily_summary.most_active_workspace {
            suggestions.push(format!("Continue working on {}", workspace.workspace_name));
        }

        if daily_summary.workspace_count > 3 {
            suggestions.push("Consider focusing on fewer workspaces".to_string());
        }

        if daily_summary.total_duration_seconds == 0 {
            suggestions.push("Start a new work session".to_string());
        }

        Ok(DailyBriefing {
            greeting,
            summary,
            most_active_workspace: daily_summary.most_active_workspace,
            longest_focus_session: daily_summary.longest_session_duration,
            primary_language: daily_summary.languages.first().map(|l| l.language.clone()),
            insights,
            suggestions,
        })
    }

    /// Gets daily summary for today.
    pub async fn get_today_summary(&self) -> Result<DailySummary, DatabaseError> {
        self.analytics_service.get_daily_summary(Utc::now()).await
    }

    /// Gets daily summary for yesterday.
    pub async fn get_yesterday_summary(&self) -> Result<DailySummary, DatabaseError> {
        let yesterday = Utc::now() - Duration::days(1);
        self.analytics_service.get_daily_summary(yesterday).await
    }

    /// Gets weekly summary for current week.
    pub async fn get_this_week_summary(&self) -> Result<WeeklySummary, DatabaseError> {
        let now = Utc::now();
        let days_from_monday = now.weekday().num_days_from_monday() as i64;
        let week_start = now - Duration::days(days_from_monday);
        self.analytics_service.get_weekly_summary(week_start).await
    }

    /// Gets weekly summary for last week.
    pub async fn get_last_week_summary(&self) -> Result<WeeklySummary, DatabaseError> {
        let now = Utc::now();
        let days_from_monday = now.weekday().num_days_from_monday() as i64;
        let this_week_start = now - Duration::days(days_from_monday);
        let last_week_start = this_week_start - Duration::days(7);
        self.analytics_service
            .get_weekly_summary(last_week_start)
            .await
    }

    /// Gets monthly summary for current month.
    pub async fn get_this_month_summary(&self) -> Result<MonthlySummary, DatabaseError> {
        let now = Utc::now();
        let naive_date = now.date_naive();
        let year = naive_date.year();
        let month = naive_date.month();
        let month_start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        self.analytics_service
            .get_monthly_summary(month_start_date)
            .await
    }

    /// Gets comprehensive workspace insight.
    pub async fn get_workspace_insight(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceInsight, DatabaseError> {
        self.analytics_service
            .get_workspace_insight(workspace_id)
            .await
    }

    /// Gets activity summary for a custom time range.
    pub async fn get_activity_summary(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ActivitySummary, DatabaseError> {
        let daily_summary = self.analytics_service.get_daily_summary(start).await?;

        Ok(ActivitySummary {
            time_range: format!("{} to {}", start.format("%Y-%m-%d"), end.format("%Y-%m-%d")),
            duration_seconds: daily_summary.total_duration_seconds,
            session_count: daily_summary.session_count,
            workspace_count: daily_summary.workspace_count,
            file_count: daily_summary.file_count,
            edit_count: daily_summary.edit_count,
            commit_count: daily_summary.commit_count,
            primary_language: daily_summary.languages.first().map(|l| l.language.clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::repository::AnalyticsRepository;
    use crate::database::test_database;
    use crate::models::CreateWorkspaceInput;
    use crate::repositories::{
        FileRepository, SettingsRepository, TimelineRepository, WorkspaceRepository,
    };
    use crate::services::ContextService;
    use crate::session::SessionEngine;

    async fn setup() -> (AnalyticsEngine, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();

        let workspace_repo = WorkspaceRepository::new(pool.clone());
        let timeline_repo = TimelineRepository::new(pool.clone());
        let file_repo = FileRepository::new(pool.clone());
        let settings_repo = SettingsRepository::new(pool.clone());
        let analytics_repo = AnalyticsRepository::new(pool.clone());

        let session_engine = SessionEngine::new(timeline_repo, file_repo.clone());
        let context_service =
            ContextService::new(session_engine, workspace_repo.clone(), settings_repo);

        let analytics_service =
            AnalyticsService::new(analytics_repo, context_service, workspace_repo, file_repo);

        let engine = AnalyticsEngine::new(analytics_service);

        (engine, temp_dir)
    }

    #[tokio::test]
    async fn get_daily_briefing_returns_greeting() {
        let (engine, _guard) = setup().await;

        let briefing = engine.get_daily_briefing().await.unwrap();

        assert!(!briefing.greeting.is_empty());
        assert!(
            briefing.greeting.contains("Good morning")
                || briefing.greeting.contains("Good afternoon")
                || briefing.greeting.contains("Good evening")
                || briefing.greeting.contains("Good night")
        );
    }

    #[tokio::test]
    async fn get_today_summary_returns_zero_for_no_activity() {
        let (engine, _guard) = setup().await;

        let summary = engine.get_today_summary().await.unwrap();

        assert_eq!(summary.total_duration_seconds, 0);
        assert_eq!(summary.session_count, 0);
        assert_eq!(summary.edit_count, 0);
    }
}
