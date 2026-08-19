//! Plan Execution Engine - Orchestrates plan execution with progress tracking and control.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::app_events::{emit, AppEventEmitter, EVENT_EXECUTION_PROGRESS};
use crate::copilot::execution::*;
use crate::copilot::execution_checkpoint::ExecutionCheckpoint;
use crate::copilot::execution_context::{ExecutionContext, UNRESOLVED_VARIABLE_MARKER};
use crate::copilot::execution_repository::ExecutionRepository;
use crate::copilot::memory::MemoryEngine;
use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::{ExecutionPlan, PlanGate, PlanTask};
use crate::copilot::tools::{ToolExecutor, ToolInvocationRequest, ToolInvocationStatus};
use crate::errors::DatabaseError;

struct ActiveExecutionState {
    cancellation_token: CancellationToken,
    workspace_id: Option<Uuid>,
    /// Dependency graph of the plan being executed, aligned to the persisted
    /// steps by index (steps are created in `plan.tasks` order). Lets the
    /// scheduler pick the next runnable step based on plan and gates.
    tasks: Option<Vec<PlanTask>>,
    /// The full plan being executed, so active-page checkpoints can persist
    /// the DAG (dependencies, gates, ordering) for later reconstruction.
    plan: Option<ExecutionPlan>,
    /// Execution-scoped variable store: step outputs, structured results and
    /// shared context resolved `{{...}}` templates before a tool runs.
    context: ExecutionContext,
}

/// Engine for executing approved plans.
#[derive(Clone)]
pub struct ExecutionEngine {
    repository: Arc<ExecutionRepository>,
    tool_executor: Arc<ToolExecutor>,
    active_executions: Arc<RwLock<std::collections::HashMap<Uuid, ActiveExecutionState>>>,
    /// Planner reports keyed by execution id, attached when a planner-driven
    /// run completes so the streamed/queried progress carries retry accounting.
    planner_reports: Arc<RwLock<std::collections::HashMap<Uuid, PlannerReport>>>,
    /// Frontend event emitter for live `execution:progress` snapshots.
    event_emitter: Option<Arc<dyn AppEventEmitter>>,
    /// Execution memory capture (RC-6 M1). Terminal states are recorded
    /// here so ContextSphere learns from every run. Optional; captures are
    /// best-effort and never affect the execution lifecycle.
    memory: Option<Arc<MemoryEngine>>,
}

impl ExecutionEngine {
    /// Creates a new execution engine.
    pub fn new(repository: Arc<ExecutionRepository>, tool_executor: Arc<ToolExecutor>) -> Self {
        Self {
            repository,
            tool_executor,
            active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            planner_reports: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_emitter: None,
            memory: None,
        }
    }

    /// Attaches a frontend event emitter for streamed execution progress.
    pub fn with_event_emitter(mut self, emitter: Arc<dyn AppEventEmitter>) -> Self {
        self.event_emitter = Some(emitter);
        self
    }

    /// Attaches the execution memory store so terminal runs are captured
    /// for learning (RC-6 M1).
    pub fn with_memory(mut self, memory: Arc<MemoryEngine>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Starts execution of an approved plan.
    pub async fn start_execution(
        &self,
        plan: &ExecutionPlan,
        conversation_id: Option<Uuid>,
    ) -> Result<Uuid, DatabaseError> {
        let execution = self
            .repository
            .create_execution(plan.id, conversation_id, plan.tasks.len())
            .await?;

        let execution_id = execution.id;

        for (idx, task) in plan.tasks.iter().enumerate() {
            let step = ExecutionStep {
                id: Uuid::new_v4(),
                execution_id,
                step_number: idx,
                description: task.description.clone(),
                tool_name: task.tool_name.clone(),
                arguments: task.arguments.clone(),
                status: StepStatus::Pending,
                result: None,
                error: None,
                started_at: None,
                completed_at: None,
                created_at: chrono::Utc::now(),
            };
            self.repository.create_step(step).await?;
        }

        self.repository
            .record_audit(
                execution_id,
                "execution_started",
                AuditActor::User,
                "Plan execution initiated by user",
            )
            .await?;

        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Running, None)
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Started,
            step_number: None,
            message: format!("Starting execution of plan: {}", plan.goal),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        let execution = self.repository.get_execution(execution_id).await?.unwrap();
        self.active_executions.write().await.insert(
            execution.id,
            ActiveExecutionState {
                cancellation_token: CancellationToken::new(),
                workspace_id: plan.workspace_id,
                tasks: if plan.tasks.is_empty() {
                    None
                } else {
                    Some(plan.tasks.clone())
                },
                plan: Some(plan.clone()),
                context: ExecutionContext::new(plan.workspace_id, plan.goal.clone()),
            },
        );

        self.publish_progress(execution_id).await?;

        Ok(execution_id)
    }

