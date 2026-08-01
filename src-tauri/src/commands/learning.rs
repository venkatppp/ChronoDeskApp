//! Learning IPC Commands - User feedback and adaptive learning.

use std::sync::Arc;
use tauri::State;

use crate::learning::models::*;
use crate::learning::{AdaptiveLearningEngine, LearningRepository};

/// Submits user feedback on a recommendation or prediction.
#[tauri::command]
pub async fn submit_feedback(
    engine: State<'_, Arc<AdaptiveLearningEngine>>,
    request: SubmitFeedbackRequest,
) -> Result<(), String> {
    engine
        .record_feedback(
            request.feedback_type,
            request.target_type,
            request.target_id,
            request.action,
            request.context.unwrap_or(serde_json::json!({})),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Gets learning insights for the dashboard.
#[tauri::command]
pub async fn get_learning_insights(
    engine: State<'_, Arc<AdaptiveLearningEngine>>,
) -> Result<LearningInsights, String> {
    engine
        .get_learning_insights()
        .await
        .map_err(|e| e.to_string())
}

/// Adjusts prediction confidence based on learned preferences.
#[tauri::command]
pub async fn adjust_prediction_confidence(
    engine: State<'_, Arc<AdaptiveLearningEngine>>,
    target_type: FeedbackTargetType,
    target_id: String,
    base_confidence: f64,
) -> Result<ConfidenceExplanation, String> {
    engine
        .adjust_prediction_confidence(target_type, &target_id, base_confidence)
        .await
        .map_err(|e| e.to_string())
}

/// Learns workflow patterns from user behavior.
#[tauri::command]
pub async fn learn_workflow_patterns(
    engine: State<'_, Arc<AdaptiveLearningEngine>>,
    workflow_type: String,
    duration_seconds: i64,
    files: Vec<String>,
    time_of_day: i32,
) -> Result<(), String> {
    engine
        .learn_workflow_patterns(&workflow_type, duration_seconds, files, time_of_day)
        .await
        .map_err(|e| e.to_string())
}

/// Gets all user preferences.
#[tauri::command]
pub async fn get_user_preferences(
    repository: State<'_, LearningRepository>,
) -> Result<Vec<UserPreference>, String> {
    repository
        .get_all_preferences()
        .await
        .map_err(|e| e.to_string())
}

/// Gets behavioral patterns.
#[tauri::command]
pub async fn get_behavioral_patterns(
    repository: State<'_, LearningRepository>,
) -> Result<Vec<BehavioralPattern>, String> {
    repository
        .get_all_patterns()
        .await
        .map_err(|e| e.to_string())
}

/// Gets confidence trends over time.
#[tauri::command]
pub async fn get_confidence_trends(
    repository: State<'_, LearningRepository>,
    days: i64,
) -> Result<Vec<ConfidenceTrend>, String> {
    repository
        .get_confidence_trends(days)
        .await
        .map_err(|e| e.to_string())
}

/// Gets learning statistics.
#[tauri::command]
pub async fn get_learning_stats(
    repository: State<'_, LearningRepository>,
) -> Result<LearningStats, String> {
    repository
        .get_learning_stats()
        .await
        .map_err(|e| e.to_string())
}
