//! Autonomous Planning Engine.
//!
//! Turns a user goal into a structured, dependency-aware [`ExecutionPlan`]
//! that the existing [`ExecutionEngine`] can run: steps carry explicit
//! dependencies (a DAG rather than a flat list), may be gated behind the
//! outcome of an earlier step ([`PlanGate`] conditional execution), and a
//! failed step triggers a bounded replan that revises the remaining work
//! instead of aborting the goal.
//!
//! The planner reuses the shared [`ToolExecutor`] invocation pipeline (the
//! same validation/permission/timeout path used by the existing engines) and
//! consults the persistent [`ToolPermissionService`] when building the plan,
//! so planning never introduces a second execution path.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::copilot::proactive_models::{ExecutionPlan, PlanApprovalStatus, PlanGate, PlanTask};
use crate::copilot::tools::{
    ToolDefinition, ToolExecutor, ToolInvocationRequest, ToolInvocationResult,
    ToolInvocationStatus, ToolPermissionDecision, ToolPermissionLevel, ToolPermissionService,
};
use crate::errors::DatabaseError;

/// Number of replan passes permitted for a single goal before giving up.
pub const MAX_REPLAN_ATTEMPTS: usize = 3;

/// Errors surfaced by the planning pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PlannerError {
    #[error("Planner was cancelled")]
    Cancelled,
    #[error("goal is empty")]
    EmptyGoal,
    #[error("no tools are available for this goal/workspace")]
    NoToolsAvailable,
    #[error("plan contains a dependency cycle")]
    DependencyCycle,
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("database error: {0}")]
    Database(#[from] DatabaseError),
}

/// Outcome of an autonomous plan run.
#[derive(Debug, Clone)]
pub struct PlannerReport {
    pub plan: ExecutionPlan,
    /// Task ids executed successfully, in order.
    pub completed: Vec<Uuid>,
    /// Task ids skipped because a conditional gate was not satisfied.
    pub skipped: Vec<Uuid>,
    /// Task ids that failed and were replaced by replanning.
    pub replaced: Vec<Uuid>,
    /// Number of replan passes actually performed.
    pub replan_count: usize,
    /// First error observed during the run, when any.
    pub error: Option<String>,
}

/// The autonomous planner. Cheap to clone; all state lives behind `Arc`s.
#[derive(Clone)]
pub struct Planner {
    tool_executor: Arc<ToolExecutor>,
    permission_service: Option<Arc<ToolPermissionService>>,
}

impl Planner {
    /// Creates a new planner over the shared tool pipeline.
    pub fn new(
        tool_executor: Arc<ToolExecutor>,
        permission_service: Option<Arc<ToolPermissionService>>,
    ) -> Self {
        Self {
            tool_executor,
            permission_service,
        }
    }

    /// Generates a dependency-aware plan for a goal in a workspace.
    ///
    /// The produced plan is a chain-shaped DAG whose deeper steps depend on
    /// the output of earlier ones, with the final action step gated behind a
    /// [`PlanGate::AfterSuccess`] of its predecessor — modeling conditional
    /// execution that depends on previous results.
    pub async fn plan(
        &self,
        workspace_id: Option<Uuid>,
        cancellation_token: Option<&tokio_util::sync::CancellationToken>,
        goal: &str,
    ) -> Result<ExecutionPlan, PlannerError> {
        if let Some(token) = cancellation_token {
            if token.is_cancelled() {
                return Err(PlannerError::Cancelled);
            }
        }
        if goal.trim().is_empty() {
            return Err(PlannerError::EmptyGoal);
        }

        let tools = self.available_tools(workspace_id).await;
        if tools.is_empty() {
            return Err(PlannerError::NoToolsAvailable);
        }
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

        let workflow = workflow_plan(&tool_names);
        if workflow.is_empty() {
            return Err(PlannerError::NoToolsAvailable);
        }

        let mut tasks: Vec<PlanTask> = Vec::new();
        let mut previous: Option<Uuid> = None;

        for (tool_name, description) in workflow {
            let task_id = Uuid::new_v4();
            let dependencies = previous.into_iter().collect::<Vec<_>>();
            let condition = previous
                .filter(|_| tool_name == "resume_workspace")
                .map(PlanGate::AfterSuccess);

            tasks.push(PlanTask {
                id: task_id,
                description: description.to_string(),
                dependencies,
                estimated_minutes: 5,
                required_files: vec![],
                tool_name: Some(tool_name),
                arguments: None,
                completed: false,
                condition,
            });
            previous = Some(task_id);
        }

        let total_minutes: i32 = tasks.iter().map(|t| t.estimated_minutes).sum();

        Ok(ExecutionPlan {
            id: Uuid::new_v4(),
            workspace_id,
            goal: goal.to_string(),
            tasks,
            estimated_duration_minutes: total_minutes,
            required_files: vec![],
            checkpoints: vec![
                "Context collected".to_string(),
                "Plan steps resolved".to_string(),
                "Action executed".to_string(),
            ],
            confidence: 0.8,
            reasoning: "Deterministic dependency-aware plan built from the tool registry"
                .to_string(),
            status: PlanApprovalStatus::Pending,
            created_at: Utc::now(),
        })
    }

