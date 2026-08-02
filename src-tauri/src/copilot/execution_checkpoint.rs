//! Execution checkpoints - durable snapshot of a running execution.
//!
//! A checkpoint captures everything needed to reconstruct an execution after
//! a pause or an application restart: the plan DAG (dependencies, gates,
//! ordering), the resolved [`ExecutionContext`], the execution status, and
//! which steps already completed / were skipped / failed. One checkpoint row
//! exists per execution, written after every completed step and on pause.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::execution::ExecutionStatus;
use crate::copilot::execution_context::ExecutionContext;
use crate::copilot::proactive_models::ExecutionPlan;

/// Serializable snapshot of one execution at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionCheckpoint {
    /// The execution this checkpoint belongs to (one row per execution).
    pub execution_id: Uuid,
    /// The plan being executed, including the full task DAG (dependencies,
    /// conditional gates, ordering) so the scheduler can be reconstructed.
    pub plan: ExecutionPlan,
    /// The execution-scoped variable store (step outputs, shared variables).
    pub context: ExecutionContext,
    /// Execution status captured at save time (`Running`/`Paused`).
    pub status: ExecutionStatus,
    /// Step numbers that have completed successfully.
    pub completed_steps: Vec<usize>,
    /// Step numbers that were skipped.
    pub skipped_steps: Vec<usize>,
    /// Step numbers that failed.
    pub failed_steps: Vec<usize>,
    /// When the checkpoint row was first created.
    pub created_at: DateTime<Utc>,
    /// When the checkpoint row was last written.
    pub updated_at: DateTime<Utc>,
}

impl ExecutionCheckpoint {
    /// Builds a fresh checkpoint for the given execution.
    pub fn new(
        execution_id: Uuid,
        plan: ExecutionPlan,
        context: ExecutionContext,
        status: ExecutionStatus,
        completed_steps: Vec<usize>,
        skipped_steps: Vec<usize>,
        failed_steps: Vec<usize>,
    ) -> Self {
        let now = Utc::now();
        Self {
            execution_id,
            plan,
            context,
            status,
            completed_steps,
            skipped_steps,
            failed_steps,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::execution_context::ExecutionContext;
    use crate::copilot::proactive_models::{PlanApprovalStatus, PlanGate, PlanTask};

    fn sample_plan() -> ExecutionPlan {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        ExecutionPlan {
            id: Uuid::new_v4(),
            workspace_id: Some(Uuid::from_bytes([7; 16])),
            goal: "serialize checkpoint".into(),
            tasks: vec![
                PlanTask {
                    id: a,
                    description: "first".into(),
                    dependencies: vec![],
                    estimated_minutes: 1,
                    required_files: vec![],
                    tool_name: Some("list_workspaces".into()),
                    arguments: Some(serde_json::json!({})),
                    completed: false,
                    condition: None,
                },
                PlanTask {
                    id: b,
                    description: "second".into(),
                    dependencies: vec![a],
                    estimated_minutes: 1,
                    required_files: vec![],
                    tool_name: Some("get_workspace".into()),
                    arguments: Some(
                        serde_json::json!({ "workspace_id": "{{steps.list_workspaces[0].id}}" }),
                    ),
                    completed: false,
                    condition: Some(PlanGate::AfterSuccess(a)),
                },
            ],
            estimated_duration_minutes: 2,
            required_files: vec![],
            checkpoints: vec![],
            confidence: 0.8,
            reasoning: "test".into(),
            status: PlanApprovalStatus::Pending,
            created_at: Utc::now(),
        }
    }

    fn sample_context() -> ExecutionContext {
        let mut ctx = ExecutionContext::new(
            Some(Uuid::from_bytes([7; 16])),
            "serialize checkpoint".into(),
        );
        ctx.set_step_output(
            0,
            Some("list_workspaces"),
            serde_json::json!({
                "workspaces": [
                    { "id": "w1", "path": "/one", "nested": { "a": [1, 2.5, "three"] } }
                ],
                "active": { "id": "w1" }
            }),
        );
        ctx.set_variable("boosted", serde_json::json!(true));
        ctx
    }

    #[test]
    fn checkpoint_serializes_and_deserializes() {
        let execution_id = Uuid::new_v4();
        let plan = sample_plan();
        let checkpoint = ExecutionCheckpoint::new(
            execution_id,
            plan.clone(),
            sample_context(),
            ExecutionStatus::Pending,
            vec![0],
            vec![],
            vec![],
        );

        let json = serde_json::to_value(&checkpoint).expect("checkpoint serializes");
        let round_tripped: ExecutionCheckpoint =
            serde_json::from_value(json).expect("checkpoint deserializes");

        assert_eq!(round_tripped.execution_id, execution_id);
        assert_eq!(round_tripped.status, ExecutionStatus::Pending);
        assert_eq!(round_tripped.completed_steps, vec![0]);
        assert_eq!(
            round_tripped.plan.tasks.len(),
            plan.tasks.len(),
            "plan DAG must survive round-trip"
        );
        assert_eq!(
            round_tripped.plan.tasks[1].condition,
            Some(PlanGate::AfterSuccess(plan.tasks[0].id)),
            "conditional gate must survive round-trip"
        );
    }

    #[test]
    fn context_survives_checkpoint_round_trip_without_type_loss() {
        let checkpoint = ExecutionCheckpoint::new(
            Uuid::new_v4(),
            sample_plan(),
            sample_context(),
            ExecutionStatus::Running,
            vec![0],
            vec![],
            vec![],
        );

        let json = serde_json::to_value(&checkpoint).unwrap();
        let restored: ExecutionCheckpoint = serde_json::from_value(json).unwrap();

        // Arrays, nested objects, numbers and booleans must be preserved.
        let output = restored
            .context
            .step_output("list_workspaces")
            .expect("output restored");
        assert_eq!(output["workspaces"][0]["id"], "w1");
        assert_eq!(output["workspaces"][0]["path"], "/one");
        assert_eq!(output["workspaces"][0]["nested"]["a"][0], 1);
        assert_eq!(output["workspaces"][0]["nested"]["a"][1], 2.5);
        assert_eq!(output["workspaces"][0]["nested"]["a"][2], "three");
        assert_eq!(output["active"]["id"], "w1");

        // Shared variables restored too, so downstream steps still resolve.
        let goal = restored
            .context
            .resolve_string("{{goal}}")
            .expect("goal resolves");
        assert_eq!(goal, serde_json::json!("serialize checkpoint"));
        let ws = restored
            .context
            .resolve_string("{{steps.list_workspaces.workspaces[0].id}}")
            .expect("downstream variable resolves after round-trip");
        assert_eq!(ws, serde_json::json!("w1"));
    }
}
