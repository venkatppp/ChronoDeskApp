//! Plan Execution Engine - Orchestrates plan execution with progress tracking and control.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::copilot::execution::*;
use crate::copilot::execution_repository::ExecutionRepository;
use crate::copilot::proactive_models::ExecutionPlan;
use crate::copilot::tools::{ToolExecutor, ToolInvocationRequest, ToolInvocationStatus};
use crate::errors::DatabaseError;

struct ActiveExecutionState {
    cancellation_token: CancellationToken,
    workspace_id: Option<Uuid>,
}

/// Engine for executing approved plans.
#[derive(Clone)]
pub struct ExecutionEngine {
    repository: Arc<ExecutionRepository>,
    tool_executor: Arc<ToolExecutor>,
    active_executions: Arc<RwLock<std::collections::HashMap<Uuid, ActiveExecutionState>>>,
}

impl ExecutionEngine {
    /// Creates a new execution engine.
    pub fn new(repository: Arc<ExecutionRepository>, tool_executor: Arc<ToolExecutor>) -> Self {
        Self {
            repository,
            tool_executor,
            active_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
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
            },
        );

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

    pub async fn execute_until_complete(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        loop {
            self.execute_next_step(execution_id).await?;
            let Some(execution) = self.repository.get_execution(execution_id).await? else {
                return Err(DatabaseError::IoError(format!(
                    "Execution not found: {}",
                    execution_id
                )));
            };
            if execution.status != ExecutionStatus::Running {
                return Ok(());
            }
        }
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
        if execution.current_step >= steps.len() {
            self.complete_execution(execution_id).await?;
            return Ok(());
        }

        let step = &steps[execution.current_step];

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

        let (cancellation_token, workspace_id) = self.execution_context(execution_id).await;

        let result = if let (Some(tool_name), Some(arguments)) = (&step.tool_name, &step.arguments)
        {
            self.tool_executor
                .invoke_tool_with_context(ToolInvocationRequest {
                    tool_name: tool_name.clone(),
                    arguments: arguments.clone(),
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
                .update_current_step(execution_id, execution.current_step + 1)
                .await?;
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
                    .update_current_step(execution_id, execution.current_step + 1)
                    .await?;
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
    pub async fn pause_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        self.repository
            .update_execution_status(execution_id, ExecutionStatus::Paused, None)
            .await?;

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

        Ok(())
    }

    /// Resumes a paused execution.
    pub async fn resume_execution(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
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

        self.active_executions.write().await.remove(&execution_id);

        Ok(())
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
            .get_execution_events(execution_id, 10)
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

        Ok(ExecutionProgress {
            execution_id,
            status: execution.status,
            current_step: execution.current_step,
            total_steps: execution.total_steps,
            progress_percentage,
            steps,
            recent_events: events,
        })
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

        self.active_executions.write().await.remove(&execution_id);

        Ok(())
    }
}