    /// Resolves the topological execution order of a plan's tasks.
    ///
    /// Kahn's algorithm over the dependency edges. Returns
    /// `PlannerError::DependencyCycle` when the graph cannot be linearized.
    pub fn dependency_order(&self, plan: &ExecutionPlan) -> Result<Vec<PlanTask>, PlannerError> {
        topological_order(&plan.tasks)
    }

    /// Evaluates whether a step's conditional gate permits execution given
    /// the outcome of the referenced predecessor.
    pub fn condition_satisfied(
        &self,
        condition: Option<PlanGate>,
        outcomes: &HashMap<Uuid, ToolInvocationStatus>,
    ) -> bool {
        match condition {
            None => true,
            Some(PlanGate::AfterSuccess(predecessor)) => {
                outcomes.get(&predecessor) == Some(&ToolInvocationStatus::Success)
            }
            Some(PlanGate::AfterFailure(predecessor)) => matches!(
                outcomes.get(&predecessor),
                Some(ToolInvocationStatus::Failed) | Some(ToolInvocationStatus::Cancelled)
            ),
        }
    }

    /// Produces a revised plan after a step failed. Completed tasks are kept
    /// out of the way, the failed task is dropped, and the surviving work is
    /// re-linked so execution can continue instead of aborting the goal.
    pub fn replan_after_failure(
        &self,
        plan: &ExecutionPlan,
        completed: &[Uuid],
        failed: Uuid,
    ) -> Result<ExecutionPlan, PlannerError> {
        let kept: Vec<PlanTask> = plan
            .tasks
            .iter()
            .filter(|task| task.id != failed && !completed.contains(&task.id))
            .map(|task| {
                let mut revised = task.clone();
                revised
                    .dependencies
                    .retain(|dep| *dep != failed && !completed.contains(dep));
                revised.condition = None;
                revised
            })
            .collect();

        // Prune surviving tasks whose dependencies no longer exist so the
        // replanned graph stays acyclic and connected.
        let surviving_ids: Vec<Uuid> = kept.iter().map(|t| t.id).collect();
        let mut connected: Vec<PlanTask> = kept
            .into_iter()
            .filter(|task| {
                task.dependencies
                    .iter()
                    .all(|dep| surviving_ids.contains(dep))
            })
            .collect();
        connected.sort_by(|a, b| a.description.cmp(&b.description));

        let mut replanned = plan.clone();
        replanned.status = PlanApprovalStatus::Pending;
        replanned.tasks = connected;
        replanned.confidence = (plan.confidence - 0.1).max(0.4);
        replanned.reasoning = format!(
            "Replanned after task {} failed; remaining steps were re-linked",
            failed
        );
        Ok(replanned)
    }

