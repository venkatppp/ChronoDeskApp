//! Plan Execution Repository - Database operations for plan execution tracking.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::copilot::execution::*;
use crate::copilot::execution_checkpoint::ExecutionCheckpoint;
use crate::copilot::execution_context::ExecutionContext;
use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::ExecutionPlan;
use crate::errors::DatabaseError;

/// Repository for plan execution persistence.
#[derive(Clone)]
pub struct ExecutionRepository {
    pool: SqlitePool,
}

impl ExecutionRepository {
    /// Creates a new execution repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new plan execution record.
    pub async fn create_execution(
        &self,
        plan_id: Uuid,
        conversation_id: Option<Uuid>,
        total_steps: usize,
    ) -> Result<PlanExecution, DatabaseError> {
        let execution = PlanExecution {
            id: Uuid::new_v4(),
            plan_id,
            conversation_id,
            status: ExecutionStatus::Pending,
            current_step: 0,
            total_steps,
            started_at: None,
            completed_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        sqlx::query(
            r#"
            INSERT INTO plan_executions (
                id, plan_id, conversation_id, status, current_step, total_steps,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(execution.id.to_string())
        .bind(execution.plan_id.to_string())
        .bind(execution.conversation_id.map(|id| id.to_string()))
        .bind(execution.status.to_string())
        .bind(execution.current_step as i64)
        .bind(execution.total_steps as i64)
        .bind(execution.created_at.to_rfc3339())
        .bind(execution.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(execution)
    }

    /// Gets an execution by ID.
    pub async fn get_execution(
        &self,
        execution_id: Uuid,
    ) -> Result<Option<PlanExecution>, DatabaseError> {
        type ExecutionRow = (
            String,
            String,
            Option<String>,
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
        );

        let row: Option<ExecutionRow> = sqlx::query_as(
            r#"
            SELECT id, plan_id, conversation_id, status, current_step, total_steps,
                   started_at, completed_at, error, created_at, updated_at
            FROM plan_executions
            WHERE id = ?
            "#,
        )
        .bind(execution_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some((
            id,
            plan_id,
            conversation_id,
            status,
            current_step,
            total_steps,
            started_at,
            completed_at,
            error,
            created_at,
            updated_at,
        )) = row
        {
            Ok(Some(PlanExecution {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                plan_id: Uuid::parse_str(&plan_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                conversation_id: conversation_id
                    .map(|s| Uuid::parse_str(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                status: self.parse_execution_status(&status)?,
                current_step: current_step as usize,
                total_steps: total_steps as usize,
                started_at: started_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: completed_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                error,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
            }))
        } else {
            Ok(None)
        }
    }

    /// Updates execution status.
    pub async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
        error: Option<String>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let started_at = if status == ExecutionStatus::Running {
            Some(now.to_rfc3339())
        } else {
            None
        };
        let completed_at = if matches!(
            status,
            ExecutionStatus::Completed | ExecutionStatus::Failed | ExecutionStatus::Cancelled
        ) {
            Some(now.to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE plan_executions
            SET status = ?,
                error = ?,
                started_at = COALESCE(started_at, ?),
                completed_at = COALESCE(?, completed_at),
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(error)
        .bind(started_at)
        .bind(completed_at)
        .bind(now.to_rfc3339())
        .bind(execution_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates current step.
    pub async fn update_current_step(
        &self,
        execution_id: Uuid,
        step: usize,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE plan_executions
            SET current_step = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(step as i64)
        .bind(Utc::now().to_rfc3339())
        .bind(execution_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Creates an execution step.
    pub async fn create_step(&self, step: ExecutionStep) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO plan_execution_steps (
                id, execution_id, step_number, description, tool_name, arguments,
                status, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(step.id.to_string())
        .bind(step.execution_id.to_string())
        .bind(step.step_number as i64)
        .bind(&step.description)
        .bind(step.tool_name.as_deref())
        .bind(step.arguments.as_ref().map(|v| v.to_string()))
        .bind(step.status.to_string())
        .bind(step.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates step status.
    pub async fn update_step_status(
        &self,
        step_id: Uuid,
        status: StepStatus,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let started_at = if status == StepStatus::Running {
            Some(now.to_rfc3339())
        } else {
            None
        };
        let completed_at = if matches!(status, StepStatus::Completed | StepStatus::Failed) {
            Some(now.to_rfc3339())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE plan_execution_steps
            SET status = ?,
                result = ?,
                error = ?,
                started_at = COALESCE(started_at, ?),
                completed_at = COALESCE(?, completed_at)
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(result)
        .bind(error)
        .bind(started_at)
        .bind(completed_at)
        .bind(step_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets all steps for an execution.
    pub async fn get_execution_steps(
        &self,
        execution_id: Uuid,
    ) -> Result<Vec<ExecutionStep>, DatabaseError> {
        type StepRow = (
            String,
            String,
            i64,
            String,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        );

        let rows: Vec<StepRow> = sqlx::query_as(
            r#"
            SELECT id, execution_id, step_number, description, tool_name, arguments,
                   status, result, error, started_at, completed_at, created_at
            FROM plan_execution_steps
            WHERE execution_id = ?
            ORDER BY step_number ASC
            "#,
        )
        .bind(execution_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut steps = Vec::new();
        for (
            id,
            exec_id,
            step_number,
            description,
            tool_name,
            arguments,
            status,
            result,
            error,
            started_at,
            completed_at,
            created_at,
        ) in rows
        {
            steps.push(ExecutionStep {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                execution_id: Uuid::parse_str(&exec_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                step_number: step_number as usize,
                description,
                tool_name,
                arguments: arguments
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok()),
                status: self.parse_step_status(&status)?,
                result,
                error,
                started_at: started_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: completed_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(steps)
    }

    /// Records an execution event.
    pub async fn record_event(&self, event: ExecutionEvent) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO plan_execution_events (
                id, execution_id, event_type, step_number, message, metadata, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.id.to_string())
        .bind(event.execution_id.to_string())
        .bind(event.event_type.to_string())
        .bind(event.step_number.map(|n| n as i64))
        .bind(&event.message)
        .bind(event.metadata.as_ref().map(|v| v.to_string()))
        .bind(event.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets recent events for an execution.
    pub async fn get_execution_events(
        &self,
        execution_id: Uuid,
        limit: usize,
    ) -> Result<Vec<ExecutionEvent>, DatabaseError> {
        type EventRow = (
            String,
            String,
            String,
            Option<i64>,
            String,
            Option<String>,
            String,
        );

        let rows: Vec<EventRow> = sqlx::query_as(
            r#"
                SELECT id, execution_id, event_type, step_number, message, metadata, created_at
                FROM plan_execution_events
                WHERE execution_id = ?
                ORDER BY created_at DESC
                LIMIT ?
                "#,
        )
        .bind(execution_id.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for (id, exec_id, event_type, step_number, message, metadata, created_at) in rows {
            events.push(ExecutionEvent {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                execution_id: Uuid::parse_str(&exec_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                event_type: self.parse_event_type(&event_type)?,
                step_number: step_number.map(|n| n as usize),
                message,
                metadata: metadata.as_ref().and_then(|s| serde_json::from_str(s).ok()),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        events.reverse(); // Return in chronological order
        Ok(events)
    }

    /// Records an audit log entry.
    pub async fn record_audit(
        &self,
        execution_id: Uuid,
        action: &str,
        actor: AuditActor,
        details: &str,
    ) -> Result<(), DatabaseError> {
        let audit = ExecutionAudit {
            id: Uuid::new_v4(),
            execution_id,
            action: action.to_string(),
            actor,
            details: details.to_string(),
            created_at: Utc::now(),
        };

        sqlx::query(
            r#"
            INSERT INTO plan_execution_audit (
                id, execution_id, action, actor, details, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(audit.id.to_string())
        .bind(audit.execution_id.to_string())
        .bind(&audit.action)
        .bind(audit.actor.to_string())
        .bind(&audit.details)
        .bind(audit.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Upserts the checkpoint for an execution. One row per execution — a
    /// second write overwrites the previous snapshot entirely.
    pub async fn save_checkpoint(
        &self,
        checkpoint: &ExecutionCheckpoint,
    ) -> Result<(), DatabaseError> {
        let plan = serde_json::to_string(&checkpoint.plan)?;
        let context = serde_json::to_string(&checkpoint.context)?;
        let completed = serde_json::to_string(&checkpoint.completed_steps)?;
        let skipped = serde_json::to_string(&checkpoint.skipped_steps)?;
        let failed = serde_json::to_string(&checkpoint.failed_steps)?;

        sqlx::query(
            r#"
            INSERT INTO plan_execution_checkpoints (
                execution_id, plan, context, status,
                completed_steps, skipped_steps, failed_steps,
                created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(execution_id) DO UPDATE SET
                plan = excluded.plan,
                context = excluded.context,
                status = excluded.status,
                completed_steps = excluded.completed_steps,
                skipped_steps = excluded.skipped_steps,
                failed_steps = excluded.failed_steps,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(checkpoint.execution_id.to_string())
        .bind(plan)
        .bind(context)
        .bind(checkpoint.status.to_string())
        .bind(completed)
        .bind(skipped)
        .bind(failed)
        .bind(checkpoint.updated_at.to_rfc3339())
        .bind(checkpoint.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Loads the saved checkpoint for an execution, if any.
    pub async fn get_checkpoint(
        &self,
        execution_id: Uuid,
    ) -> Result<Option<ExecutionCheckpoint>, DatabaseError> {
        type Row = (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        );

        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT execution_id, plan, context, status,
                   completed_steps, skipped_steps, failed_steps,
                   created_at, updated_at
            FROM plan_execution_checkpoints
            WHERE execution_id = ?
            "#,
        )
        .bind(execution_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let Some((
            _,
            plan_json,
            context_json,
            status,
            completed_json,
            skipped_json,
            failed_json,
            created_at,
            updated_at,
        )) = row
        else {
            return Ok(None);
        };

        Ok(Some(ExecutionCheckpoint {
            execution_id,
            plan: serde_json::from_str::<ExecutionPlan>(&plan_json)?,
            context: serde_json::from_str::<ExecutionContext>(&context_json)?,
            status: self.parse_execution_status(&status)?,
            completed_steps: serde_json::from_str(&completed_json)?,
            skipped_steps: serde_json::from_str(&skipped_json)?,
            failed_steps: serde_json::from_str(&failed_json)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| DatabaseError::IoError(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map_err(|e| DatabaseError::IoError(e.to_string()))?
                .with_timezone(&Utc),
        }))
    }

    /// Deletes the checkpoint for an execution (used on terminal states).
    pub async fn delete_checkpoint(&self, execution_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM plan_execution_checkpoints WHERE execution_id = ?")
            .bind(execution_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Persists the planner's final run summary for an execution (UPSERT so a
    /// replan run keeps only its last report per execution).
    pub async fn save_planner_report(
        &self,
        execution_id: Uuid,
        report: &PlannerReport,
    ) -> Result<(), DatabaseError> {
        let report_json = serde_json::to_string(report)?;
        sqlx::query(
            r#"
            INSERT INTO plan_execution_reports (execution_id, report, created_at)
            VALUES (?, ?, ?)
            ON CONFLICT(execution_id) DO UPDATE SET
                report = excluded.report,
                created_at = excluded.created_at
            "#,
        )
        .bind(execution_id.to_string())
        .bind(report_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Loads the planner report attached to an execution, if any.
    pub async fn get_planner_report(
        &self,
        execution_id: Uuid,
    ) -> Result<Option<PlannerReport>, DatabaseError> {
        type Row = (String,);
        let row: Option<Row> = sqlx::query_as(
            r#"
            SELECT report FROM plan_execution_reports WHERE execution_id = ?
            "#,
        )
        .bind(execution_id.to_string())
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((report_json,)) => Ok(Some(serde_json::from_str(&report_json)?)),
            None => Ok(None),
        }
    }

    /// Lists the most recently updated executions, newest first, so the
    /// dashboard can re-attach to an in-flight or last-completed run.
    pub async fn list_recent_executions(
        &self,
        limit: usize,
    ) -> Result<Vec<PlanExecution>, DatabaseError> {
        type Row = (
            String,
            String,
            Option<String>,
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            String,
        );

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            SELECT id, plan_id, conversation_id, status, current_step, total_steps,
                   started_at, completed_at, error, created_at, updated_at
            FROM plan_executions
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut executions = Vec::new();
        for (
            id,
            plan_id,
            conversation_id,
            status,
            current_step,
            total_steps,
            started_at,
            completed_at,
            error,
            created_at,
            updated_at,
        ) in rows
        {
            executions.push(PlanExecution {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                plan_id: Uuid::parse_str(&plan_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                conversation_id: conversation_id
                    .map(|s| Uuid::parse_str(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                status: self.parse_execution_status(&status)?,
                current_step: current_step as usize,
                total_steps: total_steps as usize,
                started_at: started_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: completed_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
                error,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(executions)
    }

    /// Helper: parse execution status.
    fn parse_execution_status(&self, s: &str) -> Result<ExecutionStatus, DatabaseError> {
        match s {
            "pending" => Ok(ExecutionStatus::Pending),
            "running" => Ok(ExecutionStatus::Running),
            "paused" => Ok(ExecutionStatus::Paused),
            "completed" => Ok(ExecutionStatus::Completed),
            "failed" => Ok(ExecutionStatus::Failed),
            "cancelled" => Ok(ExecutionStatus::Cancelled),
            _ => Err(DatabaseError::IoError(format!(
                "Unknown execution status: {}",
                s
            ))),
        }
    }

    /// Helper: parse step status.
    fn parse_step_status(&self, s: &str) -> Result<StepStatus, DatabaseError> {
        match s {
            "pending" => Ok(StepStatus::Pending),
            "running" => Ok(StepStatus::Running),
            "completed" => Ok(StepStatus::Completed),
            "failed" => Ok(StepStatus::Failed),
            "skipped" => Ok(StepStatus::Skipped),
            _ => Err(DatabaseError::IoError(format!(
                "Unknown step status: {}",
                s
            ))),
        }
    }

    /// Helper: parse event type.
    fn parse_event_type(&self, s: &str) -> Result<ExecutionEventType, DatabaseError> {
        match s {
            "started" => Ok(ExecutionEventType::Started),
            "step_started" => Ok(ExecutionEventType::StepStarted),
            "step_completed" => Ok(ExecutionEventType::StepCompleted),
            "step_failed" => Ok(ExecutionEventType::StepFailed),
            "paused" => Ok(ExecutionEventType::Paused),
            "resumed" => Ok(ExecutionEventType::Resumed),
            "checkpoint_saved" => Ok(ExecutionEventType::CheckpointSaved),
            "checkpoint_loaded" => Ok(ExecutionEventType::CheckpointLoaded),
            "completed" => Ok(ExecutionEventType::Completed),
            "failed" => Ok(ExecutionEventType::Failed),
            "cancelled" => Ok(ExecutionEventType::Cancelled),
            _ => Err(DatabaseError::IoError(format!("Unknown event type: {}", s))),
        }
    }
}
