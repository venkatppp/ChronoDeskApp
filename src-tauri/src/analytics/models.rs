//! Analytics domain models.
//!
//! Strongly typed models for analytics data including daily/weekly/monthly
//! summaries, trends, activity heatmaps, and workspace insights.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Time range for analytics queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeRange {
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    Custom {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
}

/// Trend indicator showing change compared to previous period.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendIndicator {
    /// Current value
    pub current: f64,

    /// Previous period value
    pub previous: f64,

    /// Percentage change (positive = increase, negative = decrease)
    pub change_percent: f64,

    /// Human-readable trend description
    pub description: String,
}

impl TrendIndicator {
    pub fn new(current: f64, previous: f64, description: String) -> Self {
        let change_percent = if previous > 0.0 {
            ((current - previous) / previous) * 100.0
        } else if current > 0.0 {
            100.0
        } else {
            0.0
        };

        Self {
            current,
            previous,
            change_percent,
            description,
        }
    }

    pub fn is_improving(&self) -> bool {
        self.change_percent > 0.0
    }
}

/// Language usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageUsage {
    pub language: String,
    pub file_count: i64,
    pub edit_count: i64,
    pub percentage: f64,
}

/// Daily activity summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySummary {
    pub date: DateTime<Utc>,
    pub total_duration_seconds: i64,
    pub session_count: usize,
    pub workspace_count: usize,
    pub file_count: i64,
    pub edit_count: i64,
    pub commit_count: i64,
    pub languages: Vec<LanguageUsage>,
    pub most_active_workspace: Option<WorkspaceDaySummary>,
    pub longest_session_duration: Option<i64>,
    pub average_session_duration: Option<i64>,
}

/// Weekly activity summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklySummary {
    pub week_start: DateTime<Utc>,
    pub week_end: DateTime<Utc>,
    pub total_duration_seconds: i64,
    pub session_count: usize,
    pub workspace_count: usize,
    pub file_count: i64,
    pub edit_count: i64,
    pub commit_count: i64,
    pub languages: Vec<LanguageUsage>,
    pub most_productive_day: Option<DateTime<Utc>>,
    pub average_daily_duration: i64,
    pub focus_trend: Option<TrendIndicator>,
}

/// Monthly activity summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySummary {
    pub month_start: DateTime<Utc>,
    pub month_end: DateTime<Utc>,
    pub total_duration_seconds: i64,
    pub session_count: usize,
    pub workspace_count: usize,
    pub file_count: i64,
    pub edit_count: i64,
    pub commit_count: i64,
    pub languages: Vec<LanguageUsage>,
    pub active_workspaces: Vec<String>,
    pub weekly_average_duration: i64,
}

/// Workspace-specific daily summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDaySummary {
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub duration_seconds: i64,
    pub session_count: usize,
    pub edit_count: i64,
}

/// Comprehensive workspace insights.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInsight {
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub today_edits: i64,
    pub weekly_edits: i64,
    pub total_sessions: usize,
    pub average_session_duration: i64,
    pub most_edited_files: Vec<String>,
    pub primary_language: Option<String>,
    pub last_active: DateTime<Utc>,
    pub activity_trend: Option<TrendIndicator>,
    pub health_trend: Option<TrendIndicator>,
}

/// Activity summary for specific time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub time_range: String,
    pub duration_seconds: i64,
    pub session_count: usize,
    pub workspace_count: usize,
    pub file_count: i64,
    pub edit_count: i64,
    pub commit_count: i64,
    pub primary_language: Option<String>,
}

/// Daily briefing for dashboard display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyBriefing {
    pub greeting: String,
    pub summary: ActivitySummary,
    pub most_active_workspace: Option<WorkspaceDaySummary>,
    pub longest_focus_session: Option<i64>,
    pub primary_language: Option<String>,
    pub insights: Vec<String>,
    pub suggestions: Vec<String>,
}