    /// Executes the next step in a plan.
    pub fn execute_next_step(
        &self,
        execution_id: Uuid,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), DatabaseError>> + Send + '_>>
    {
        Box::pin(async move { self.execute_next_step_impl(execution_id).await })
    }

    /// Drives an execution to a terminal state (or until it is paused).
    ///
    /// Before scheduling each step it re-reads the persisted execution
    /// status; on `Completed`/`Cancelled`/`Paused`/`Failed` it returns
    /// `Ok(())` without attempting another step, so a pause mid-run never
    /// surfaces as a lifecycle error here. `execute_next_step` keeps its own
    /// guards for direct callers.
    pub async fn execute_until_complete(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        loop {
            let Some(execution) = self.repository.get_execution(execution_id).await? else {
                return Err(DatabaseError::IoError(format!(
                    "Execution not found: {}",
                    execution_id
                )));
            };
            match execution.status {
                ExecutionStatus::Completed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::Paused
                | ExecutionStatus::Failed => return Ok(()),
                ExecutionStatus::Pending | ExecutionStatus::Running => {}
            }
            self.execute_next_step(execution_id).await?;
        }
    }

    /// Picks the next step to run: the first non-terminal step whose
    /// dependencies (from the plan DAG) are all completed and whose
    /// conditional gate, if any, is satisfied. Without an attached plan,
    /// steps simply run in the persisted order.
    async fn next_runnable_step_index(
        &self,
        execution_id: Uuid,
        execution: &PlanExecution,
        steps: &[ExecutionStep],
    ) -> Result<Option<usize>, DatabaseError> {
        let plan_tasks = {
            let guard = self.active_executions.read().await;
            guard
                .get(&execution_id)
                .and_then(|state| state.tasks.clone())
        };

        let Some(tasks) = plan_tasks else {
            let next = steps
                .get(execution.current_step)
                .map(|_| execution.current_step);
            return Ok(next);
        };

        let outcomes: std::collections::HashMap<Uuid, ToolInvocationStatus> = tasks
            .iter()
            .zip(steps.iter())
            .map(|(task, step)| {
                let status = match step.status {
                    StepStatus::Completed => ToolInvocationStatus::Success,
                    StepStatus::Skipped => ToolInvocationStatus::Cancelled,
                    StepStatus::Failed => ToolInvocationStatus::Failed,
                    StepStatus::Pending | StepStatus::Running => ToolInvocationStatus::Pending,
                };
                (task.id, status)
            })
            .collect();

        for (index, step) in steps.iter().enumerate() {
            if step.status != StepStatus::Pending {
                continue;
            }
            let Some(task) = tasks.get(index) else {
                continue;
            };
            let dependencies_satisfied = task.dependencies.iter().all(|dependency| {
                tasks.iter().enumerate().any(|(i, candidate)| {
                    candidate.id == *dependency
                        && steps.get(i).is_some_and(|dep_step| {
                            dep_step.status == StepStatus::Completed
                                || dep_step.status == StepStatus::Skipped
                        })
                })
            });
            if !dependencies_satisfied {
                continue;
            }
            let gate_satisfied = match task.condition {
                None => true,
                Some(PlanGate::AfterSuccess(predecessor)) => outcomes
                    .get(&predecessor)
                    .map(|s| matches!(s, ToolInvocationStatus::Success))
                    .unwrap_or(false),
                Some(PlanGate::AfterFailure(predecessor)) => outcomes
                    .get(&predecessor)
                    .map(|s| matches!(s, ToolInvocationStatus::Failed))
                    .unwrap_or(false),
            };
            if gate_satisfied {
                return Ok(Some(index));
            }
        }

        Ok(None)
    }

