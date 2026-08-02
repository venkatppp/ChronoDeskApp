//! Autonomous Agent Runtime models - sessions, reasoning events, execution
//! policies, budgets, approvals, and the progress payload streamed to the
//! frontend (`autonomous:session` / `autonomous:reasoning`).
//!
//! RC-5 M6. These are pure data types plus deterministic policy decisions
//! (no I/O), so each policy rule is unit-testable in isolation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::proactive_models::ExecutionPlan;

/// Lifecycle status of an autonomous session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousStatus {
    /// The session is actively reasoning/executing.
    Running,
    /// The session is paused between plan runs (or after a user pause).
    Paused,
    /// The session is parked, waiting for a human approval / rejection.
    WaitingApproval,
    /// The goal was reached.
    Completed,
    /// The goal could not be reached (unrecoverable failure).
    Failed,
    /// The session was cancelled (user, approval rejection, or budget).
    Cancelled,
}

impl std::fmt::Display for AutonomousStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutonomousStatus::Running => write!(f, "running"),
            AutonomousStatus::Paused => write!(f, "paused"),
            AutonomousStatus::WaitingApproval => write!(f, "waiting_approval"),
            AutonomousStatus::Completed => write!(f, "completed"),
            AutonomousStatus::Failed => write!(f, "failed"),
            AutonomousStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl AutonomousStatus {
    /// Whether the session can no longer be acted upon by the runtime.
    pub fn terminal(self) -> bool {
        matches!(
            self,
            AutonomousStatus::Completed | AutonomousStatus::Failed | AutonomousStatus::Cancelled
        )
    }
}

/// Which stage of the reason–act–observe loop a reasoning event belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningPhase {
    /// A plan was generated or is being assembled.
    Planning,
    /// A plan handoff to the execution engine started.
    Executing,
    /// A plan run finished and produced results.
    Observed,
    /// A failed plan is being rewritten (feedback-aware).
    Replanning,
    /// The run suspended for an approval checkpoint.
    AwaitingApproval,
    /// An approval decision was received.
    ApprovalResolved,
    /// Budget counters were updated or a warning fired.
    BudgetUpdate,
    /// The session was paused / resumed.
    Pause,
    /// The session reached a terminal state.
    Terminal,
}

impl std::fmt::Display for ReasoningPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReasoningPhase::Planning => write!(f, "planning"),
            ReasoningPhase::Executing => write!(f, "executing"),
            ReasoningPhase::Observed => write!(f, "observed"),
            ReasoningPhase::Replanning => write!(f, "replanning"),
            ReasoningPhase::AwaitingApproval => write!(f, "awaiting_approval"),
            ReasoningPhase::ApprovalResolved => write!(f, "approval_resolved"),
            ReasoningPhase::BudgetUpdate => write!(f, "budget_update"),
            ReasoningPhase::Pause => write!(f, "pause"),
            ReasoningPhase::Terminal => write!(f, "terminal"),
        }
    }
}

/// One step in the session's reason–act–observe timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningEvent {
    pub session_id: Uuid,
    pub phase: ReasoningPhase,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl ReasoningEvent {
    pub fn new(
        session_id: Uuid,
        phase: ReasoningPhase,
        message: impl Into<String>,
        detail: Option<serde_json::Value>,
    ) -> Self {
        Self {
            session_id,
            phase,
            message: message.into(),
            detail,
            created_at: Utc::now(),
        }
    }
}

/// When an autonomous session pauses for a human-in-the-loop decision.
///
/// The session's `AutonomousStatus` flips to `WaitingApproval` while the
/// operator decides; `decided_at`/`approved` fill in once `approve` or
/// `reject` is called.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub goal: String,
    /// The plan about to be executed, so the UI can show its DAG.
    pub plan: ExecutionPlan,
    /// Human-readable reason the session needs a checkpoint (risk level,
    /// policy mode, ...).
    pub reason: String,
    pub requested_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// How strictly an autonomous session pauses for operator confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Never pause for confirmation. The autonomous tool still obeys the
    /// persistent `ToolPermissionService` (e.g. a per-workspace `Deny`).
    #[default]
    Automatic,
    /// Pause only when the upcoming plan would run a tool marked
    /// `requires_confirmation`, or whose static risk is `High`.
    OnRisk,
    /// Pause before every plan run.
    Manual,
}

impl ApprovalMode {
    /// Whether the runtime may pass a plan to the engine without holding it.
    pub fn requires_pause(self) -> bool {
        matches!(self, ApprovalMode::OnRisk | ApprovalMode::Manual)
    }
}

/// Wall-clock and step-count spending limits for one session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBudget {
    /// Max steps the session may consume across all plan runs (0 = unlimited).
    pub max_steps: usize,
    /// Max distinct engine executions started (0 = unlimited).
    pub max_plans: u64,
    /// Max feedback-driven replans (adapts `Planner::MAX_REPLAN_ATTEMPTS`).
    pub max_replans: u64,
    /// Max wall-clock seconds for the whole session (0 = unlimited).
    pub max_duration_seconds: u64,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_steps: 50,
            max_plans: 8,
            max_replans: 3,
            max_duration_seconds: 3600,
        }
    }
}

