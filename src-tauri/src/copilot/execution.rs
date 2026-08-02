//! Plan Execution - Manages execution of approved multi-step plans with progress tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::ExecutionPlan;

/// Plan execution record with progress tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExecution {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub conversation_id: Option<Uuid>,
    pub status: ExecutionStatus,
    pub current_step: usize,
    pub total_steps: usize,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status of plan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Paused => write!(f, "paused"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Individual execution step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub step_number: usize,
    pub description: String,
    pub tool_name: Option<String>,
    pub arguments: Option<serde_json::Value>,
    pub status: StepStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Status of an execution step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepStatus::Pending => write!(f, "pending"),
            StepStatus::Running => write!(f, "running"),
            StepStatus::Completed => write!(f, "completed"),
            StepStatus::Failed => write!(f, "failed"),
            StepStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Execution progress event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub event_type: ExecutionEventType,
    pub step_number: Option<usize>,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Type of execution event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    Started,
    StepStarted,
    StepCompleted,
    StepFailed,
    Paused,
    Resumed,
    /// A checkpoint was written (after a completed step, or on pause).
    CheckpointSaved,
    /// A checkpoint was loaded to rebuild this execution.
    CheckpointLoaded,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExecutionEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionEventType::Started => write!(f, "started"),
            ExecutionEventType::StepStarted => write!(f, "step_started"),
            ExecutionEventType::StepCompleted => write!(f, "step_completed"),
            ExecutionEventType::StepFailed => write!(f, "step_failed"),
            ExecutionEventType::Paused => write!(f, "paused"),
            ExecutionEventType::Resumed => write!(f, "resumed"),
            ExecutionEventType::CheckpointSaved => write!(f, "checkpoint_saved"),
            ExecutionEventType::CheckpointLoaded => write!(f, "checkpoint_loaded"),
            ExecutionEventType::Completed => write!(f, "completed"),
            ExecutionEventType::Failed => write!(f, "failed"),
            ExecutionEventType::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Execution audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAudit {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub action: String,
    pub actor: AuditActor,
    pub details: String,
    pub created_at: DateTime<Utc>,
}

/// Actor who performed an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditActor {
    User,
    System,
    AI,
}

impl std::fmt::Display for AuditActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditActor::User => write!(f, "user"),
            AuditActor::System => write!(f, "system"),
            AuditActor::AI => write!(f, "ai"),
        }
    }
}

/// Request to start plan execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartExecutionRequest {
    pub plan_id: Uuid,
    pub conversation_id: Option<Uuid>,
}

/// Execution progress summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProgress {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub current_step: usize,
    pub total_steps: usize,
    pub progress_percentage: f64,
    pub steps: Vec<ExecutionStep>,
    pub recent_events: Vec<ExecutionEvent>,
    /// The full plan DAG backing this execution, when available, so the
    /// frontend can render dependency/gate structure alongside step status.
    pub plan: Option<ExecutionPlan>,
    /// Planner report attached once a planner-driven run completes, so the
    /// streamed progress snapshot carries replan/retry accounting to the UI.
    pub planner_report: Option<PlannerReport>,
}