    /// Internal implementation of execute_next_step.
    async fn execute_next_step_impl(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        let execution = self.repository.get_execution(execution_id).await?;
        let execution = execution.ok_or_else(|| {
            DatabaseError::IoError(format!("Execution not found: {}", execution_id))
        })?;

        if execution.status != ExecutionStatus::Running {
            return Err(DatabaseError::IoError(format!(
                "Execution not in running state: {:?}",
                execution.status
            )));
        }

        let steps = self.repository.get_execution_steps(execution_id).await?;

        let Some(step_index) = self
            .next_runnable_step_index(execution_id, &execution, &steps)
            .await?
        else {
            if steps.iter().any(|s| s.status == StepStatus::Pending) {
                self.fail_execution(
                    execution_id,
                    "no runnable step: unsatisfied dependencies or conditional gates",
                )
                .await?;
            } else {
                self.complete_execution(execution_id).await?;
            }
            return Ok(());
        };

        let step = &steps[step_index];

        self.repository
            .update_step_status(step.id, StepStatus::Running, None, None)
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::StepStarted,
            step_number: Some(step.step_number),
            message: format!(
                "Starting step {}: {}",
                step.step_number + 1,
                step.description
            ),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;
        self.publish_progress(execution_id).await?;

        let (cancellation_token, workspace_id) = self.execution_context(execution_id).await;

        let result = if let Some(tool_name) = &step.tool_name {
            let context = self.context_snapshot(execution_id).await;

            let arguments = match &context {
                Some(context) => match context.resolve(
                    &step
                        .arguments
                        .clone()
                        .unwrap_or_else(|| serde_json::json!({})),
                ) {
                    Ok(resolved) => resolved,
                    Err(error) => {
                        let error_msg = if error.to_string().starts_with(UNRESOLVED_VARIABLE_MARKER)
                        {
                            error.to_string()
                        } else {
                            format!("{}: {}", UNRESOLVED_VARIABLE_MARKER, error)
                        };
                        self.repository
                            .update_step_status(
                                step.id,
                                StepStatus::Failed,
                                None,
                                Some(error_msg.clone()),
                            )
                            .await?;
                        let event = ExecutionEvent {
                            id: Uuid::new_v4(),
                            execution_id,
                            event_type: ExecutionEventType::StepFailed,
                            step_number: Some(step.step_number),
                            message: format!("Failed step {}: {}", step.step_number + 1, error_msg),
                            metadata: None,
                            created_at: chrono::Utc::now(),
                        };
                        self.repository.record_event(event).await?;
                        self.fail_execution(execution_id, &error_msg).await?;
                        return Ok(());
                    }
                },
                None => step
                    .arguments
                    .clone()
                    .unwrap_or_else(|| serde_json::json!({})),
            };

            self.tool_executor
                .invoke_tool_with_context(ToolInvocationRequest {
                    tool_name: tool_name.clone(),
                    arguments,
                    workspace_id,
                    cancellation_token: cancellation_token.clone(),
                })
                .await
        } else {
            self.repository
                .update_step_status(
                    step.id,
                    StepStatus::Skipped,
                    Some(
                        serde_json::json!({"status": "skipped", "reason": "No tool specified"})
                            .to_string(),
                    ),
                    None,
                )
                .await?;
            self.repository
                .update_current_step(execution_id, step_index + 1)
                .await?;
            self.save_checkpoint(execution_id).await?;
            self.publish_progress(execution_id).await?;
            return Ok(());
        };

        match result {
            Ok(invocation) if invocation.status == ToolInvocationStatus::Success => {
                let invocation_json = serde_json::to_value(&invocation)?;
                self.repository
                    .update_step_status(
                        step.id,
                        StepStatus::Completed,
                        Some(invocation_json.to_string()),
                        None,
                    )
                    .await?;

                // Store the structured tool result in the execution context so
                // downstream steps can bind `{{steps.<name>.<path>}}`.
                if let Some(context) = invocation.result.as_ref() {
                    let mut guard = self.active_executions.write().await;
                    if let Some(state) = guard.get_mut(&execution_id) {
                        state.context.set_step_output(
                            step.step_number,
                            step.tool_name.as_deref(),
                            context.clone(),
                        );
                    }
                }

                let event = ExecutionEvent {
                    id: Uuid::new_v4(),
                    execution_id,
                    event_type: ExecutionEventType::StepCompleted,
                    step_number: Some(step.step_number),
                    message: format!(
                        "Completed step {}: {}",
                        step.step_number + 1,
                        step.description
                    ),
                    metadata: Some(invocation_json),
                    created_at: chrono::Utc::now(),
                };
                self.repository.record_event(event).await?;

                self.repository
                    .update_current_step(execution_id, step_index + 1)
                    .await?;
                self.save_checkpoint(execution_id).await?;
                self.publish_progress(execution_id).await?;
            }
            Ok(invocation) if invocation.status == ToolInvocationStatus::Cancelled => {
                let invocation_json = serde_json::to_value(&invocation)?;
                self.repository
                    .update_step_status(
                        step.id,
                        StepStatus::Failed,
                        Some(invocation_json.clone().to_string()),
                        invocation.error.clone(),
                    )
                    .await?;
                self.cancel_execution(execution_id).await?;
            }
            Ok(invocation) => {
                let error_msg = invocation
                    .error
                    .clone()
                    .unwrap_or_else(|| "tool invocation failed".to_string());
                let invocation_json = serde_json::to_value(&invocation)?;
                self.repository
                    .update_step_status(
                        step.id,
                        StepStatus::Failed,
                        Some(invocation_json.clone().to_string()),
                        Some(error_msg.clone()),
                    )
                    .await?;

                let event = ExecutionEvent {
                    id: Uuid::new_v4(),
                    execution_id,
                    event_type: ExecutionEventType::StepFailed,
                    step_number: Some(step.step_number),
                    message: format!("Failed step {}: {}", step.step_number + 1, error_msg),
                    metadata: Some(invocation_json),
                    created_at: chrono::Utc::now(),
                };
                self.repository.record_event(event).await?;
                self.fail_execution(execution_id, &error_msg).await?;
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.repository
                    .update_step_status(step.id, StepStatus::Failed, None, Some(error_msg.clone()))
                    .await?;

                let event = ExecutionEvent {
                    id: Uuid::new_v4(),
                    execution_id,
                    event_type: ExecutionEventType::StepFailed,
                    step_number: Some(step.step_number),
                    message: format!("Failed step {}: {}", step.step_number + 1, error_msg),
                    metadata: None,
                    created_at: chrono::Utc::now(),
                };
                self.repository.record_event(event).await?;

                self.fail_execution(execution_id, &error_msg).await?;
            }
        }

        Ok(())
    }

