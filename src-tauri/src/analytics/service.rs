//! Analytics Service
//!
//! Transforms raw analytics data into meaningful summaries and insights.
//! Orchestrates AnalyticsRepository and ContextService to produce
//! daily/weekly/monthly summaries with trends and recommendations.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::analytics::models::{
    DailySummary, LanguageUsage, MonthlySummary, TrendIndicator, WeeklySummary,
    WorkspaceDaySummary, WorkspaceInsight,
};
use crate::analytics::repository::AnalyticsRepository;
use crate::errors::DatabaseError;
use crate::repositories::{FileRepository, WorkspaceRepository};
use crate::services::ContextService;

/// Analytics Service: transforms raw data into insights.
#[derive(Debug, Clone)]
pub struct AnalyticsService {
    analytics_repository: AnalyticsRepository,
    context_service: ContextService,
    workspace_repository: WorkspaceRepository,
    file_repository: FileRepository,
}

impl AnalyticsService {
    pub fn new(
        analytics_repository: AnalyticsRepository,
        context_service: ContextService,
        workspace_repository: WorkspaceRepository,
        file_repository: FileRepository,
    ) -> Self {
        Self {
            analytics_repository,
            context_service,
            workspace_repository,
            file_repository,
        }
    }

    /// Gets daily summary for a specific date.
    pub async fn get_daily_summary(
        &self,
        date: DateTime<Utc>,
    ) -> Result<DailySummary, DatabaseError> {
        let start = date.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = start + Duration::days(1);

        let counts = self.analytics_repository.count_events(start, end).await?;
        let file_count = self
            .analytics_repository
            .count_distinct_files(start, end)
            .await?;
        let workspace_count = self
            .analytics_repository
            .count_distinct_workspaces(start, end)
            .await?;

        // Get workspace activities
        let workspace_activities = self
            .analytics_repository
            .get_workspace_activity(start, end, 10)
            .await?;

        // Get most active workspace
        let most_active_workspace = if let Some(activity) = workspace_activities.first() {
            let workspace = self
                .workspace_repository
                .get_by_id(activity.workspace_id)
                .await?;

            // Get sessions for this workspace
            let sessions = self
                .context_service
                .get_workspace_sessions(activity.workspace_id, None)
                .await?;

            let workspace_sessions: Vec<_> = sessions
                .iter()
                .filter(|s| s.started_at >= start && s.started_at < end)
                .collect();

            let duration: i64 = workspace_sessions.iter().map(|s| s.duration_seconds).sum();

            Some(WorkspaceDaySummary {
                workspace_id: activity.workspace_id,
                workspace_name: workspace.name,
                duration_seconds: duration,
                session_count: workspace_sessions.len(),
                edit_count: counts.edit_count,
            })
        } else {
            None
        };

        // Get all sessions for today across all workspaces
        let all_sessions = self.get_sessions_for_range(start, end).await?;

        let total_duration: i64 = all_sessions.iter().map(|s| s.duration_seconds).sum();
        let longest_session = all_sessions.iter().map(|s| s.duration_seconds).max();
        let avg_session = if !all_sessions.is_empty() {
            Some(total_duration / all_sessions.len() as i64)
        } else {
            None
        };

        // Aggregate languages from all sessions
        let languages = self.aggregate_languages(&all_sessions);

        Ok(DailySummary {
            date,
            total_duration_seconds: total_duration,
            session_count: all_sessions.len(),
            workspace_count: workspace_count as usize,
            file_count,
            edit_count: counts.edit_count,
            commit_count: counts.commit_count,
            languages,
            most_active_workspace,
            longest_session_duration: longest_session,
            average_session_duration: avg_session,
        })
    }

