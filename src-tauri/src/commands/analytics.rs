//! Analytics IPC commands.
//!
//! Thin handlers exposing AnalyticsEngine to the frontend.

use tauri::State;

use crate::analytics::engine::AnalyticsEngine;
use crate::analytics::models::{
    DailyBriefing, DailySummary, MonthlySummary, WeeklySummary, WorkspaceInsight,
};

/// Gets daily briefing for dashboard.
#[tauri::command]
pub async fn get_daily_briefing(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<DailyBriefing, String> {
    analytics_engine
        .get_daily_briefing()
        .await
        .map_err(|e| e.to_string())
}

/// Gets today's activity summary.
#[tauri::command]
pub async fn get_today_summary(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<DailySummary, String> {
    analytics_engine
        .get_today_summary()
        .await
        .map_err(|e| e.to_string())
}

/// Gets yesterday's activity summary.
#[tauri::command]
pub async fn get_yesterday_summary(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<DailySummary, String> {
    analytics_engine
        .get_yesterday_summary()
        .await
        .map_err(|e| e.to_string())
}

/// Gets this week's activity summary.
#[tauri::command]
pub async fn get_this_week_summary(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<WeeklySummary, String> {
    analytics_engine
        .get_this_week_summary()
        .await
        .map_err(|e| e.to_string())
}

/// Gets last week's activity summary.
#[tauri::command]
pub async fn get_last_week_summary(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<WeeklySummary, String> {
    analytics_engine
        .get_last_week_summary()
        .await
        .map_err(|e| e.to_string())
}

/// Gets this month's activity summary.
#[tauri::command]
pub async fn get_this_month_summary(
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<MonthlySummary, String> {
    analytics_engine
        .get_this_month_summary()
        .await
        .map_err(|e| e.to_string())
}

/// Gets comprehensive workspace insight.
#[tauri::command]
pub async fn get_workspace_insight(
    workspace_id: String,
    analytics_engine: State<'_, AnalyticsEngine>,
) -> Result<WorkspaceInsight, String> {
    let id =
        uuid::Uuid::parse_str(&workspace_id).map_err(|e| format!("Invalid workspace_id: {}", e))?;

    analytics_engine
        .get_workspace_insight(id)
        .await
        .map_err(|e| e.to_string())
}