    /// Pauses an execution.
    ///
    /// The current (already-started) tool is allowed to finish; the persisted
    /// status flip to `Paused` is what stops `execute_until_complete` before
    /// it schedules the *next* tool. A checkpoint is saved so the execution
    /// can be resumed later (possibly after an application restart).
    pub async fn pause_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Paused, None)
            .await?;
        self.save_checkpoint(execution_id).await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Paused,
            step_number: None,
            message: "Execution paused by user".to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        self.repository
            .record_audit(
                execution_id,
                "execution_paused",
                AuditActor::User,
                "User paused execution",
            )
            .await?;

        self.publish_progress(execution_id).await?;

        Ok(())
    }

    /// Resumes a paused execution.
    ///
    /// If the in-memory [`ActiveExecutionState`] still exists (same-process
    /// pause), it is reused as-is. Otherwise — e.g. after an application
    /// restart — the checkpoint row is loaded and a fresh active state is
    /// rebuilt from the stored plan + `ExecutionContext`, so execution
    /// continues from the next runnable step and already-completed steps are
    /// never re-run.
    pub async fn resume_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        let has_active = self
            .active_executions
            .read()
            .await
            .contains_key(&execution_id);
        if !has_active {
            match self.repository.get_checkpoint(execution_id).await? {
                Some(checkpoint) => {
                    let plan = checkpoint.plan;
                    let tasks = if plan.tasks.is_empty() {
                        None
                    } else {
                        Some(plan.tasks.clone())
                    };
                    self.active_executions.write().await.insert(
                        execution_id,
                        ActiveExecutionState {
                            cancellation_token: CancellationToken::new(),
                            workspace_id: plan.workspace_id,
                            tasks,
                            plan: Some(plan),
                            context: checkpoint.context,
                        },
                    );

                    let event = ExecutionEvent {
                        id: Uuid::new_v4(),
                        execution_id,
                        event_type: ExecutionEventType::CheckpointLoaded,
                        step_number: None,
                        message: "Checkpoint loaded; resuming execution".to_string(),
                        metadata: Some(serde_json::json!({
                            "completed": checkpoint.completed_steps,
                            "skipped": checkpoint.skipped_steps,
                            "failed": checkpoint.failed_steps,
                        })),
                        created_at: chrono::Utc::now(),
                    };
                    self.repository.record_event(event).await?;
                }
                None => {
                    return Err(DatabaseError::IoError(format!(
                        "No checkpoint available for execution: {}",
                        execution_id
                    )));
                }
            }
        }

        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Running, None)
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Resumed,
            step_number: None,
            message: "Execution resumed".to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        self.repository
            .record_audit(
                execution_id,
                "execution_resumed",
                AuditActor::User,
                "User resumed execution",
            )
            .await?;

        self.publish_progress(execution_id).await?;

        Ok(())
    }

    /// Cancels an execution.
    pub async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        if let Some(active) = self.active_executions.read().await.get(&execution_id) {
            active.cancellation_token.cancel();
        }

        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Cancelled, None)
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Cancelled,
            step_number: None,
            message: "Execution cancelled by user".to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        self.repository
            .record_audit(
                execution_id,
                "execution_cancelled",
                AuditActor::User,
                "User cancelled execution",
            )
            .await?;

        // Emit the final snapshot while the active plan is still indexed so the
        // dashboard shows the DAG through the cancelled terminal state.
        self.publish_progress(execution_id).await?;

        self.capture_memory(execution_id, ExecutionStatus::Cancelled, None)
            .await;

        self.repository.delete_checkpoint(execution_id).await?;
        self.active_executions.write().await.remove(&execution_id);

        Ok(())
    }

    /// Snapshot of the current execution context (cloned), for inspection and
    /// for variable resolution in `execute_next_step_impl`.
    async fn context_snapshot(&self, execution_id: Uuid) -> Option<ExecutionContext> {
        self.active_executions
            .read()
            .await
            .get(&execution_id)
            .map(|state| state.context.clone())
    }

    async fn execution_context(
        &self,
        execution_id: Uuid,
    ) -> (Option<CancellationToken>, Option<Uuid>) {
        self.active_executions
            .read()
            .await
            .get(&execution_id)
            .map_or((None, None), |state| {
                (Some(state.cancellation_token.clone()), state.workspace_id)
            })
    }

    /// Persists a checkpoint for the execution and records a
    /// `CheckpointSaved` event. Called after every step completes/skips and
    /// on pause, so the stored checkpoint never lags behind the execution
    /// context by more than one step. No-ops when the execution has no
    /// active planner plan (backward-compatible sequential mode).
    async fn save_checkpoint(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        let Some((plan, context)) = self
            .active_executions
            .read()
            .await
            .get(&execution_id)
            .map(|state| (state.plan.clone(), state.context.clone()))
        else {
            return Ok(());
        };
        let Some(plan) = plan else {
            return Ok(());
        };

        let steps = self.repository.get_execution_steps(execution_id).await?;
        let mut completed = Vec::new();
        let mut skipped = Vec::new();
        let mut failed = Vec::new();
        for step in &steps {
            match step.status {
                StepStatus::Completed => completed.push(step.step_number),
                StepStatus::Skipped => skipped.push(step.step_number),
                StepStatus::Failed => failed.push(step.step_number),
                StepStatus::Pending | StepStatus::Running => {}
            }
        }
        let status = self
            .repository
            .get_execution(execution_id)
            .await?
            .map(|e| e.status)
            .unwrap_or(ExecutionStatus::Running);

        let checkpoint = ExecutionCheckpoint::new(
            execution_id,
            plan,
            context,
            status,
            completed,
            skipped,
            failed,
        );
        self.repository.save_checkpoint(&checkpoint).await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::CheckpointSaved,
            step_number: None,
            message: format!(
                "Checkpoint saved with {} completed step(s)",
                checkpoint.completed_steps.len()
            ),
            metadata: Some(serde_json::json!({
                "completed": checkpoint.completed_steps,
                "skipped": checkpoint.skipped_steps,
                "failed": checkpoint.failed_steps,
            })),
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        Ok(())
    }

    /// Gets execution progress.
    pub async fn get_progress(
        &self,
        execution_id: Uuid,
    ) -> Result<ExecutionProgress, DatabaseError> {
        let execution = self.repository.get_execution(execution_id).await?;
        let execution = execution.ok_or_else(|| {
            DatabaseError::IoError(format!("Execution not found: {}", execution_id))
        })?;

        let steps = self.repository.get_execution_steps(execution_id).await?;
        let events = self
            .repository
            .get_execution_events(execution_id, 50)
            .await?;

        let completed_steps = steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let progress_percentage = if execution.total_steps > 0 {
            (completed_steps as f64 / execution.total_steps as f64) * 100.0
        } else {
            0.0
        };

        let planner_report = match self.planner_reports.read().await.get(&execution_id) {
            Some(report) => Some(report.clone()),
            // Miss the in-memory cache (e.g. after an application restart):
            // fall back to the durable planner-reports table so the dashboard
            // still shows the summary for a run that completed earlier.
            None => self.repository.get_planner_report(execution_id).await?,
        };

        Ok(ExecutionProgress {
            execution_id,
            status: execution.status,
            current_step: execution.current_step,
            total_steps: execution.total_steps,
            progress_percentage,
            steps,
            recent_events: events,
            plan: self
                .active_executions
                .read()
                .await
                .get(&execution_id)
                .and_then(|state| state.plan.clone()),
            planner_report,
        })
    }

    /// Builds the current [`ExecutionProgress`] snapshot and emits it to the
    /// frontend as an `execution:progress` event. No-op when no emitter is
    /// attached (tests, headless use). Called after every state change so
    /// the Execution Dashboard stays live without polling.
    pub async fn publish_progress(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        let Some(emitter) = &self.event_emitter else {
            return Ok(());
        };
        let progress = self.get_progress(execution_id).await?;
        emit(emitter.as_ref(), EVENT_EXECUTION_PROGRESS, &progress);
        Ok(())
    }

    /// Attaches a planner report to an execution (set once the autonomous
    /// planner finishes a run), then re-emits a progress snapshot carrying
    /// it. The report is also recorded into execution memory (RC-6 M1) so
    /// the learning engine can rank planner-driven runs.
    pub async fn attach_planner_report(
        &self,
        execution_id: Uuid,
        report: PlannerReport,
    ) -> Result<(), DatabaseError> {
        // Persist first so the report survives restarts and reconnect.
        self.repository
            .save_planner_report(execution_id, &report)
            .await?;
        if let Some(memory) = &self.memory {
            if let Err(err) = memory.record_planner_report(&report).await {
                tracing::warn!(error = %err, "planner report memory capture failed");
            }
        }
        self.planner_reports
            .write()
            .await
            .insert(execution_id, report);
        self.publish_progress(execution_id).await
    }

    /// Lists recent executions (newest first), each with full progress so the
    /// dashboard can re-attach after a reload/restart.
    pub async fn list_recent(&self, limit: usize) -> Result<Vec<ExecutionProgress>, DatabaseError> {
        let executions = self.repository.list_recent_executions(limit).await?;
        let mut progress = Vec::with_capacity(executions.len());
        for execution in executions {
            progress.push(self.get_progress(execution.id).await?);
        }
        Ok(progress)
    }

    /// Completes an execution successfully.
    async fn complete_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Completed, None)
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Completed,
            step_number: None,
            message: "Execution completed successfully".to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        self.repository
            .record_audit(
                execution_id,
                "execution_completed",
                AuditActor::System,
                "All steps completed successfully",
            )
            .await?;

        // Emit the final snapshot while the active plan is still indexed so the
        // dashboard shows the DAG through the completed terminal state.
        self.publish_progress(execution_id).await?;

        self.capture_memory(execution_id, ExecutionStatus::Completed, None)
            .await;

        self.repository.delete_checkpoint(execution_id).await?;
        self.active_executions.write().await.remove(&execution_id);

        Ok(())
    }

    /// Fails an execution.
    async fn fail_execution(&self, execution_id: Uuid, error: &str) -> Result<(), DatabaseError> {
        self.repository
            .update_execution_status(
                execution_id,
                ExecutionStatus::Failed,
                Some(error.to_string()),
            )
            .await?;

        let event = ExecutionEvent {
            id: Uuid::new_v4(),
            execution_id,
            event_type: ExecutionEventType::Failed,
            step_number: None,
            message: format!("Execution failed: {}", error),
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        self.repository.record_event(event).await?;

        self.repository
            .record_audit(
                execution_id,
                "execution_failed",
                AuditActor::System,
                &format!("Execution failed: {}", error),
            )
            .await?;

        // Emit the final snapshot while the active plan is still indexed so the
        // dashboard shows the DAG through the failed terminal state.
        self.publish_progress(execution_id).await?;

        self.capture_memory(
            execution_id,
            ExecutionStatus::Failed,
            Some(error.to_string()),
        )
        .await;

        self.repository.delete_checkpoint(execution_id).await?;
        self.active_executions.write().await.remove(&execution_id);

        Ok(())
    }

    /// Records the run into execution memory (RC-6 M1). Best-effort: a
    /// capture failure is logged and never fails the execution lifecycle.
    /// Called on terminal states while the active plan and checkpoint are
    /// still present.
    async fn capture_memory(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
        error: Option<String>,
    ) {
        let Some(memory) = &self.memory else {
            return;
        };
        let (plan, workspace_id) = self
            .active_executions
            .read()
            .await
            .get(&execution_id)
            .map(|state| (state.plan.clone(), state.workspace_id))
            .unwrap_or((None, None));
        let goal = plan
            .as_ref()
            .map(|p| p.goal.clone())
            .unwrap_or_else(|| "unknown goal".to_string());
        let steps = match self.repository.get_execution_steps(execution_id).await {
            Ok(steps) => steps,
            Err(err) => {
                tracing::warn!(error = %err, "memory capture could not load steps");
                return;
            }
        };
        // Completion time (RC-6 M3): wall-clock duration of the run, when
        // the execution recorded its start/finish timestamps.
        let duration_seconds = self
            .repository
            .get_execution(execution_id)
            .await
            .ok()
            .flatten()
            .and_then(|execution| {
                execution
                    .started_at
                    .zip(execution.completed_at)
                    .map(|(started, completed)| (completed - started).num_seconds().max(0) as u64)
            });
        if let Err(err) = memory
            .record_execution(
                execution_id,
                workspace_id,
                &goal,
                plan.as_ref(),
                &steps,
                status,
                error,
                duration_seconds,
            )
            .await
        {
            tracing::warn!(error = %err, "memory capture failed for execution");
        }
    }
}