    /// Gets weekly summary.
    pub async fn get_weekly_summary(
        &self,
        week_start: DateTime<Utc>,
    ) -> Result<WeeklySummary, DatabaseError> {
        let week_end = week_start + Duration::days(7);

        let counts = self
            .analytics_repository
            .count_events(week_start, week_end)
            .await?;
        let file_count = self
            .analytics_repository
            .count_distinct_files(week_start, week_end)
            .await?;
        let workspace_count = self
            .analytics_repository
            .count_distinct_workspaces(week_start, week_end)
            .await?;

        let all_sessions = self.get_sessions_for_range(week_start, week_end).await?;
        let total_duration: i64 = all_sessions.iter().map(|s| s.duration_seconds).sum();

        let languages = self.aggregate_languages(&all_sessions);

        // Find most productive day
        let mut day_durations: Vec<(DateTime<Utc>, i64)> = Vec::new();
        for day_offset in 0..7 {
            let day_start = week_start + Duration::days(day_offset);
            let day_end = day_start + Duration::days(1);

            let day_sessions: Vec<_> = all_sessions
                .iter()
                .filter(|s| s.started_at >= day_start && s.started_at < day_end)
                .collect();

            let day_duration: i64 = day_sessions.iter().map(|s| s.duration_seconds).sum();
            day_durations.push((day_start, day_duration));
        }

        let most_productive_day = day_durations
            .iter()
            .max_by_key(|(_, duration)| duration)
            .map(|(date, _)| *date);

        let average_daily = total_duration / 7;

        // Calculate focus trend (compare with previous week)
        let prev_week_start = week_start - Duration::days(7);
        let prev_sessions = self
            .get_sessions_for_range(prev_week_start, week_start)
            .await?;
        let prev_duration: i64 = prev_sessions.iter().map(|s| s.duration_seconds).sum();

        let focus_trend = if prev_duration > 0 {
            Some(TrendIndicator::new(
                total_duration as f64,
                prev_duration as f64,
                "Focus time".to_string(),
            ))
        } else {
            None
        };

        Ok(WeeklySummary {
            week_start,
            week_end,
            total_duration_seconds: total_duration,
            session_count: all_sessions.len(),
            workspace_count: workspace_count as usize,
            file_count,
            edit_count: counts.edit_count,
            commit_count: counts.commit_count,
            languages,
            most_productive_day,
            average_daily_duration: average_daily,
            focus_trend,
        })
    }

    /// Gets monthly summary.
    pub async fn get_monthly_summary(
        &self,
        month_start: DateTime<Utc>,
    ) -> Result<MonthlySummary, DatabaseError> {
        let month_end = month_start + Duration::days(30);

        let counts = self
            .analytics_repository
            .count_events(month_start, month_end)
            .await?;
        let file_count = self
            .analytics_repository
            .count_distinct_files(month_start, month_end)
            .await?;
        let workspace_count = self
            .analytics_repository
            .count_distinct_workspaces(month_start, month_end)
            .await?;

        let all_sessions = self.get_sessions_for_range(month_start, month_end).await?;
        let total_duration: i64 = all_sessions.iter().map(|s| s.duration_seconds).sum();

        let languages = self.aggregate_languages(&all_sessions);

        // Get active workspace names
        let workspace_activities = self
            .analytics_repository
            .get_workspace_activity(month_start, month_end, 10)
            .await?;

        let mut active_workspaces = Vec::new();
        for activity in workspace_activities {
            if let Ok(workspace) = self
                .workspace_repository
                .get_by_id(activity.workspace_id)
                .await
            {
                active_workspaces.push(workspace.name);
            }
        }

        let weekly_average = total_duration / 4; // Approximate 4 weeks

        Ok(MonthlySummary {
            month_start,
            month_end,
            total_duration_seconds: total_duration,
            session_count: all_sessions.len(),
            workspace_count: workspace_count as usize,
            file_count,
            edit_count: counts.edit_count,
            commit_count: counts.commit_count,
            languages,
            active_workspaces,
            weekly_average_duration: weekly_average,
        })
    }

