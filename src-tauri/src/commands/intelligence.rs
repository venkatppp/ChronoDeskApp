//! Intelligence commands - recommendations and workspace health.

use tauri::State;

use crate::intelligence::health::{WorkspaceHealth, WorkspaceHealthEngine};
use crate::intelligence::recommendation::{
    Recommendation, RecommendationCategory, RecommendationEngine, RecommendationPriority,
};

/// Gets workspace health assessment.
#[tauri::command]
pub async fn get_workspace_health(
    workspace_id: i64,
    engine: State<'_, WorkspaceHealthEngine>,
) -> Result<WorkspaceHealth, String> {
    tracing::debug!(workspace_id, "Getting workspace health");
    engine
        .calculate_health(workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets the latest cached workspace health (without recalculation).
#[tauri::command]
pub async fn get_latest_workspace_health(
    workspace_id: i64,
    engine: State<'_, WorkspaceHealthEngine>,
) -> Result<Option<WorkspaceHealth>, String> {
    tracing::debug!(workspace_id, "Getting latest workspace health");
    engine
        .get_latest_health(workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets workspace health history.
#[tauri::command]
pub async fn get_workspace_health_history(
    workspace_id: i64,
    days: i64,
    engine: State<'_, WorkspaceHealthEngine>,
) -> Result<Vec<WorkspaceHealth>, String> {
    use chrono::{Duration, Utc};

    let since = Utc::now() - Duration::days(days);
    tracing::debug!(workspace_id, days, "Getting workspace health history");
    engine
        .get_health_history(workspace_id, since)
        .await
        .map_err(|e| e.to_string())
}

/// Generates recommendations for a workspace.
#[tauri::command]
pub async fn get_workspace_recommendations(
    workspace_id: i64,
    engine: State<'_, RecommendationEngine>,
) -> Result<Vec<Recommendation>, String> {
    tracing::debug!(workspace_id, "Generating workspace recommendations");
    engine
        .generate_recommendations(workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Generates recommendations for a specific category.
#[tauri::command]
pub async fn get_category_recommendations(
    workspace_id: i64,
    category: RecommendationCategory,
    engine: State<'_, RecommendationEngine>,
) -> Result<Vec<Recommendation>, String> {
    tracing::debug!(
        workspace_id,
        ?category,
        "Generating category recommendations"
    );
    engine
        .generate_category_recommendations(workspace_id, category)
        .await
        .map_err(|e| e.to_string())
}

/// Gets high-priority recommendations only.
#[tauri::command]
pub async fn get_priority_recommendations(
    workspace_id: i64,
    min_priority: RecommendationPriority,
    engine: State<'_, RecommendationEngine>,
) -> Result<Vec<Recommendation>, String> {
    tracing::debug!(
        workspace_id,
        ?min_priority,
        "Generating priority recommendations"
    );
    engine
        .generate_priority_recommendations(workspace_id, min_priority)
        .await
        .map_err(|e| e.to_string())
}
