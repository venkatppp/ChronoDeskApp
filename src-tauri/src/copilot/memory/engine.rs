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
use crate::copilot::memory::vector::{
    IndexResult, MemoryVectorSystem, VectorIndexStatus, VectorProvider,
};
use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::ExecutionPlan;
use crate::errors::DatabaseError;

/// The execution memory facade. Cheap to clone; all state lives behind the
/// connection pool, the shared vector provider, and the in-memory k-NN
/// index.
#[derive(Clone)]
pub struct MemoryEngine {
    repository: MemoryRepository,
    vectors: MemoryVectorSystem,
}

impl MemoryEngine {
    /// Creates a memory engine over a repository and a vector provider
    /// (RC-6 M2: the provider feeds the cache + k-NN index + background
    /// indexer).
    pub fn new(repository: MemoryRepository, provider: Arc<dyn VectorProvider>) -> Self {
        let vectors = MemoryVectorSystem::new(repository.pool().clone(), provider);
        Self {
            repository,
            vectors,
        }
    }

    /// The underlying vector system (indexer, k-NN index, cache).
    pub fn vector_system(&self) -> &MemoryVectorSystem {
        &self.vectors
    }

    /// Runs one background indexing pass over pending records (new or
    /// changed goals). The indexer worker calls this on capture
    /// notifications; exposed for the dashboard "index now" action.
    pub async fn index_pending(&self, limit: usize) -> Result<IndexResult, DatabaseError> {
        self.vectors.indexer().index_pending(limit).await
    }

    /// Drops and rebuilds the whole vector index.
    pub async fn reindex(&self) -> Result<IndexResult, DatabaseError> {
        self.vectors.indexer().reindex_all().await
    }

    /// Dashboard status of the vector index and embedding cache.
    pub async fn vector_status(&self) -> Result<VectorIndexStatus, DatabaseError> {
        self.vectors.status().await
    }

    // ------------------------------------------------------------------
    // Capture
    // ------------------------------------------------------------------

    /// Records a terminal plan execution (success/failure/cancellation)
    /// into memory. Best-effort by contract of the callers: a capture
    /// failure never affects the execution lifecycle. The record is
    /// stored immediately and the background indexer is notified to
    /// embed the goal (RC-6 M2: incremental, batched indexing).
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
            goal_embedding: None, // filled by the background indexer
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.repository.upsert(&record).await?;
        self.vectors.indexer().notify();
        Ok(())
    }

    /// Records a planner report: the final run summary (completed/skipped/
    /// replaced tasks, replan accounting) as its own memory row so the
    /// learning engine can rank planner-driven runs.
    pub async fn record_planner_report(&self, report: &PlannerReport) -> Result<(), DatabaseError> {
        let goal = report.plan.goal.clone();
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
            goal_embedding: None, // filled by the background indexer
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.repository.upsert(&record).await?;
        self.vectors.indexer().notify();
        Ok(())
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
            goal_embedding: None, // filled by the background indexer
            replay_count: 0,
            created_at: progress.updated_at,
            updated_at: progress.updated_at,
        };
        self.repository.upsert(&record).await?;
        self.vectors.indexer().notify();
        Ok(())
    }

    // ------------------------------------------------------------------
    // Semantic retrieval
    // ------------------------------------------------------------------

    /// Searches remembered runs by goal similarity, honoring the request's
    /// filters. When the vector index holds embeddings, the query is
    /// embedded once and k-NN selects the candidate ids before any SQL
    /// row decode happens; without an index it falls back to the full
    /// token-overlap scan (e.g. before the first index pass).
    pub async fn search(
        &self,
        request: &MemorySearchRequest,
    ) -> Result<Vec<MemoryHit>, DatabaseError> {
        let query_embedding = self.vectors.embed(&request.query).await;
        let all = match self.knn_candidates(
            query_embedding.as_deref(),
            request.limit,
            5,
            request.query.trim().is_empty(),
        ) {
            Some(ids) if !ids.is_empty() => self.repository.get_many(&ids).await?,
            _ => self.repository.list_all().await?,
        };
        let filtered = filter_records(request, &all);
        let mut hits = rank_records(&request.query, query_embedding.as_deref(), &filtered);
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
        let query_embedding = self.vectors.embed(goal).await;
        let all = match self.knn_candidates(
            query_embedding.as_deref(),
            limit,
            20,
            goal.trim().is_empty(),
        ) {
            Some(ids) if !ids.is_empty() => self.repository.get_many(&ids).await?,
            _ => self.repository.list_all().await?,
        };
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
        let query_embedding = self.vectors.embed(goal).await;
        let all = match self.knn_candidates(
            query_embedding.as_deref(),
            limit,
            20,
            goal.trim().is_empty(),
        ) {
            Some(ids) if !ids.is_empty() => self.repository.get_many(&ids).await?,
            _ => self.repository.list_all().await?,
        };
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

    /// Selects the candidate memory ids for a query: k-NN over the
    /// in-memory vector index, oversampled so downstream filters
    /// (workspace/status) do not starve the result list. Returns `None`
    /// when the index is empty or the query cannot be embedded (callers
    /// then fall back to the full scan).
    fn knn_candidates(
        &self,
        query_embedding: Option<&[f32]>,
        limit: usize,
        oversample: usize,
        skip: bool,
    ) -> Option<Vec<Uuid>> {
        if skip {
            return None;
        }
        let query = query_embedding?;
        let indexed = self.vectors.index_len();
        if indexed == 0 {
            return None;
        }
        let k = limit
            .saturating_mul(oversample)
            .clamp(50, 1000)
            .min(indexed);
        Some(
            self.vectors
                .knn(query, k)
                .into_iter()
                .map(|(id, _)| id)
                .collect(),
        )
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
