//! Memory Engine - the public facade over execution memory: captures runs
//! into the store (executions, planner reports, autonomous sessions), runs
//! semantic retrieval, and serves the learning engine's recommendations.
//!
//! The engine only *persists and retrieves*. Deciding what to do with a
//! recommendation belongs to the planner / autonomous runtime.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::copilot::autonomous::models::AutonomousSessionProgress;
use crate::copilot::autonomous::AutonomousStatus;
use crate::copilot::execution::{ExecutionStatus, ExecutionStep};
use crate::copilot::memory::learning;
use crate::copilot::memory::models::{
    goal_fingerprint, outcome_from_report, AvoidedStrategy, ExecutionMemoryRecord, LearnedWorkflow,
    MemoryHit, MemoryKind, MemoryOutcome, MemoryRecommendation, MemorySearchRequest, MemoryStats,
    MemoryStatus,
};
use crate::copilot::memory::repository::MemoryRepository;
use crate::copilot::memory::retrieval::{filter_records, rank_records};
use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::ExecutionPlan;
use crate::errors::DatabaseError;
use crate::semantic::embeddings::EmbeddingProvider;

/// The execution memory facade. Cheap to clone; all state lives behind the
/// connection pool and the shared embedding provider.
#[derive(Clone)]
pub struct MemoryEngine {
    repository: MemoryRepository,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl MemoryEngine {
    /// Creates a memory engine over a repository and embedding provider.
    pub fn new(
        repository: MemoryRepository,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            repository,
            embedding_provider,
        }
    }

    // ------------------------------------------------------------------
    // Capture
    // ------------------------------------------------------------------

    /// Records a terminal plan execution (success/failure/cancellation)
    /// into memory. Best-effort by contract of the callers: a capture
    /// failure never affects the execution lifecycle.
    #[allow(clippy::too_many_arguments)] // one call site; the capture payload is the point
    pub async fn record_execution(
        &self,
        execution_id: Uuid,
        workspace_id: Option<Uuid>,
        goal: &str,
        plan: Option<&ExecutionPlan>,
        steps: &[ExecutionStep],
        status: ExecutionStatus,
        error: Option<String>,
    ) -> Result<(), DatabaseError> {
        let memory_status = match status {
            ExecutionStatus::Completed => MemoryStatus::Success,
            ExecutionStatus::Failed => MemoryStatus::Failed,
            _ => MemoryStatus::Cancelled,
        };
        let embedding = self.embed_goal(goal).await;

        let outcome = MemoryOutcome {
            steps: steps.len(),
            completed: steps
                .iter()
                .filter(|s| matches!(s.status, crate::copilot::StepStatus::Completed))
                .count(),
            replaced: 0,
            replan_count: 0,
            retries_used: 0,
            plans_attempted: 1,
        };

        let record = ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: execution_id,
            workspace_id,
            goal: goal.to_string(),
            status: memory_status,
            plan: plan.cloned(),
            steps: steps.iter().map(|s| s.description.clone()).collect(),
            reasoning: vec![],
            tools_used: steps.iter().filter_map(|s| s.tool_name.clone()).collect(),
            failed_steps: steps
                .iter()
                .filter(|s| s.error.is_some())
                .map(|s| s.tool_name.clone().unwrap_or_else(|| s.description.clone()))
                .collect(),
            error,
            outcome,
            goal_embedding: embedding,
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.repository.upsert(&record).await
    }

    /// Records a planner report: the final run summary (completed/skipped/
    /// replaced tasks, replan accounting) as its own memory row so the
    /// learning engine can rank planner-driven runs.
    pub async fn record_planner_report(&self, report: &PlannerReport) -> Result<(), DatabaseError> {
        let goal = report.plan.goal.clone();
        let embedding = self.embed_goal(&goal).await;
        let status = if report.error.is_none() && !report.completed.is_empty() {
            MemoryStatus::Success
        } else {
            MemoryStatus::Failed
        };
        let record = ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::PlannerReport,
            source_id: report.execution_id.unwrap_or_else(Uuid::new_v4),
            workspace_id: report.plan.workspace_id,
            goal,
            status,
            plan: Some(report.plan.clone()),
            steps: report
                .plan
                .tasks
                .iter()
                .map(|t| t.description.clone())
                .collect(),
            reasoning: vec![format!(
                "Planner report: {} completed, {} replaced, {} replans",
                report.completed.len(),
                report.replaced.len(),
                report.replan_count
            )],
            tools_used: report
                .plan
                .tasks
                .iter()
                .filter_map(|t| t.tool_name.clone())
                .collect(),
            failed_steps: report
                .plan
                .tasks
                .iter()
                .filter(|t| report.replaced.contains(&t.id))
                .filter_map(|t| t.tool_name.clone())
                .collect(),
            error: report.error.clone(),
            outcome: outcome_from_report(report),
            goal_embedding: embedding,
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.repository.upsert(&record).await
    }