    /// Executes a goal autonomously: plan → dependency order → conditional
    /// gating → shared `ToolExecutor` invocation → bounded replan on failure.
    pub async fn execute_goal(
        &self,
        workspace_id: Option<Uuid>,
        goal: &str,
        cancellation_token: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<PlannerReport, PlannerError> {
        self.ensure_not_cancelled(cancellation_token)?;

        let mut plan = self.plan(workspace_id, cancellation_token, goal).await?;
        let mut outcomes: HashMap<Uuid, ToolInvocationStatus> = HashMap::new();
        let mut completed: Vec<Uuid> = Vec::new();
        let mut skipped: Vec<Uuid> = Vec::new();
        let mut replaced: Vec<Uuid> = Vec::new();
        let mut replan_count = 0usize;
        let mut last_error: Option<String> = None;

        loop {
            self.ensure_not_cancelled(cancellation_token)?;

            if replan_count > MAX_REPLAN_ATTEMPTS {
                break;
            }

            let ordered = self.dependency_order(&plan)?;
            let mut progressed = false;

            for task in ordered {
                if completed.contains(&task.id) || outcomes.contains_key(&task.id) {
                    continue;
                }

                if !self.condition_satisfied(task.condition, &outcomes) {
                    skipped.push(task.id);
                    outcomes.insert(task.id, ToolInvocationStatus::Cancelled);
                    continue;
                }

                progressed = true;
                match self
                    .invoke_task(workspace_id, &task, &outcomes, cancellation_token)
                    .await
                {
                    Ok(result) if result.status == ToolInvocationStatus::Success => {
                        completed.push(task.id);
                        outcomes.insert(task.id, result.status);
                    }
                    Ok(result) => {
                        replaced.push(task.id);
                        outcomes.insert(task.id, result.status);
                        if let Some(error) = &result.error {
                            last_error = Some(error.clone());
                        }
                    }
                    Err(PlannerError::Cancelled) => return Err(PlannerError::Cancelled),
                    Err(error) => {
                        replaced.push(task.id);
                        outcomes.insert(task.id, ToolInvocationStatus::Failed);
                        last_error = Some(error.to_string());
                    }
                }
            }

            if !progressed {
                break;
            }

            if replaced.is_empty() {
                break;
            }

            let failed = replaced.last().copied().unwrap();
            replaced.clear();
            replan_count += 1;
            plan = self.replan_after_failure(&plan, &completed, failed)?;
        }

        Ok(PlannerReport {
            plan,
            completed,
            skipped,
            replaced,
            replan_count,
            error: last_error,
        })
    }

    /// Invokes a plan task through the shared tool pipeline — the same path
    /// `ExecutionEngine` and the copilot loop use. Only allowed tools run.
    pub async fn invoke_task(
        &self,
        workspace_id: Option<Uuid>,
        task: &PlanTask,
        outcomes: &HashMap<Uuid, ToolInvocationStatus>,
        cancellation_token: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<ToolInvocationResult, PlannerError> {
        let Some(name) = &task.tool_name else {
            return Ok(empty_result(task.id));
        };

        if self.tool_is_denied(name, workspace_id).await {
            return Err(PlannerError::Execution(format!(
                "tool '{}' is not permitted for this workspace",
                name
            )));
        }

        let request = ToolInvocationRequest {
            tool_name: name.clone(),
            arguments: bind_arguments(task, outcomes),
            workspace_id,
            cancellation_token: cancellation_token.cloned(),
        };

        self.tool_executor
            .invoke_tool_with_context(request)
            .await
            .map_err(|e| PlannerError::Execution(e.to_string()))
    }

    /// Tools a goal may reference, excluding any denied by registry metadata
    /// or the persistent permission policy for the workspace.
    pub async fn available_tools(&self, workspace_id: Option<Uuid>) -> Vec<ToolDefinition> {
        let executor_tools = self.tool_executor.available_tools();
        let mut allowed = Vec::new();
        for tool in executor_tools {
            if tool.permission.required_level == ToolPermissionLevel::Denied {
                continue;
            }
            if let Some(permissions) = &self.permission_service {
                if permissions.resolve(&tool.name, workspace_id).await
                    == Some(ToolPermissionDecision::Deny)
                {
                    continue;
                }
            }
            allowed.push(tool);
        }
        allowed
    }

    async fn tool_is_denied(&self, name: &str, workspace_id: Option<Uuid>) -> bool {
        if let Some(permissions) = &self.permission_service {
            if permissions.resolve(name, workspace_id).await == Some(ToolPermissionDecision::Deny) {
                return true;
            }
        }
        self.tool_executor
            .available_tools()
            .into_iter()
            .any(|tool| {
                tool.name == name && tool.permission.required_level == ToolPermissionLevel::Denied
            })
    }

    fn ensure_not_cancelled(
        &self,
        token: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<(), PlannerError> {
        if let Some(token) = token {
            if token.is_cancelled() {
                return Err(PlannerError::Cancelled);
            }
        }
        Ok(())
    }
}

/// Empty result for a step that had no tool. Treated as a successful no-op
/// so the plan chain stays complete.
fn empty_result(_id: Uuid) -> ToolInvocationResult {
    ToolInvocationResult {
        invocation_id: Uuid::new_v4(),
        tool_name: String::new(),
        arguments: serde_json::Value::Null,
        status: ToolInvocationStatus::Success,
        result: None,
        error: None,
        started_at: Utc::now(),
        completed_at: Utc::now(),
        duration_ms: 0,
        attempts: 0,
    }
}

/// Binds a task's arguments. Steps that need a workspace id can be filled
/// from an earlier step's output in a future milestone; today the plan steps
/// use their optional workspace-aware tools with no explicit arguments.
fn bind_arguments(
    task: &PlanTask,
    _outcomes: &HashMap<Uuid, ToolInvocationStatus>,
) -> serde_json::Value {
    if let Some(arguments) = &task.arguments {
        return arguments.clone();
    }
    serde_json::json!({})
}

/// Kahn-style topological order over the plan's task graph.
fn topological_order(tasks: &[PlanTask]) -> Result<Vec<PlanTask>, PlannerError> {
    let mut by_id: HashMap<Uuid, PlanTask> = HashMap::new();
    for task in tasks {
        by_id.insert(task.id, task.clone());
    }

    let mut indegree: HashMap<Uuid, usize> = HashMap::new();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for task in tasks {
        *indegree.entry(task.id).or_insert(0) += 0;
        for dep in &task.dependencies {
            if by_id.contains_key(dep) {
                adjacency.entry(*dep).or_default().push(task.id);
                *indegree.entry(task.id).or_insert(0) += 1;
            }
        }
    }

    let mut queue: VecDeque<Uuid> = tasks
        .iter()
        .filter(|t| indegree.get(&t.id).copied().unwrap_or(0) == 0)
        .map(|t| t.id)
        .collect();

    let mut order: Vec<PlanTask> = Vec::new();
    while let Some(uuid) = queue.pop_front() {
        if let Some(task) = by_id.get(&uuid) {
            order.push(task.clone());
        }
        if let Some(neighbors) = adjacency.get(&uuid) {
            for neighbor in neighbors {
                if let Some(degree) = indegree.get_mut(neighbor) {
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }

    if order.len() != tasks.len() {
        return Err(PlannerError::DependencyCycle);
    }
    Ok(order)
}

/// A deterministic, dependency-aware list of steps built from the tools
/// actually available. Each step leads into the next; a `resume_workspace`
/// action is appended as the gated, conditional tail step.
fn workflow_plan(available: &[String]) -> Vec<(String, String)> {
    let has = |name: &str| available.iter().any(|n| n == name);
    let mut steps: Vec<(String, String)> = Vec::new();

    if has("list_workspaces") {
        steps.push((
            "list_workspaces".to_string(),
            "List active workspaces".to_string(),
        ));
    } else if has("get_active_workspace") {
        steps.push((
            "get_active_workspace".to_string(),
            "Get the active workspace".to_string(),
        ));
    }

    if has("get_recent_events") {
        steps.push((
            "get_recent_events".to_string(),
            "Gather recent workspace activity".to_string(),
        ));
    } else if has("search_timeline") {
        steps.push((
            "search_timeline".to_string(),
            "Search the workspace timeline".to_string(),
        ));
    }

    if has("get_session_summary") {
        steps.push((
            "get_session_summary".to_string(),
            "Summarize the current session".to_string(),
        ));
    }

    if has("resume_workspace") {
        steps.push((
            "resume_workspace".to_string(),
            "Resume focused work".to_string(),
        ));
    }

    steps
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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

    async fn executor() -> (
        Arc<ToolExecutor>,
        Arc<ToolPermissionService>,
        tempfile::TempDir,
    ) {
        let (database, guard) = test_database().await;
        let pool = database.pool().clone();
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

        (executor, permission_service, guard)
    }

    async fn planner() -> (Planner, tempfile::TempDir) {
        let (executor, permission_service, guard) = executor().await;
        (Planner::new(executor, Some(permission_service)), guard)
    }

    fn seed_plan() -> ExecutionPlan {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let tasks = vec![
            PlanTask {
                id: a,
                description: "A".into(),
                dependencies: vec![],
                estimated_minutes: 1,
                required_files: vec![],
                tool_name: Some("list_workspaces".into()),
                arguments: None,
                completed: false,
                condition: None,
            },
            PlanTask {
                id: b,
                description: "B".into(),
                dependencies: vec![a],
                estimated_minutes: 2,
                required_files: vec![],
                tool_name: Some("get_recent_events".into()),
                arguments: None,
                completed: false,
                condition: Some(PlanGate::AfterSuccess(a)),
            },
            PlanTask {
                id: c,
                description: "C".into(),
                dependencies: vec![b],
                estimated_minutes: 3,
                required_files: vec![],
                tool_name: Some("get_session_summary".into()),
                arguments: None,
                completed: false,
                condition: Some(PlanGate::AfterSuccess(b)),
            },
        ];
        ExecutionPlan {
            id: Uuid::new_v4(),
            workspace_id: None,
            goal: "plan generation".into(),
            tasks,
            estimated_duration_minutes: 6,
            required_files: vec![],
            checkpoints: vec![],
            confidence: 0.8,
            reasoning: "test".into(),
            status: PlanApprovalStatus::Pending,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn plan_generation_produces_dependency_aware_steps() {
        let (planner, _guard) = planner().await;
        let cancellation = tokio_util::sync::CancellationToken::new();

        let plan = planner
            .plan(None, Some(&cancellation), "Resume my focus session")
            .await
            .expect("plan should generate");
        assert!(!plan.tasks.is_empty());
        assert!(
            plan.tasks.len() >= 3,
            "expected a chain, got {}",
            plan.tasks.len()
        );

        for task in &plan.tasks {
            assert!(
                task.dependencies
                    .iter()
                    .all(|dep| { plan.tasks.iter().any(|other| other.id == *dep) }),
                "dependency {} must reference an existing task",
                task.id
            );
        }
        assert!(
            plan.tasks.iter().any(|t| t.condition.is_some()),
            "expected at least one conditional gate"
        );

        let ordered = planner
            .dependency_order(&plan)
            .expect("dependency plan must be topologically sortable");
        assert_eq!(ordered.len(), plan.tasks.len());
    }

    #[tokio::test]
    async fn dependency_resolution_orders_and_detects_cycles() {
        let (planner, _guard) = planner().await;

        let plan = seed_plan();
        let ordered = planner
            .dependency_order(&plan)
            .expect("acyclic plan should produce an order");
        assert_eq!(ordered.len(), 3);
        assert_eq!(ordered[0].id, plan.tasks[0].id);

        let mut cyclic = seed_plan();
        let id_a = cyclic.tasks[0].id;
        let id_b = cyclic.tasks[1].id;
        let id_c = cyclic.tasks[2].id;
        cyclic.tasks[1].dependencies = vec![id_a, id_c];
        cyclic.tasks[2].dependencies = vec![id_b];
        assert!(
            matches!(
                planner.dependency_order(&cyclic),
                Err(PlannerError::DependencyCycle)
            ),
            "cycle must be detected"
        );
    }

    #[tokio::test]
    async fn replanning_after_failure_keeps_remaining_steps() {
        let (planner, _guard) = planner().await;
        let original = seed_plan();
        let failed = original.tasks[1].id;

        let replanned = planner
            .replan_after_failure(&original, &[], failed)
            .expect("replan should succeed");
        assert!(
            replanned.tasks.iter().all(|t| t.id != failed),
            "failed task must be dropped"
        );
        assert!(!replanned.tasks.is_empty(), "surviving work must remain");
        assert!(
            replanned.tasks.iter().any(|t| t.condition.is_none()),
            "replanned steps should no longer gate on the failed predecessor"
        );
        assert!(replanned.confidence < original.confidence);
    }

    #[tokio::test]
    async fn conditional_execution_honors_gates() {
        let (planner, _guard) = planner().await;
        let mut outcomes = HashMap::new();
        let precedent = Uuid::new_v4();

        assert!(planner.condition_satisfied(None, &outcomes));

        assert!(!planner.condition_satisfied(Some(PlanGate::AfterSuccess(precedent)), &outcomes));
        outcomes.insert(precedent, ToolInvocationStatus::Success);
        assert!(planner.condition_satisfied(Some(PlanGate::AfterSuccess(precedent)), &outcomes));

        outcomes.insert(precedent, ToolInvocationStatus::Success);
        assert!(!planner.condition_satisfied(Some(PlanGate::AfterFailure(precedent)), &outcomes));
        outcomes.insert(precedent, ToolInvocationStatus::Failed);
        assert!(planner.condition_satisfied(Some(PlanGate::AfterFailure(precedent)), &outcomes));
    }

    #[tokio::test]
    async fn planner_cancellation_returns_cancelled() {
        let (planner, _guard) = planner().await;
        let tok = tokio_util::sync::CancellationToken::new();
        tok.cancel();

        let result = planner
            .plan(Some(workspace_id(1)), Some(&tok), "resume work")
            .await;
        assert!(
            matches!(result, Err(PlannerError::Cancelled)),
            "plan must abort on a cancelled token"
        );

        let result = planner
            .execute_goal(Some(workspace_id(1)), "resume work", Some(&tok))
            .await;
        assert!(matches!(result, Err(PlannerError::Cancelled)));
    }
}
