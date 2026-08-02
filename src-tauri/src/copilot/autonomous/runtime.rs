//! Autonomous Agent Runtime - the reason–act–observe loop that drives
//! [`Planner`] and [`ExecutionEngine`] through an autonomous *session*,
//! enforcing budgets, retries, timeouts, approval checkpoints, and
//! cancellation, and streaming reasoning events to the frontend.
//!
//! This runtime owns the *session*. It never schedules an execution step
//! (that is the engine's DAG-walker) and never invokes a tool (that is the
//! shared `ToolExecutor` pipeline). Everything below is reused unchanged.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{Notify, RwLock};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::app_events::{emit, AppEventEmitter};
use crate::copilot::autonomous::models::*;
use crate::copilot::execution::{ExecutionStatus, StepStatus};
use crate::copilot::execution_engine::ExecutionEngine;
use crate::copilot::memory::MemoryEngine;
use crate::copilot::planner::{Planner, PlannerReport, ReplanFeedback};
use crate::copilot::proactive_models::ExecutionPlan;
use crate::copilot::tools::{ToolExecutor, ToolRiskLevel};
use crate::copilot::PlannerError;
use crate::errors::DatabaseError;

/// Named frontend events emitted by the autonomous runtime.
pub const EVENT_AUTONOMOUS_SESSION: &str = "autonomous:session";
pub const EVENT_AUTONOMOUS_REASONING: &str = "autonomous:reasoning";

/// Maximum reasoning events retained per session (bounded history; the
/// stream itself is one event per message, so only the reconnect snapshot
/// is capped).
const REASONING_EVENT_CAP: usize = 200;

/// Errors surfaced by the autonomous runtime.
#[derive(Debug, thiserror::Error)]
pub enum AutonomousRuntimeError {
    #[error("session not found: {0}")]
    NotFound(Uuid),
    #[error("session is not active: {0}")]
    NotActive(Uuid),
    #[error("session has no pending approval checkpoint")]
    NoPendingApproval,
    #[error("planning failed: {0}")]
    Planning(#[from] PlannerError),
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
    #[error("session {0} is already in a terminal state")]
    Terminal(Uuid),
}

/// Per-session live state. Owned paths: the loop holds an `Arc<SessionState>`
/// and mutates `inner` through short, non-holding locks, so the global
/// registry lock is never held across `.await` points.
struct SessionState {
    session_id: Uuid,
    inner: RwLock<SessionInner>,
    token: CancellationToken,
    /// Fired whenever an approval checkpoint is resolved.
    approval_notify: Arc<Notify>,
    started_at: Instant,
}

struct SessionInner {
    workspace_id: Option<Uuid>,
    goal: String,
    status: AutonomousStatus,
    policy: ExecutionPolicy,
    reasoning: VecDeque<ReasoningEvent>,
    current_plan: Option<ExecutionPlan>,
    execution_id: Option<Uuid>,
    last_execution_id: Option<Uuid>,
    plans_attempted: u64,
    plans_completed: u64,
    steps_completed: u64,
    retries_used: u64,
    replans_used: u64,
    error: Option<String>,
    pending_approval: Option<ApprovalRequest>,
    attempted_tools: std::collections::HashSet<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

/// Outcome of one plan run through the engine.
enum RunOutcome {
    Completed {
        steps_done: usize,
    },
    Cancelled,
    Paused,
    Timeout(String),
    Failed {
        error: String,
        tool_name: Option<String>,
        failed_step: Option<Uuid>,
        steps_done: usize,
    },
}

/// Approval gate verdict.
enum GateDecision {
    Proceed,
    Rejected,
}

/// The autonomous agent runtime.
#[derive(Clone)]
pub struct AutonomousRuntime {
    planner: Arc<Planner>,
    engine: Arc<ExecutionEngine>,
    sessions: Arc<RwLock<HashMap<Uuid, Arc<SessionState>>>>,
    event_emitter: Option<Arc<dyn AppEventEmitter>>,
    tool_meta: ToolRiskLookup,
    /// Execution memory consulted during reasoning and fed with terminal
    /// sessions (RC-6 M1). Optional for backward compatibility; all
    /// memory interactions are advisory and best-effort.
    memory: Option<Arc<MemoryEngine>>,
}

/// Statically inspected risk metadata over the tool registry, used for
/// approval checkpoint decisions.
#[derive(Clone)]
pub struct ToolRiskLookup {
    confirmation: Vec<String>,
    high_risk: Vec<String>,
}

impl ToolRiskLookup {
    fn build(executor: &ToolExecutor) -> Self {
        let mut confirmation = Vec::new();
        let mut high_risk = Vec::new();
        for def in executor.available_tools() {
            if def.requires_confirmation {
                confirmation.push(def.name.clone());
            }
            if def.permission.risk_level == ToolRiskLevel::High {
                high_risk.push(def.name.clone());
            }
        }
        Self {
            confirmation,
            high_risk,
        }
    }
}

/// Pure approval decision: given the policy, the tools in an upcoming plan,
/// and the tools already seen, decide whether an approval checkpoint is
/// required and why. Kept deterministic and side-effect free for tests.
pub fn approval_required(
    policy: &ApprovalPolicy,
    plan: &ExecutionPlan,
    meta: &ToolRiskLookup,
) -> (bool, String) {
    if plan.tasks.is_empty() {
        return (false, String::new());
    }
    match policy.mode {
        ApprovalMode::Automatic => (false, String::new()),
        ApprovalMode::Manual => (true, "manual approval mode".to_string()),
        ApprovalMode::OnRisk => {
            let mut reason = String::new();
            for task in &plan.tasks {
                let Some(tool_name) = task.tool_name.as_deref() else {
                    continue;
                };
                let confirmation = meta.confirmation.iter().any(|t| t == tool_name);
                let high = meta.high_risk.iter().any(|t| t == tool_name);
                if confirmation || high {
                    reason = if confirmation {
                        format!("step '{}' requires operator confirmation", task.description)
                    } else {
                        format!("step '{}' is high risk", task.description)
                    };
                    return (true, reason);
                }
            }
            (false, reason)
        }
    }
}

impl AutonomousRuntime {
    /// Creates a runtime over the shared planner + engine + tool executor.
    pub fn new(
        planner: Arc<Planner>,
        engine: Arc<ExecutionEngine>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        let tool_meta = ToolRiskLookup::build(&tool_executor);
        Self {
            planner,
            engine,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_emitter: None,
            tool_meta,
            memory: None,
        }
    }