/// Retry behaviour for a failed *plan run* (per-session, not per-tool).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Extra re-attempts of a failing plan before replanning (0 = none).
    pub max_attempts: u64,
    /// Back-off, in ms, before starting a retry execution.
    pub backoff_ms: u64,
    /// Whether timeouts count toward the retry budget.
    pub retry_on_timeout: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff_ms: 250,
            retry_on_timeout: true,
        }
    }
}

/// Timeout guards for one session's executions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// Per-step tool timeout in ms (passed through to `ToolExecutor`).
    pub step_timeout_ms: u64,
    /// Whole-plan timeout in seconds (cancelled if exceeded). 0 = none.
    pub plan_timeout_seconds: u64,
    /// How long to wait for an operator decision (0 = wait forever).
    pub approval_timeout_seconds: u64,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            step_timeout_ms: 10_000,
            plan_timeout_seconds: 0,
            approval_timeout_seconds: 0,
        }
    }
}

impl TimeoutPolicy {
    pub fn step_timeout_ms(&self) -> u64 {
        if self.step_timeout_ms == 0 {
            10_000
        } else {
            self.step_timeout_ms
        }
    }

    pub fn plan_timeout_s(&self) -> u64 {
        self.plan_timeout_seconds
    }

    pub fn approval_timeout_s(&self) -> u64 {
        self.approval_timeout_seconds
    }
}

/// Approval gating configuration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub mode: ApprovalMode,
    /// Replanning that introduces a tool not previously run also pauses
    /// (`OnRisk`/`Manual`).
    pub gate_replans: bool,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self {
            mode: ApprovalMode::Automatic,
            gate_replans: false,
        }
    }
}

/// Execution policies for one autonomous session — budget, retries,
/// timeouts, and approvals, with sane defaults that match the existing
/// planner/engine behaviour.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionPolicy {
    pub budget: ExecutionBudget,
    pub retry: RetryPolicy,
    pub timeout: TimeoutPolicy,
    pub approval: ApprovalPolicy,
}

/// Live snapshot of a session, streamed over `autonomous:session` and
/// returned by every autonomous IPC command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSessionProgress {
    pub session_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub goal: String,
    pub status: AutonomousStatus,
    pub policy: ExecutionPolicy,
    /// Bounded reason log (newest last).
    pub reasoning: Vec<ReasoningEvent>,
    /// The plan currently executing or awaiting approval.
    pub current_plan: Option<ExecutionPlan>,
    /// Execution engine run id, when one is active.
    pub execution_id: Option<Uuid>,
    /// Last engine execution id, for reconnect/recovery.
    pub last_execution_id: Option<Uuid>,
    /// Plans handed to the engine (attempts), including retries of the same.
    pub plans_attempted: u64,
    /// Plans that completed successfully.
    pub plans_completed: u64,
    pub steps_completed: u64,
    pub retries_used: u64,
    pub replans_used: u64,
    pub steps_left: u64,
    pub error: Option<String>,
    /// Tied to the *step* timeout override (see `TimeoutPolicy`).
    pub pending_approval: Option<ApprovalRequest>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AutonomousSessionProgress {
    pub fn terminal(&self) -> bool {
        matches!(
            self.status,
            AutonomousStatus::Completed | AutonomousStatus::Failed | AutonomousStatus::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_matches_backward_compatible_limits() {
        let policy = ExecutionPolicy::default();
        assert_eq!(policy.budget.max_replans, 3);
        assert_eq!(policy.retry.max_attempts, 1);
        assert_eq!(policy.timeout.step_timeout_ms(), 10_000);
        assert!(matches!(policy.approval.mode, ApprovalMode::Automatic));
    }

    #[test]
    fn approval_modes_require_pause_consistently() {
        assert!(!ApprovalMode::Automatic.requires_pause());
        assert!(ApprovalMode::OnRisk.requires_pause());
        assert!(ApprovalMode::Manual.requires_pause());
    }

    #[test]
    fn terminal_sessions_are_detected() {
        assert!(!base_progress_with_status(AutonomousStatus::Running).terminal());
        assert!(base_progress_with_status(AutonomousStatus::Completed).terminal());
        assert!(base_progress_with_status(AutonomousStatus::Failed).terminal());
        assert!(base_progress_with_status(AutonomousStatus::Cancelled).terminal());
        assert!(!base_progress_with_status(AutonomousStatus::WaitingApproval).terminal());
        assert!(!base_progress_with_status(AutonomousStatus::Paused).terminal());
    }

    fn base_progress_with_status(status: AutonomousStatus) -> AutonomousSessionProgress {
        AutonomousSessionProgress {
            session_id: Uuid::new_v4(),
            workspace_id: None,
            goal: "g".into(),
            status,
            policy: ExecutionPolicy::default(),
            reasoning: vec![],
            current_plan: None,
            execution_id: None,
            last_execution_id: None,
            plans_attempted: 0,
            plans_completed: 0,
            steps_completed: 0,
            retries_used: 0,
            replans_used: 0,
            steps_left: 0,
            error: None,
            pending_approval: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn reasoning_event_round_trips() {
        let event = ReasoningEvent::new(
            Uuid::new_v4(),
            ReasoningPhase::Planning,
            "planning ready",
            Some(serde_json::json!({ "steps": 3 })),
        );
        let json = serde_json::to_value(&event).unwrap();
        let back: ReasoningEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back.message, "planning ready");
        assert!(matches!(back.phase, ReasoningPhase::Planning));
    }
}