    /// Records a terminal autonomous session into memory.
    pub async fn record_autonomous_session(
        &self,
        progress: &AutonomousSessionProgress,
    ) -> Result<(), DatabaseError> {
        let status = match progress.status {
            AutonomousStatus::Completed => MemoryStatus::Success,
            AutonomousStatus::Failed => MemoryStatus::Failed,
            _ => MemoryStatus::Cancelled,
        };
        let embedding = self.embed_goal(&progress.goal).await;

        let (plan, steps, tools, failed_steps): (
            Option<ExecutionPlan>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        ) = match &progress.current_plan {
            Some(plan) => (
                Some(plan.clone()),
                plan.tasks.iter().map(|t| t.description.clone()).collect(),
                plan.tasks
                    .iter()
                    .filter_map(|t| t.tool_name.clone())
                    .collect(),
                plan.tasks
                    .iter()
                    .filter(|t| !t.completed)
                    .filter_map(|t| t.tool_name.clone())
                    .collect(),
            ),
            None => (None, vec![], vec![], vec![]),
        };

        let record = ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::AutonomousSession,
            source_id: progress.session_id,
            workspace_id: progress.workspace_id,
            goal: progress.goal.clone(),
            status,
            plan,
            steps,
            reasoning: progress
                .reasoning
                .iter()
                .map(|event| format!("{}: {}", event.phase, event.message))
                .collect(),
            tools_used: tools,
            failed_steps,
            error: progress.error.clone(),
            outcome: MemoryOutcome {
                steps: progress.steps_completed as usize,
                completed: progress.steps_completed as usize,
                replaced: progress.replans_used as usize,
                replan_count: progress.replans_used as usize,
                retries_used: progress.retries_used,
                plans_attempted: progress.plans_attempted,
            },
            goal_embedding: embedding,
            replay_count: 0,
            created_at: progress.updated_at,
            updated_at: progress.updated_at,
        };
        self.repository.upsert(&record).await
    }

    // ------------------------------------------------------------------
    // Semantic retrieval
    // ------------------------------------------------------------------

    /// Searches remembered runs by goal similarity, honoring the request's
    /// filters.
    pub async fn search(
        &self,
        request: &MemorySearchRequest,
    ) -> Result<Vec<MemoryHit>, DatabaseError> {
        let query_embedding = self.embed_goal(&request.query).await;
        let all = self.repository.list_all().await?;
        let filtered = filter_records(request, &all);
        let hits = rank_records(&request.query, query_embedding.as_deref(), &filtered);
        let mut hits = hits;
        hits.truncate(request.limit);
        Ok(hits)
    }

    /// Retrieves the most similar *successful* workflows for a goal,
    /// ranked by the learning blend (similarity + success history +
    /// recency).
    pub async fn recommend(
        &self,
        goal: &str,
        workspace_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<MemoryRecommendation>, DatabaseError> {
        let query_embedding = self.embed_goal(goal).await;
        let all = self.repository.list_all().await?;
        let scoped = filter_records(
            &MemorySearchRequest {
                query: goal.to_string(),
                kind: None,
                workspace_id,
                status: None,
                limit: usize::MAX,
            },
            &all,
        );
        let now_ms = Utc::now().timestamp_millis();
        let ranked =
            learning::rank_historical(goal, query_embedding.as_deref(), &scoped, false, now_ms);
        let recommendations: Vec<MemoryRecommendation> = ranked
            .into_iter()
            .take(limit)
            .map(|hit| MemoryRecommendation {
                score: learning::learned_score(&hit.record, hit.similarity, &scoped, now_ms),
                replay_count: hit.record.replay_count,
                record: hit.record,
            })
            .collect();
        Ok(recommendations)
    }

    /// Retrieves failed/cancelled strategies relevant to a goal — what the
    /// runtime should avoid repeating.
    pub async fn avoid(
        &self,
        goal: &str,
        workspace_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<AvoidedStrategy>, DatabaseError> {
        let query_embedding = self.embed_goal(goal).await;
        let all = self.repository.list_all().await?;
        let scoped = filter_records(
            &MemorySearchRequest {
                query: goal.to_string(),
                kind: None,
                workspace_id,
                status: None,
                limit: usize::MAX,
            },
            &all,
        );
        Ok(learning::avoid_strategies(
            goal,
            query_embedding.as_deref(),
            &scoped,
            limit,
        ))
    }

    /// Aggregated workflows learned from repeated executions.
    pub async fn learned_workflows(&self) -> Result<Vec<LearnedWorkflow>, DatabaseError> {
        let all = self.repository.list_all().await?;
        Ok(learning::learned_workflows(&all))
    }

    /// Dashboard statistics over the whole store.
    pub async fn stats(&self) -> Result<MemoryStats, DatabaseError> {
        let all = self.repository.list_all().await?;
        Ok(learning::compute_stats(&all))
    }

    /// Marks a record as replayed (used when the planner reuses it), so
    /// the learning engine can weight frequently-reused workflows.
    pub async fn mark_replayed(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.repository.mark_replayed(id).await
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// Whether the fingerprint of a goal has a known successful workflow.
    pub async fn has_successful_workflow(&self, goal: &str) -> Result<bool, DatabaseError> {
        let all = self.repository.list_all().await?;
        let fingerprint = goal_fingerprint(goal);
        Ok(all
            .iter()
            .any(|r| r.status == MemoryStatus::Success && goal_fingerprint(&r.goal) == fingerprint))
    }

    async fn embed_goal(&self, goal: &str) -> Option<Vec<f32>> {
        self.embedding_provider.embed(goal).await.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::semantic::embeddings::LocalEmbeddingProvider;

    async fn engine() -> (MemoryEngine, tempfile::TempDir) {
        let (database, guard) = test_database().await;
        let repo = MemoryRepository::new(database.pool().clone());
        let provider: Arc<dyn EmbeddingProvider> = Arc::new(LocalEmbeddingProvider::default());
        (MemoryEngine::new(repo, provider), guard)
    }

    fn step(tool: Option<&str>, description: &str, error: Option<&str>) -> ExecutionStep {
        ExecutionStep {
            id: Uuid::new_v4(),
            execution_id: Uuid::new_v4(),
            step_number: 0,
            description: description.to_string(),
            tool_name: tool.map(String::from),
            arguments: None,
            status: if error.is_some() {
                crate::copilot::StepStatus::Failed
            } else {
                crate::copilot::StepStatus::Completed
            },
            result: None,
            error: error.map(String::from),
            started_at: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn record_execution_and_search_find_it() {
        let (engine, _guard) = engine().await;
        let execution_id = Uuid::new_v4();
        engine
            .record_execution(
                execution_id,
                None,
                "resume my focus session",
                None,
                &[
                    step(Some("list_workspaces"), "List workspaces", None),
                    step(Some("resume_workspace"), "Resume focused work", None),
                ],
                ExecutionStatus::Completed,
                None,
            )
            .await
            .expect("capture should succeed");

        let hits = engine
            .search(&MemorySearchRequest::new("resume my focus session"))
            .await
            .expect("search should succeed");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].record.source_id, execution_id);
        assert!(matches!(hits[0].record.status, MemoryStatus::Success));
        assert!(hits[0].similarity > 0.9);
        assert_eq!(
            hits[0].record.tools_used,
            vec!["list_workspaces", "resume_workspace"]
        );
    }

    #[tokio::test]
    async fn failed_executions_are_retrievable_as_avoid_strategies() {
        let (engine, _guard) = engine().await;
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(
                    Some("get_recent_events"),
                    "Gather activity",
                    Some("permission denied"),
                )],
                ExecutionStatus::Failed,
                Some("permission denied".into()),
            )
            .await
            .expect("capture should succeed");

        let avoided = engine
            .avoid("resume my focus session", None, 5)
            .await
            .expect("avoid should succeed");
        assert_eq!(avoided.len(), 1);
        assert!(avoided[0].failure.contains("permission denied"));
    }

    #[tokio::test]
    async fn recommend_prefers_successful_workflows() {
        let (engine, _guard) = engine().await;
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(Some("list_workspaces"), "List workspaces", None)],
                ExecutionStatus::Completed,
                None,
            )
            .await
            .unwrap();
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(
                    Some("get_recent_events"),
                    "Gather activity",
                    Some("denied"),
                )],
                ExecutionStatus::Failed,
                Some("denied".into()),
            )
            .await
            .unwrap();

        let recommendations = engine
            .recommend("resume my focus session", None, 5)
            .await
            .expect("recommend should succeed");
        assert_eq!(recommendations.len(), 1);
        assert!(matches!(
            recommendations[0].record.status,
            MemoryStatus::Success
        ));
        assert!(recommendations[0].score > 0.5);
    }

    #[tokio::test]
    async fn planner_report_recording_round_trips() {
        let (engine, _guard) = engine().await;
        let report = PlannerReport {
            plan: ExecutionPlan {
                id: Uuid::new_v4(),
                workspace_id: None,
                goal: "recover after step failure".into(),
                tasks: vec![],
                estimated_duration_minutes: 0,
                required_files: vec![],
                checkpoints: vec![],
                confidence: 0.8,
                reasoning: "r".into(),
                status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
                created_at: Utc::now(),
            },
            execution_id: Some(Uuid::new_v4()),
            completed: vec![Uuid::new_v4()],
            skipped: vec![],
            replaced: vec![],
            replan_count: 0,
            error: None,
        };
        engine
            .record_planner_report(&report)
            .await
            .expect("report capture should succeed");

        let hits = engine
            .search(&MemorySearchRequest {
                query: "recover after step failure".into(),
                kind: Some(MemoryKind::PlannerReport),
                workspace_id: None,
                status: None,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].record.outcome.completed, 1);
    }

    #[tokio::test]
    async fn autonomous_session_recording_round_trips() {
        let (engine, _guard) = engine().await;
        let progress = AutonomousSessionProgress {
            session_id: Uuid::new_v4(),
            workspace_id: None,
            goal: "resume the most recent workspace".into(),
            status: AutonomousStatus::Completed,
            policy: Default::default(),
            reasoning: vec![crate::copilot::autonomous::models::ReasoningEvent::new(
                Uuid::new_v4(),
                crate::copilot::autonomous::models::ReasoningPhase::Terminal,
                "Goal reached: 3 steps completed",
                None,
            )],
            current_plan: None,
            execution_id: None,
            last_execution_id: None,
            plans_attempted: 1,
            plans_completed: 1,
            steps_completed: 3,
            retries_used: 0,
            replans_used: 0,
            steps_left: 0,
            error: None,
            pending_approval: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        engine
            .record_autonomous_session(&progress)
            .await
            .expect("session capture should succeed");

        let hits = engine
            .search(&MemorySearchRequest {
                query: "resume the most recent workspace".into(),
                kind: Some(MemoryKind::AutonomousSession),
                workspace_id: None,
                status: None,
                limit: 10,
            })
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].record.outcome.steps, 3);
        assert_eq!(hits[0].record.outcome.plans_attempted, 1);
    }

    #[tokio::test]
    async fn stats_and_learned_workflows_aggregate() {
        let (engine, _guard) = engine().await;
        for _ in 0..2 {
            engine
                .record_execution(
                    Uuid::new_v4(),
                    None,
                    "resume my focus session",
                    None,
                    &[step(Some("list_workspaces"), "List workspaces", None)],
                    ExecutionStatus::Completed,
                    None,
                )
                .await
                .unwrap();
        }
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(
                    Some("get_recent_events"),
                    "Gather activity",
                    Some("denied"),
                )],
                ExecutionStatus::Failed,
                Some("denied".into()),
            )
            .await
            .unwrap();

        let stats = engine.stats().await.unwrap();
        assert_eq!(stats.total_records, 3);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.failed, 1);

        let workflows = engine.learned_workflows().await.unwrap();
        assert_eq!(workflows.len(), 1);
        let focus = &workflows[0];
        assert_eq!(focus.success_count, 2);
        assert_eq!(focus.failure_count, 1);
        assert_eq!(focus.goal_fingerprint, "resume my focus session");
    }

    #[tokio::test]
    async fn mark_replayed_increments_replay_count() {
        let (engine, _guard) = engine().await;
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(Some("list_workspaces"), "List workspaces", None)],
                ExecutionStatus::Completed,
                None,
            )
            .await
            .unwrap();
        let hits = engine
            .search(&MemorySearchRequest::new("resume my focus session"))
            .await
            .unwrap();
        let id = hits[0].record.id;
        engine.mark_replayed(id).await.unwrap();
        engine.mark_replayed(id).await.unwrap();

        let stats = engine.stats().await.unwrap();
        assert_eq!(stats.total_replays, 2);
    }
}