    /// Attaches the frontend event emitter forwarding `autonomous:*` events.
    pub fn with_event_emitter(mut self, emitter: Arc<dyn AppEventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Attaches the execution memory store the runtime consults during
    /// reasoning and feeds with terminal sessions (RC-6 M1).
    pub fn with_memory(mut self, memory: Arc<MemoryEngine>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Starts an autonomous session for a goal. Returns immediately with the
    /// initial progress snapshot; the reason–act–observe loop runs detached.
    pub async fn start_session(
        &self,
        workspace_id: Option<Uuid>,
        goal: &str,
        policy: Option<ExecutionPolicy>,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        if goal.trim().is_empty() {
            return Err(AutonomousRuntimeError::Planning(PlannerError::EmptyGoal));
        }
        let policy = policy.unwrap_or_default();
        let session_id = Uuid::new_v4();
        let state = Arc::new(SessionState {
            session_id,
            inner: RwLock::new(SessionInner {
                workspace_id,
                goal: goal.to_string(),
                status: AutonomousStatus::Running,
                policy,
                reasoning: VecDeque::new(),
                current_plan: None,
                execution_id: None,
                last_execution_id: None,
                plans_attempted: 0,
                plans_completed: 0,
                steps_completed: 0,
                retries_used: 0,
                replans_used: 0,
                error: None,
                pending_approval: None,
                attempted_tools: std::collections::HashSet::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }),
            token: CancellationToken::new(),
            approval_notify: Arc::new(Notify::new()),
            started_at: Instant::now(),
        });

        self.sessions
            .write()
            .await
            .insert(session_id, state.clone());
        self.record_reasoning(
            &state,
            ReasoningPhase::Planning,
            format!("Starting autonomous session for goal: {goal}"),
            workspace_id.map(|id| serde_json::json!({ "workspace_id": id })),
        )
        .await;

        let runtime = Arc::new(self.clone());
        let loop_state = state.clone();
        tokio::spawn(async move {
            runtime.run_session(loop_state).await;
        });

        self.progress_for(&state).await
    }

    /// Current progress snapshot for a session.
    pub async fn get_progress(
        &self,
        session_id: Uuid,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let state = self.session(session_id).await?;
        self.progress_for(&state).await
    }

    /// Recent sessions (newest first) as live progress snapshots.
    pub async fn list_recent(&self, limit: usize) -> Vec<AutonomousSessionProgress> {
        let states: Vec<Arc<SessionState>> = self.sessions.read().await.values().cloned().collect();
        let mut progress = Vec::with_capacity(states.len());
        for state in states {
            if let Ok(progress_snapshot) = self.progress_for(&state).await {
                progress.push(progress_snapshot);
            }
        }
        progress.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        progress.truncate(limit);
        progress
    }

    /// Pauses the session: pauses the active engine run and suspends the
    /// reason loop between plan runs.
    pub async fn pause_session(
        &self,
        session_id: Uuid,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let state = self.session(session_id).await?;
        let (status, execution_id) = {
            let inner = state.inner.read().await;
            (inner.status, inner.execution_id)
        };
        match status {
            AutonomousStatus::Running => {}
            AutonomousStatus::Paused => return self.progress_for(&state).await,
            _ => return Err(AutonomousRuntimeError::NotActive(session_id)),
        }
        if let Some(execution_id) = execution_id {
            self.engine.pause_execution(execution_id).await?;
        }
        self.set_status(&state, AutonomousStatus::Paused).await;
        self.record_reasoning(
            &state,
            ReasoningPhase::Pause,
            "Session paused by operator",
            None,
        )
        .await;
        self.progress_for(&state).await
    }

    /// Resumes a paused session.
    pub async fn resume_session(
        &self,
        session_id: Uuid,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let state = self.session(session_id).await?;
        let (status, execution_id) = {
            let inner = state.inner.read().await;
            (inner.status, inner.execution_id)
        };
        if status != AutonomousStatus::Paused {
            return Err(AutonomousRuntimeError::NotActive(session_id));
        }
        if let Some(execution_id) = execution_id {
            self.engine.resume_execution(execution_id).await?;
        }
        self.set_status(&state, AutonomousStatus::Running).await;
        self.record_reasoning(
            &state,
            ReasoningPhase::Pause,
            "Session resumed by operator",
            None,
        )
        .await;
        self.progress_for(&state).await
    }

    /// Cancels a session: propagates to the active engine run and to the
    /// session's cancellation token so the loop exits cooperatively.
    pub async fn cancel_session(
        &self,
        session_id: Uuid,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let state = self.session(session_id).await?;
        let status = {
            let inner = state.inner.read().await;
            inner.status
        };
        if status.terminal() {
            return Err(AutonomousRuntimeError::Terminal(session_id));
        }
        state.token.cancel();
        let execution_id = {
            let inner = state.inner.read().await;
            inner.execution_id
        };
        if let Some(execution_id) = execution_id {
            let _ = self.engine.cancel_execution(execution_id).await;
        }
        self.set_status(&state, AutonomousStatus::Cancelled).await;
        self.record_reasoning(
            &state,
            ReasoningPhase::Terminal,
            "Session cancelled by operator",
            None,
        )
        .await;
        self.capture_session(&state).await;
        self.progress_for(&state).await
    }

    /// Approves a pending approval checkpoint.
    pub async fn approve_session(
        &self,
        session_id: Uuid,
        note: Option<String>,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        self.resolve_approval(session_id, true, note).await
    }

    /// Rejects a pending approval checkpoint (terminates the session).
    pub async fn reject_session(
        &self,
        session_id: Uuid,
        note: Option<String>,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        self.resolve_approval(session_id, false, note).await
    }

    async fn resolve_approval(
        &self,
        session_id: Uuid,
        approved: bool,
        note: Option<String>,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let state = self.session(session_id).await?;
        {
            let mut inner = state.inner.write().await;
            let Some(pending) = inner.pending_approval.as_mut() else {
                return Err(AutonomousRuntimeError::NoPendingApproval);
            };
            if pending.approved.is_some() {
                return Err(AutonomousRuntimeError::NoPendingApproval);
            }
            pending.approved = Some(approved);
            pending.note = note.clone();
            pending.decided_at = Some(Utc::now());
        }
        if approved {
            self.set_status(&state, AutonomousStatus::Running).await;
            self.record_reasoning(
                &state,
                ReasoningPhase::ApprovalResolved,
                "Approval granted by operator",
                note.map(|n| serde_json::json!({ "note": n })),
            )
            .await;
        } else {
            self.set_status(&state, AutonomousStatus::Cancelled).await;
            self.record_reasoning(
                &state,
                ReasoningPhase::Terminal,
                "Plan rejected by operator",
                note.map(|n| serde_json::json!({ "note": n })),
            )
            .await;
            self.capture_session(&state).await;
        }
        state.approval_notify.notify_waiters();
        self.progress_for(&state).await
    }

    // ------------------------------------------------------------------
    // Session loop internals
    // ------------------------------------------------------------------

    async fn run_session(self: Arc<Self>, state: Arc<SessionState>) {
        let (workspace_id, goal, token) = {
            let inner = state.inner.read().await;
            (inner.workspace_id, inner.goal.clone(), state.token.clone())
        };

        // Consult execution memory (RC-6 M1): surface learned workflows
        // and previously failed strategies in the reasoning stream before
        // the planner builds the first plan. The planner itself reuses a
        // matching successful workflow when one exists.
        if let Some(memory) = &self.memory {
            match memory.recommend(&goal, workspace_id, 3).await {
                Ok(recommendations) if !recommendations.is_empty() => {
                    let top = &recommendations[0];
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Planning,
                        format!(
                            "Execution memory holds {} similar run(s); top workflow scored {:.2}",
                            recommendations.len(),
                            top.score
                        ),
                        Some(serde_json::json!({
                            "memory_id": top.record.id,
                            "replays": top.replay_count,
                            "similar_goal": top.record.goal,
                        })),
                    )
                    .await;
                }
                Ok(_) => {
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Planning,
                        "Execution memory has no similar workflows for this goal",
                        None,
                    )
                    .await;
                }
                Err(error) => {
                    tracing::warn!(error = %error, "memory consultation failed for session");
                }
            }
        }

        let mut plan = match self.planner.plan(workspace_id, Some(&token), &goal).await {
            Ok(plan) => plan,
            Err(error) => {
                if token.is_cancelled() {
                    self.set_status(&state, AutonomousStatus::Cancelled).await;
                } else {
                    self.set_status(&state, AutonomousStatus::Failed).await;
                }
                self.set_error(&state, error.to_string()).await;
                self.record_reasoning(
                    &state,
                    ReasoningPhase::Terminal,
                    format!("Planning failed: {error}"),
                    None,
                )
                .await;
                self.capture_session(&state).await;
                return;
            }
        };

        let mut completed: Vec<Uuid> = Vec::new();

        loop {
            if token.is_cancelled() {
                self.set_status(&state, AutonomousStatus::Cancelled).await;
                self.record_reasoning(&state, ReasoningPhase::Terminal, "Session cancelled", None)
                    .await;
                self.capture_session(&state).await;
                return;
            }

            let runnable = {
                let inner = state.inner.read().await;
                inner.status == AutonomousStatus::Running
            };
            if !runnable {
                self.wait_for_resume(&state).await;
                continue;
            }

            // 1. Execution budget gate.
            if let Some(reason) = self.budget_breach(&state).await {
                self.set_status(&state, AutonomousStatus::Failed).await;
                let error = format!("Execution budget exceeded: {reason}");
                self.set_error(&state, error.clone()).await;
                self.record_reasoning(&state, ReasoningPhase::Terminal, error, None)
                    .await;
                self.capture_session(&state).await;
                return;
            }

            // 2. Approval checkpoint (human-in-the-loop).
            match self.approval_gate(&state, &plan).await {
                Err(_) => {
                    if token.is_cancelled() {
                        self.set_status(&state, AutonomousStatus::Cancelled).await;
                        self.record_reasoning(
                            &state,
                            ReasoningPhase::Terminal,
                            "Session cancelled",
                            None,
                        )
                        .await;
                        self.capture_session(&state).await;
                    }
                    return;
                }
                Ok(GateDecision::Rejected) => return,
                Ok(GateDecision::Proceed) => {}
            }

            if token.is_cancelled() {
                self.set_status(&state, AutonomousStatus::Cancelled).await;
                self.record_reasoning(&state, ReasoningPhase::Terminal, "Session cancelled", None)
                    .await;
                self.capture_session(&state).await;
                return;
            }

            // 3. Run the plan through the execution engine.
            self.record_reasoning(
                &state,
                ReasoningPhase::Executing,
                format!(
                    "Driving the {}-step plan through the execution engine",
                    plan.tasks.len()
                ),
                Some(serde_json::json!({ "plan_id": plan.id })),
            )
            .await;

            let outcome = self.run_plan(&state, &plan).await;

            match outcome {
                RunOutcome::Completed { steps_done } => {
                    self.finish_completed(&state, plan, steps_done, completed)
                        .await;
                    return;
                }
                RunOutcome::Cancelled => {
                    self.set_status(&state, AutonomousStatus::Cancelled).await;
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Terminal,
                        "Execution cancelled",
                        None,
                    )
                    .await;
                    self.capture_session(&state).await;
                    return;
                }
                RunOutcome::Paused => {
                    self.set_status(&state, AutonomousStatus::Paused).await;
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Pause,
                        "Plan run paused; awaiting resume",
                        None,
                    )
                    .await;
                    self.wait_for_resume(&state).await;
                    if token.is_cancelled() {
                        self.set_status(&state, AutonomousStatus::Cancelled).await;
                    }
                }
                RunOutcome::Timeout(message) => {
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Replanning,
                        format!("Plan run timed out: {message}"),
                        None,
                    )
                    .await;
                    match self
                        .recover_failure(&state, &plan, &mut completed, None, message, true)
                        .await
                    {
                        Some(replanned) => plan = replanned,
                        None => return,
                    }
                }
                RunOutcome::Failed {
                    error,
                    tool_name,
                    failed_step,
                    steps_done,
                } => {
                    self.note_steps_done(&state, steps_done).await;
                    self.note_attempted_tool(&state, tool_name.as_deref()).await;
                    self.record_reasoning(
                        &state,
                        ReasoningPhase::Observed,
                        format!("Plan run failed: {error}"),
                        tool_name.map(|t| serde_json::json!({ "tool": t })),
                    )
                    .await;
                    match self
                        .recover_failure(&state, &plan, &mut completed, failed_step, error, false)
                        .await
                    {
                        Some(replanned) => plan = replanned,
                        None => return,
                    }
                }
            }
        }
    }

    /// Drives one engine execution (applying the per-plan timeout policy if
    /// configured), then returns the outcome.
    async fn run_plan(&self, state: &Arc<SessionState>, plan: &ExecutionPlan) -> RunOutcome {
        let execution_id = match self.engine.start_execution(plan, None).await {
            Ok(id) => id,
            Err(error) => {
                return RunOutcome::Failed {
                    error: error.to_string(),
                    tool_name: None,
                    failed_step: None,
                    steps_done: 0,
                };
            }
        };
        {
            let mut inner = state.inner.write().await;
            inner.execution_id = Some(execution_id);
            inner.last_execution_id = Some(execution_id);
            inner.plans_attempted += 1;
            inner.updated_at = Utc::now();
        }

        let plan_timeout_s = {
            let inner = state.inner.read().await;
            inner.policy.timeout.plan_timeout_seconds
        };

        let drive = self.engine.execute_until_complete(execution_id);
        let result = if plan_timeout_s > 0 {
            match tokio::time::timeout(Duration::from_secs(plan_timeout_s), drive).await {
                Ok(result) => result,
                Err(_) => {
                    let _ = self.engine.cancel_execution(execution_id).await;
                    return RunOutcome::Timeout(format!(
                        "plan ran past the {}s deadline",
                        plan_timeout_s
                    ));
                }
            }
        } else {
            drive.await
        };

        if let Err(error) = result {
            return RunOutcome::Failed {
                error: error.to_string(),
                tool_name: None,
                failed_step: None,
                steps_done: 0,
            };
        }

        let progress = match self.engine.get_progress(execution_id).await {
            Ok(progress) => progress,
            Err(error) => {
                return RunOutcome::Failed {
                    error: error.to_string(),
                    tool_name: None,
                    failed_step: None,
                    steps_done: 0,
                };
            }
        };

        let steps_done = progress
            .steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed | StepStatus::Skipped))
            .count();

        match progress.status {
            ExecutionStatus::Completed => RunOutcome::Completed { steps_done },
            ExecutionStatus::Cancelled => RunOutcome::Cancelled,
            ExecutionStatus::Paused => RunOutcome::Paused,
            ExecutionStatus::Pending | ExecutionStatus::Running => RunOutcome::Failed {
                error: "execution did not reach a terminal state".into(),
                tool_name: None,
                failed_step: None,
                steps_done,
            },
            ExecutionStatus::Failed => {
                let failed = progress
                    .steps
                    .iter()
                    .find(|s| s.status == StepStatus::Failed)
                    .map(|step| {
                        let task_id = plan.tasks.get(step.step_number).map(|t| t.id);
                        let error = step
                            .error
                            .clone()
                            .unwrap_or_else(|| "tool invocation failed".to_string());
                        (step.tool_name.clone(), task_id, error)
                    });
                match failed {
                    Some((tool_name, failed_step, error)) => RunOutcome::Failed {
                        error,
                        tool_name,
                        failed_step,
                        steps_done,
                    },
                    None => RunOutcome::Failed {
                        error: "plan execution failed".into(),
                        tool_name: None,
                        failed_step: None,
                        steps_done,
                    },
                }
            }
        }
    }

    /// Handles a failed/timeout plan run: retry (feedback-aware replan that
    /// re-attempts the failed task), drop-through by replanning, or terminate
    /// when nothing can progress.
    ///
    /// Returns `Some(replanned)` when the loop should continue with the
    /// revised plan, or `None` when the session reached a terminal state.
    async fn recover_failure(
        &self,
        state: &Arc<SessionState>,
        plan: &ExecutionPlan,
        completed: &mut [Uuid],
        failed_step: Option<Uuid>,
        error: String,
        timed_out: bool,
    ) -> Option<ExecutionPlan> {
        // Variable-resolution failures cannot ever be healed by replanning.
        if error.contains("unresolved variable") || error.contains("invalid template") {
            self.set_status(state, AutonomousStatus::Failed).await;
            let err = format!("Unrecoverable resolution failure: {error}");
            self.set_error(state, err.clone()).await;
            self.record_reasoning(state, ReasoningPhase::Terminal, err, None)
                .await;
            self.capture_session(state).await;
            return None;
        }

        let (policy, retries, replans, retries_left, replans_left) = {
            let inner = state.inner.read().await;
            (
                inner.policy.clone(),
                inner.retries_used,
                inner.replans_used,
                inner.retries_used < inner.policy.retry.max_attempts,
                inner.replans_used < inner.policy.budget.max_replans,
            )
        };

        // Timeouts only count toward the retry budget when configured.
        let retryable = if timed_out {
            policy.retry.retry_on_timeout
        } else {
            true
        };
        let can_retry = retryable && retries_left;

        if !can_retry && !replans_left {
            let reason = format!(
                "Recovery budget exhausted ({} retries, {} replans): {error}",
                retries, replans
            );
            self.set_status(state, AutonomousStatus::Failed).await;
            self.set_error(state, reason.clone()).await;
            self.record_reasoning(state, ReasoningPhase::Terminal, reason, None)
                .await;
            self.capture_session(state).await;
            return None;
        }

        let feedback = ReplanFeedback {
            failed: failed_step,
            tool_name: None,
            error: Some(error.clone()),
            retry_exhausted: !can_retry,
        };

        // Consult execution memory (RC-6 M1): surface strategies that
        // failed for similar goals before retrying/replanning, so the
        // runtime visibly avoids repeating past mistakes.
        if let Some(memory) = &self.memory {
            let (goal, workspace_id) = (plan.goal.clone(), plan.workspace_id);
            match memory.avoid(&goal, workspace_id, 3).await {
                Ok(avoided) if !avoided.is_empty() => {
                    let top = &avoided[0];
                    self.record_reasoning(
                        state,
                        ReasoningPhase::Replanning,
                        format!(
                            "Memory: avoiding a previously failed strategy — {}",
                            top.failure
                        ),
                        Some(serde_json::json!({
                            "memory_id": top.record.id,
                            "similarity": top.similarity,
                        })),
                    )
                    .await;
                }
                _ => {}
            }
        }

        if can_retry {
            let mut inner = state.inner.write().await;
            inner.retries_used += 1;
            inner.updated_at = Utc::now();
            drop(inner);
            self.record_reasoning(
                state,
                ReasoningPhase::Replanning,
                "Retrying the failed plan run with the same step",
                Some(serde_json::json!({ "retries_used": retries + 1 })),
            )
            .await;
        } else {
            let mut inner = state.inner.write().await;
            inner.replans_used += 1;
            inner.updated_at = Utc::now();
            drop(inner);
            self.record_reasoning(
                state,
                ReasoningPhase::Replanning,
                "Replanning around the failed step",
                None,
            )
            .await;
        }

        match self
            .planner
            .replan_with_feedback(plan, completed, &feedback)
        {
            Ok(replanned) => {
                let mut inner = state.inner.write().await;
                inner.current_plan = Some(replanned.clone());
                inner.updated_at = Utc::now();
                drop(inner);
                self.publish(state).await;
                Some(replanned)
            }
            Err(err) => {
                let reason = format!("Unable to replan: {err}");
                self.set_status(state, AutonomousStatus::Failed).await;
                self.set_error(state, reason.clone()).await;
                self.record_reasoning(state, ReasoningPhase::Terminal, reason, None)
                    .await;
                self.capture_session(state).await;
                None
            }
        }
    }

    async fn approval_gate(
        &self,
        state: &Arc<SessionState>,
        plan: &ExecutionPlan,
    ) -> Result<GateDecision, String> {
        let policy = {
            let inner = state.inner.read().await;
            inner.policy.clone()
        };
        let (required, reason) = approval_required(&policy.approval, plan, &self.tool_meta);
        if !required {
            return Ok(GateDecision::Proceed);
        }

        let request = ApprovalRequest {
            request_id: Uuid::new_v4(),
            session_id: state.session_id,
            goal: plan.goal.clone(),
            plan: plan.clone(),
            reason,
            requested_at: Utc::now(),
            decided_at: None,
            approved: None,
            note: None,
        };
        {
            let mut inner = state.inner.write().await;
            inner.pending_approval = Some(request.clone());
            inner.status = AutonomousStatus::WaitingApproval;
            inner.updated_at = Utc::now();
        }
        self.record_reasoning(
            state,
            ReasoningPhase::AwaitingApproval,
            format!("Approval requested: {}", plan.goal),
            Some(serde_json::json!({ "request_id": request.request_id })),
        )
        .await;
        self.publish(state).await;

        // Wait for the operator decision (approve/reject) or timeout/cancel.
        loop {
            let decided = {
                let inner = state.inner.read().await;
                inner.pending_approval.as_ref().and_then(|p| p.approved)
            };
            match decided {
                Some(true) => {
                    let mut inner = state.inner.write().await;
                    inner.pending_approval = None;
                    inner.status = AutonomousStatus::Running;
                    inner.updated_at = Utc::now();
                    return Ok(GateDecision::Proceed);
                }
                Some(false) => {
                    let mut inner = state.inner.write().await;
                    inner.pending_approval = None;
                    inner.status = AutonomousStatus::Cancelled;
                    inner.updated_at = Utc::now();
                    self.record_reasoning(
                        state,
                        ReasoningPhase::Terminal,
                        "Plan rejected by operator",
                        None,
                    )
                    .await;
                    return Ok(GateDecision::Rejected);
                }
                None => {}
            }

            if state.token.is_cancelled() {
                return Err("session cancelled".into());
            }

            let approval_timeout_s = {
                let inner = state.inner.read().await;
                inner.policy.timeout.approval_timeout_s()
            };
            let notified = state.approval_notify.notified();
            if approval_timeout_s > 0 {
                tokio::select! {
                    _ = state.token.cancelled() => return Err("session cancelled".into()),
                    _ = notified => {}
                    _ = tokio::time::sleep(Duration::from_secs(approval_timeout_s)) => {
                        // Auto-reject when no human decision arrives in time.
                        {
                            let mut inner = state.inner.write().await;
                            if let Some(pending) = inner.pending_approval.as_mut() {
                                pending.approved = Some(false);
                                pending.note = Some("approval timeout".to_string());
                                pending.decided_at = Some(Utc::now());
                            }
                        }
                    }
                }
            } else {
                tokio::select! {
                    _ = state.token.cancelled() => return Err("session cancelled".into()),
                    _ = notified => {}
                }
            }
        }
    }

    /// Waits while the session/engine is paused, resuming automatically once
    /// `resume_session` flips the engine status back to running.
    async fn wait_for_resume(&self, state: &Arc<SessionState>) {
        loop {
            if state.token.is_cancelled() {
                return;
            }
            let status = {
                let inner = state.inner.read().await;
                inner.status
            };
            match status {
                AutonomousStatus::Running | AutonomousStatus::WaitingApproval => return,
                AutonomousStatus::Completed
                | AutonomousStatus::Failed
                | AutonomousStatus::Cancelled => return,
                AutonomousStatus::Paused => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn finish_completed(
        &self,
        state: &Arc<SessionState>,
        plan: ExecutionPlan,
        steps_done: usize,
        completed: Vec<Uuid>,
    ) {
        // Attach the planner report to the final execution stream so the
        // existing dashboard shows the run's accounting.
        if let Some(execution_id) = {
            let inner = state.inner.read().await;
            inner.last_execution_id
        } {
            let report = PlannerReport {
                plan: plan.clone(),
                execution_id: Some(execution_id),
                completed: completed.clone(),
                skipped: Vec::new(),
                replaced: Vec::new(),
                replan_count: {
                    let inner = state.inner.read().await;
                    inner.replans_used as usize
                },
                error: None,
            };
            let _ = self
                .engine
                .attach_planner_report(execution_id, report)
                .await;
        }

        {
            let mut inner = state.inner.write().await;
            inner.plans_completed += 1;
            inner.steps_completed += steps_done as u64;
            inner.status = AutonomousStatus::Completed;
            inner.execution_id = None;
            inner.current_plan = Some(plan.clone());
            inner.updated_at = Utc::now();
        }
        self.record_reasoning(
            state,
            ReasoningPhase::Terminal,
            format!("Goal reached: {} steps completed", steps_done),
            Some(serde_json::json!({ "goal": plan.goal })),
        )
        .await;
        self.capture_session(state).await;
    }

    async fn budget_breach(&self, state: &Arc<SessionState>) -> Option<String> {
        let (budget, steps_completed) = {
            let inner = state.inner.read().await;
            (inner.policy.budget.clone(), inner.steps_completed)
        };
        if budget.max_steps > 0 && steps_completed >= budget.max_steps as u64 {
            return Some(format!(
                "step budget ({}) exhausted after {} steps",
                budget.max_steps, steps_completed
            ));
        }
        if budget.max_duration_seconds > 0
            && state.started_at.elapsed().as_secs() >= budget.max_duration_seconds
        {
            return Some(format!(
                "duration budget ({}s) exhausted",
                budget.max_duration_seconds
            ));
        }
        let plans_used = {
            let inner = state.inner.read().await;
            inner.plans_attempted
        };
        if budget.max_plans > 0 && plans_used >= budget.max_plans {
            return Some(format!(
                "plan budget ({}) exhausted after {} plans",
                budget.max_plans, plans_used
            ));
        }
        None
    }

    // ------------------------------------------------------------------
    // State helpers
    // ------------------------------------------------------------------

    async fn session(&self, session_id: Uuid) -> Result<Arc<SessionState>, AutonomousRuntimeError> {
        self.sessions
            .read()
            .await
            .get(&session_id)
            .cloned()
            .ok_or(AutonomousRuntimeError::NotFound(session_id))
    }

    async fn set_status(&self, state: &Arc<SessionState>, status: AutonomousStatus) {
        let mut inner = state.inner.write().await;
        inner.status = status;
        inner.updated_at = Utc::now();
    }

    async fn set_error(&self, state: &Arc<SessionState>, error: String) {
        let mut inner = state.inner.write().await;
        inner.error = Some(error);
        inner.updated_at = Utc::now();
    }

    async fn note_steps_done(&self, state: &Arc<SessionState>, steps: usize) {
        let mut inner = state.inner.write().await;
        inner.steps_completed += steps as u64;
        inner.updated_at = Utc::now();
    }

    async fn note_attempted_tool(&self, state: &Arc<SessionState>, tool: Option<&str>) {
        if let Some(tool) = tool {
            let mut inner = state.inner.write().await;
            inner.attempted_tools.insert(tool.to_string());
        }
    }

    async fn record_reasoning(
        &self,
        state: &Arc<SessionState>,
        phase: ReasoningPhase,
        message: impl Into<String>,
        detail: Option<serde_json::Value>,
    ) {
        let event = ReasoningEvent::new(state.session_id, phase, message, detail);
        if let Some(emitter) = &self.event_emitter {
            emit(emitter.as_ref(), EVENT_AUTONOMOUS_REASONING, &event);
        }
        let mut inner = state.inner.write().await;
        inner.reasoning.push_back(event);
        while inner.reasoning.len() > REASONING_EVENT_CAP {
            inner.reasoning.pop_front();
        }
        inner.updated_at = Utc::now();
    }

    async fn publish(&self, state: &Arc<SessionState>) {
        if self.event_emitter.is_none() {
            return;
        }
        if let Ok(progress) = self.progress_for(state).await {
            if let Some(emitter) = &self.event_emitter {
                emit(emitter.as_ref(), EVENT_AUTONOMOUS_SESSION, &progress);
            }
        }
    }

    /// Records the session's terminal snapshot into execution memory
    /// (RC-6 M1). Best-effort; a capture failure never affects the loop.
    async fn capture_session(&self, state: &Arc<SessionState>) {
        let Some(memory) = &self.memory else {
            return;
        };
        match self.progress_for(state).await {
            Ok(progress) => {
                if let Err(error) = memory.record_autonomous_session(&progress).await {
                    tracing::warn!(error = %error, "autonomous session memory capture failed");
                }
            }
            Err(error) => {
                tracing::warn!(error = %error, "session snapshot unavailable for memory capture");
            }
        }
    }

    async fn progress_for(
        &self,
        state: &Arc<SessionState>,
    ) -> Result<AutonomousSessionProgress, AutonomousRuntimeError> {
        let inner = state.inner.read().await;
        Ok(AutonomousSessionProgress {
            session_id: state.session_id,
            workspace_id: inner.workspace_id,
            goal: inner.goal.clone(),
            status: inner.status,
            policy: inner.policy.clone(),
            reasoning: inner.reasoning.iter().cloned().collect(),
            current_plan: inner.current_plan.clone(),
            execution_id: inner.execution_id,
            last_execution_id: inner.last_execution_id,
            plans_attempted: inner.plans_attempted,
            plans_completed: inner.plans_completed,
            steps_completed: inner.steps_completed,
            retries_used: inner.retries_used,
            replans_used: inner.replans_used,
            steps_left: inner
                .policy
                .budget
                .max_steps
                .saturating_sub(inner.steps_completed as usize) as u64,
            error: inner.error.clone(),
            pending_approval: inner.pending_approval.clone(),
            created_at: inner.created_at,
            updated_at: inner.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::execution_engine::ExecutionEngine;
    use crate::copilot::execution_repository::ExecutionRepository;
    use crate::copilot::memory::vector::LocalVectorProvider;
    use crate::copilot::memory::{MemoryEngine, MemoryKind, MemoryRepository, MemorySearchRequest};
    use crate::copilot::planner::Planner;
    use crate::copilot::tools::ToolPermissionService;
    use crate::database::test_database;
    use crate::repositories::{
        FileRepository, SettingsRepository, TimelineRepository, WorkspaceRepository,
    };
    use crate::services::{TimelineService, WorkspaceService};
    use crate::session::SessionEngine;
    use crate::timeline::recorder::TimelineRecorder;
    use crate::timeline::TimelineEngine;

    fn workspace_id(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    /// Seeds the workspaces the workflow's `resume_workspace` tail step and
    /// binding plans reference (mirrors the planner test harness).
    async fn seed_workspaces(pool: &sqlx::SqlitePool) {
        for n in 1..=4u8 {
            let id = workspace_id(n);
            let now = Utc::now();
            sqlx::query(
                "INSERT OR IGNORE INTO workspaces
                    (id, name, description, status, health_score, root_path, last_active_at, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(format!("Workspace {n}"))
            .bind::<Option<String>>(None)
            .bind(crate::models::workspace::WorkspaceStatus::Active.as_str())
            .bind(0.0_f64)
            .bind::<Option<String>>(None)
            .bind(now)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .expect("workspace seeding should succeed");
        }
    }

    /// Builds the shared planner + engine + executor stack, mirroring the
    /// planner's integration harness, so a session can plan and execute a
    /// real (in-memory, sqlite-backed) goal.
    async fn execution_stack(
        pool: &sqlx::SqlitePool,
    ) -> (Arc<Planner>, Arc<ExecutionEngine>, Arc<ToolExecutor>) {
        let workspace_repo = WorkspaceRepository::new(pool.clone());
        let file_repo = FileRepository::new(pool.clone());
        let timeline_repo = TimelineRepository::new(pool.clone());

        let workspace_service =
            Arc::new(WorkspaceService::new(workspace_repo, timeline_repo.clone()));
        let session_engine = Arc::new(SessionEngine::new(
            TimelineRepository::new(pool.clone()),
            FileRepository::new(pool.clone()),
        ));
        let timeline_engine = Arc::new(TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(file_repo, timeline_repo.clone()),
            timeline_repo,
        )));

        let permission_service = Arc::new(
            ToolPermissionService::new(SettingsRepository::new(pool.clone()))
                .await
                .expect("permission service should initialize"),
        );

        let executor = Arc::new(
            ToolExecutor::new(workspace_service, session_engine, timeline_engine)
                .with_permission_service(permission_service.clone()),
        );

        let engine = Arc::new(ExecutionEngine::new(
            Arc::new(ExecutionRepository::new(pool.clone())),
            executor.clone(),
        ));
        let planner = Arc::new(
            Planner::new(executor.clone(), Some(permission_service))
                .with_execution_engine(engine.clone()),
        );

        (planner, engine, executor)
    }

    #[tokio::test]
    async fn session_drives_a_completed_goal() {
        let (database, _guard) = test_database().await;
        let pool = database.pool().clone();
        seed_workspaces(&pool).await;
        let (planner, engine, executor) = execution_stack(&pool).await;
        let runtime = AutonomousRuntime::new(planner, engine, executor);

        let progress = runtime
            .start_session(None, "resume the most recent workspace", None)
            .await
            .expect("session should start");
        assert_eq!(progress.status, AutonomousStatus::Running);
        let session_id = progress.session_id;

        let terminal = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let snapshot = runtime
                    .get_progress(session_id)
                    .await
                    .expect("session should exist");
                if snapshot.terminal() {
                    return snapshot;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("session should reach a terminal state");

        match terminal.status {
            AutonomousStatus::Completed => {
                assert!(
                    terminal.steps_completed >= 1,
                    "completed session should have executed at least one step"
                );
            }
            AutonomousStatus::Failed => {
                panic!(
                    "session should not fail for a seedable workspace: {:?}",
                    terminal.error
                );
            }
            other => panic!("unexpected terminal status: {:?}", other),
        }
        assert!(terminal.plans_attempted >= 1);
    }

    #[tokio::test]
    async fn cancel_marks_session_terminal_and_rejects_noop() {
        let (database, _guard) = test_database().await;
        let pool = database.pool().clone();
        seed_workspaces(&pool).await;
        let (planner, engine, executor) = execution_stack(&pool).await;
        let runtime = AutonomousRuntime::new(planner, engine, executor);

        let progress = runtime
            .start_session(None, "resume the most recent workspace", None)
            .await
            .expect("session should start");
        let session_id = progress.session_id;

        let cancelled = runtime
            .cancel_session(session_id)
            .await
            .expect("cancel should succeed");
        assert_eq!(cancelled.status, AutonomousStatus::Cancelled);
        assert!(cancelled.terminal());

        // A second cancel on a terminal session is a no-op/error path.
        assert!(runtime.cancel_session(session_id).await.is_err());
    }

    #[tokio::test]
    async fn empty_goal_is_rejected() {
        let (database, _guard) = test_database().await;
        let pool = database.pool().clone();
        let (planner, engine, executor) = execution_stack(&pool).await;
        let runtime = AutonomousRuntime::new(planner, engine, executor);

        match runtime.start_session(None, "   ", None).await {
            Err(AutonomousRuntimeError::Planning(_)) => {}
            other => panic!("expected planning error, got {:?}", other),
        }
    }

    #[test]
    fn approval_required_respects_policy_mode() {
        let plan = ExecutionPlan {
            id: Uuid::new_v4(),
            workspace_id: None,
            goal: "g".into(),
            tasks: vec![crate::copilot::proactive_models::PlanTask {
                id: Uuid::new_v4(),
                description: "resume".into(),
                dependencies: vec![],
                estimated_minutes: 1,
                required_files: vec![],
                tool_name: Some("resume_workspace".into()),
                arguments: None,
                completed: false,
                condition: None,
            }],
            estimated_duration_minutes: 0,
            required_files: vec![],
            checkpoints: vec![],
            confidence: 0.9,
            reasoning: "".into(),
            status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
            created_at: Utc::now(),
        };
        let meta = ToolRiskLookup {
            confirmation: vec!["resume_workspace".into()],
            high_risk: vec![],
        };
        let auto = ApprovalPolicy::default();
        assert!(!approval_required(&auto, &plan, &meta).0);

        let manual = ApprovalPolicy {
            mode: ApprovalMode::Manual,
            gate_replans: false,
        };
        assert!(approval_required(&manual, &plan, &meta).0);
    }

    #[tokio::test]
    async fn completed_session_is_captured_into_memory() {
        let (database, _guard) = test_database().await;
        let pool = database.pool().clone();
        seed_workspaces(&pool).await;
        let memory = Arc::new(MemoryEngine::new(
            MemoryRepository::new(pool.clone()),
            Arc::new(LocalVectorProvider::default()),
        ));

        let (planner, engine, executor) = execution_stack(&pool).await;
        let runtime = AutonomousRuntime::new(planner, engine, executor).with_memory(memory.clone());

        let progress = runtime
            .start_session(None, "resume the most recent workspace", None)
            .await
            .expect("session should start");
        let session_id = progress.session_id;

        let terminal = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let snapshot = runtime
                    .get_progress(session_id)
                    .await
                    .expect("session should exist");
                if snapshot.terminal() {
                    return snapshot;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("session should reach a terminal state");

        if terminal.status != AutonomousStatus::Completed {
            panic!(
                "session should complete for a seedable workspace: {:?}",
                terminal.error
            );
        }

        let hits = memory
            .search(&MemorySearchRequest {
                query: "resume the most recent workspace".into(),
                kind: Some(MemoryKind::AutonomousSession),
                workspace_id: None,
                status: None,
                limit: 10,
            })
            .await
            .expect("memory search should succeed");
        assert_eq!(
            hits.len(),
            1,
            "terminal sessions must be captured into execution memory"
        );
        assert_eq!(hits[0].record.source_id, session_id);
        assert!(matches!(
            hits[0].record.status,
            crate::copilot::memory::MemoryStatus::Success
        ));
        assert!(
            !hits[0].record.reasoning.is_empty(),
            "session reasoning should be remembered"
        );
    }
}