    /// Gets comprehensive workspace insights.
    pub async fn get_workspace_insight(
        &self,
        workspace_id: Uuid,
    ) -> Result<WorkspaceInsight, DatabaseError> {
        let workspace = self.workspace_repository.get_by_id(workspace_id).await?;

        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let week_start = today_start - Duration::days(7);

        let today_counts = self
            .analytics_repository
            .count_workspace_events(workspace_id, today_start, today_start + Duration::days(1))
            .await?;

        let week_counts = self
            .analytics_repository
            .count_workspace_events(workspace_id, week_start, now)
            .await?;

        // Get sessions
        let sessions = self
            .context_service
            .get_workspace_sessions(workspace_id, None)
            .await?;

        let avg_duration = if !sessions.is_empty() {
            sessions.iter().map(|s| s.duration_seconds).sum::<i64>() / sessions.len() as i64
        } else {
            0
        };

        // Get most edited files
        let file_edits = self
            .analytics_repository
            .get_most_edited_files(workspace_id, week_start, now, 5)
            .await?;

        let mut most_edited_files = Vec::new();
        for edit in file_edits {
            if let Ok(file) = self.file_repository.get_by_id(edit.file_id).await {
                most_edited_files.push(file.path_or_url);
            }
        }

        // Primary language (most used in sessions)
        let primary_language = sessions
            .iter()
            .flat_map(|s| &s.languages)
            .fold(std::collections::HashMap::new(), |mut acc, lang| {
                *acc.entry(lang.clone()).or_insert(0) += 1;
                acc
            })
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang);

        let last_active = self
            .analytics_repository
            .get_last_activity(workspace_id)
            .await?
            .unwrap_or(workspace.created_at);

        Ok(WorkspaceInsight {
            workspace_id,
            workspace_name: workspace.name,
            today_edits: today_counts.edit_count,
            weekly_edits: week_counts.edit_count,
            total_sessions: sessions.len(),
            average_session_duration: avg_duration,
            most_edited_files,
            primary_language,
            last_active,
            activity_trend: None, // TODO: Implement trend comparison
            health_trend: None,   // TODO: Implement health trend
        })
    }

    /// Gets sessions for a time range across all workspaces.
    async fn get_sessions_for_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<crate::session::types::Session>, DatabaseError> {
        // Get all workspace activities
        let workspace_activities = self
            .analytics_repository
            .get_workspace_activity(start, end, 100)
            .await?;

        let mut all_sessions = Vec::new();

        for activity in workspace_activities {
            if let Ok(sessions) = self
                .context_service
                .get_workspace_sessions(activity.workspace_id, None)
                .await
            {
                let filtered: Vec<_> = sessions
                    .into_iter()
                    .filter(|s| s.started_at >= start && s.started_at < end)
                    .collect();
                all_sessions.extend(filtered);
            }
        }

        Ok(all_sessions)
    }

    /// Aggregates language usage from sessions.
    fn aggregate_languages(
        &self,
        sessions: &[crate::session::types::Session],
    ) -> Vec<LanguageUsage> {
        let mut lang_counts: std::collections::HashMap<String, (i64, i64)> =
            std::collections::HashMap::new();

        for session in sessions {
            for lang in &session.languages {
                let entry = lang_counts.entry(lang.clone()).or_insert((0, 0));
                entry.0 += session.file_count as i64;
                entry.1 += session.event_count as i64;
            }
        }

        let total_files: i64 = lang_counts.values().map(|(files, _)| files).sum();

        let mut languages: Vec<LanguageUsage> = lang_counts
            .into_iter()
            .map(|(language, (file_count, edit_count))| {
                let percentage = if total_files > 0 {
                    (file_count as f64 / total_files as f64) * 100.0
                } else {
                    0.0
                };

                LanguageUsage {
                    language,
                    file_count,
                    edit_count,
                    percentage,
                }
            })
            .collect();

        languages.sort_by(|a, b| b.percentage.partial_cmp(&a.percentage).unwrap());
        languages
    }
}
