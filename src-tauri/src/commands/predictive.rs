//! Predictive Intelligence IPC commands.

use tauri::State;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::predictive::models::{
    AutomationRule, CreateAutomationRuleRequest, LearningProfile, PredictionsSummary, WorkflowState,
};
use crate::predictive::{AdaptiveLearning, AutomationEngine, PredictiveEngine, WorkflowEngine};

/// Gets predictions summary for the dashboard.
#[tauri::command]
pub async fn get_predictions_summary(
    engine: State<'_, PredictiveEngine>,
) -> Result<PredictionsSummary, DatabaseError> {
    engine.get_predictions_summary().await
}

/// Gets the current workflow for a workspace.
#[tauri::command]
pub async fn get_current_workflow(
    engine: State<'_, WorkflowEngine>,
    workspace_id: Uuid,
) -> Result<Option<WorkflowState>, DatabaseError> {
    engine.detect_current_workflow(workspace_id).await
}

/// Gets the learning profile for a user.
#[tauri::command]
pub async fn get_learning_profile(
    learning: State<'_, AdaptiveLearning>,
    user_id: String,
) -> Result<Option<LearningProfile>, DatabaseError> {
    learning.get_learning_profile(&user_id).await
}

/// Updates the learning profile based on recent activity.
#[tauri::command]
pub async fn update_learning_profile(
    learning: State<'_, AdaptiveLearning>,
    user_id: String,
) -> Result<(), DatabaseError> {
    learning.update_learning_profile(&user_id).await
}

/// Creates a new automation rule.
#[tauri::command]
pub async fn create_automation_rule(
    automation: State<'_, AutomationEngine>,
    request: CreateAutomationRuleRequest,
) -> Result<AutomationRule, DatabaseError> {
    automation.create_rule(request).await
}

/// Lists all automation rules.
#[tauri::command]
pub async fn list_automation_rules(
    automation: State<'_, AutomationEngine>,
) -> Result<Vec<AutomationRule>, DatabaseError> {
    automation.list_rules().await
}

/// Updates an automation rule's enabled status.
#[tauri::command]
pub async fn update_automation_rule_enabled(
    automation: State<'_, AutomationEngine>,
    rule_id: i64,
    enabled: bool,
) -> Result<(), DatabaseError> {
    automation.update_rule_enabled(rule_id, enabled).await
}

/// Deletes an automation rule.
#[tauri::command]
pub async fn delete_automation_rule(
    automation: State<'_, AutomationEngine>,
    rule_id: i64,
) -> Result<(), DatabaseError> {
    automation.delete_rule(rule_id).await
}
