//! Predictive Intelligence models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Prediction for the next workspace the user will switch to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePrediction {
    pub workspace_id: String,
    pub workspace_name: String,
    pub confidence: f64,
    pub reason: String,
    pub predicted_at: DateTime<Utc>,
}

/// Prediction for the next files the user will open.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePrediction {
    pub file_path: String,
    pub workspace_id: String,
    pub confidence: f64,
    pub reason: String,
}

/// Prediction for the next action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPrediction {
    pub action_type: String,
    pub description: String,
    pub confidence: f64,
    pub reason: String,
}

/// Prediction for session continuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContinuationPrediction {
    pub will_continue: bool,
    pub confidence: f64,
    pub estimated_duration_seconds: i64,
    pub reason: String,
}

/// Workflow type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowType {
    Coding,
    Debugging,
    Documentation,
    Research,
    Meeting,
    Custom,
}

impl WorkflowType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowType::Coding => "coding",
            WorkflowType::Debugging => "debugging",
            WorkflowType::Documentation => "documentation",
            WorkflowType::Research => "research",
            WorkflowType::Meeting => "meeting",
            WorkflowType::Custom => "custom",
        }
    }
}

/// Current workflow state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowState {
    pub workflow_type: WorkflowType,
    pub started_at: DateTime<Utc>,
    pub workspace_id: String,
    pub confidence: f64,
    pub active_files: Vec<String>,
}

/// Workflow transition detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTransition {
    pub from_workflow: WorkflowType,
    pub to_workflow: WorkflowType,
    pub confidence: f64,
    pub detected_at: DateTime<Utc>,
}

/// Adaptive learning profile (aggregated only, no personal content).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningProfile {
    pub user_id: String,
    pub preferred_work_hours: Vec<i32>, // Hours 0-23
    pub avg_session_duration_seconds: i64,
    pub workspace_switch_frequency: f64, // switches per hour
    pub technology_preferences: Vec<TechPreference>,
    pub focus_patterns: FocusPattern,
    pub last_updated: DateTime<Utc>,
}

/// Technology preference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TechPreference {
    pub technology: String,
    pub usage_percentage: f64,
}

/// Focus pattern analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusPattern {
    pub peak_focus_hours: Vec<i32>,
    pub avg_focus_duration_minutes: i32,
    pub distraction_frequency: f64,
}

/// Automation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationRule {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub trigger_type: TriggerType,
    pub trigger_config: serde_json::Value,
    pub action_type: ActionType,
    pub action_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Trigger type for automation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    WorkspaceActivated,
    LongInactive,
    DuplicatesExceedThreshold,
    ProductivityDrop,
    WorkflowTransition,
    TimeOfDay,
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerType::WorkspaceActivated => "workspace_activated",
            TriggerType::LongInactive => "long_inactive",
            TriggerType::DuplicatesExceedThreshold => "duplicates_exceed_threshold",
            TriggerType::ProductivityDrop => "productivity_drop",
            TriggerType::WorkflowTransition => "workflow_transition",
            TriggerType::TimeOfDay => "time_of_day",
        }
    }
}

/// Action type for automation rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    RestoreContext,
    CreateSnapshot,
    RecommendCleanup,
    RecommendBreak,
    NotifyUser,
    SwitchWorkspace,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::RestoreContext => "restore_context",
            ActionType::CreateSnapshot => "create_snapshot",
            ActionType::RecommendCleanup => "recommend_cleanup",
            ActionType::RecommendBreak => "recommend_break",
            ActionType::NotifyUser => "notify_user",
            ActionType::SwitchWorkspace => "switch_workspace",
        }
    }
}

/// Automation execution log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationExecution {
    pub id: i64,
    pub rule_id: i64,
    pub executed_at: DateTime<Utc>,
    pub success: bool,
    pub result: serde_json::Value,
}

/// Request to create an automation rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAutomationRuleRequest {
    pub name: String,
    pub enabled: bool,
    pub trigger_type: TriggerType,
    pub trigger_config: serde_json::Value,
    pub action_type: ActionType,
    pub action_config: serde_json::Value,
}

/// Predictions summary for dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionsSummary {
    pub next_workspace: Option<WorkspacePrediction>,
    pub next_files: Vec<FilePrediction>,
    pub next_actions: Vec<ActionPrediction>,
    pub session_continuation: Option<SessionContinuationPrediction>,
    pub current_workflow: Option<WorkflowState>,
}

#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn predictions_summary_serializes_camel_case() {
        let summary = PredictionsSummary {
            next_workspace: Some(WorkspacePrediction {
                workspace_id: "ws-1".into(),
                workspace_name: "Main".into(),
                confidence: 0.9,
                reason: "frequent use".into(),
                predicted_at: chrono::Utc::now(),
            }),
            next_files: vec![FilePrediction {
                file_path: "/src/main.rs".into(),
                workspace_id: "ws-1".into(),
                confidence: 0.8,
                reason: "open".into(),
            }],
            next_actions: vec![ActionPrediction {
                action_type: "restore_context".into(),
                description: "Restore".into(),
                confidence: 0.7,
                reason: "habit".into(),
            }],
            session_continuation: Some(SessionContinuationPrediction {
                will_continue: true,
                confidence: 0.6,
                estimated_duration_seconds: 1200,
                reason: "active".into(),
            }),
            current_workflow: Some(WorkflowState {
                workflow_type: WorkflowType::Coding,
                started_at: chrono::Utc::now(),
                workspace_id: "ws-1".into(),
                confidence: 0.5,
                active_files: vec!["/src/main.rs".into()],
            }),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("nextWorkspace").is_some());
        assert!(json.get("nextFiles").is_some());
        assert!(json.get("nextActions").is_some());
        assert!(json.get("sessionContinuation").is_some());
        assert!(json.get("currentWorkflow").is_some());
        assert!(json.get("next_files").is_none());
    }
}
