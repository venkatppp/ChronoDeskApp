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
use crate::copilot::memory::lifecycle_repository::LifecycleRepository;
use crate::copilot::memory::models::{
    goal_fingerprint, outcome_from_report, AvoidedStrategy, ExecutionMemoryRecord, LearnedWorkflow,
    MemoryAcceptance, MemoryHit, MemoryKind, MemoryOutcome, MemoryRecommendation,
    MemorySearchRequest, MemoryStats, MemoryStatus, RetentionPolicy,
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
    pub(crate) repository: MemoryRepository,
    pub(crate) vectors: MemoryVectorSystem,
    pub(crate) lifecycle: LifecycleRepository,
}

impl MemoryEngine {
    /// Creates a memory engine over a repository and a vector provider
    /// (RC-6 M2: the provider feeds the cache + k-NN index + background
    /// indexer; RC-6 M4 adds the lifecycle repository over the same
    /// pool).
    pub fn new(repository: MemoryRepository, provider: Arc<dyn VectorProvider>) -> Self {
        let vectors = MemoryVectorSystem::new(repository.pool().clone(), provider);
        let lifecycle = LifecycleRepository::new(repository.pool().clone());
        Self {
            repository,
            vectors,
            lifecycle,
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
    /// `duration_seconds` is the wall-clock run time when known (RC-6 M3:
    /// feeds the duration factor of the learned blend).
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
        duration_seconds: Option<u64>,
    ) -> Result<(), DatabaseError> {
        let memory_status = match status {
            ExecutionStatus::Completed => MemoryStatus::Success,
            ExecutionStatus::Failed => MemoryStatus::Failed,
            _ => MemoryStatus::Cancelled,
        };

        // RC-6 M4 versioning: a new successful run of a goal whose
        // workflow was already learned becomes the *next version* of that
        // workflow, chained to its most-replayed ancestor so lineage can
        // show the evolution.
        let (version, parent_id) = if memory_status == MemoryStatus::Success {
            match self
                .lifecycle
                .best_reusable_ancestor(&goal_fingerprint(goal))
                .await?
            {
                Some((parent, parent_version)) => (parent_version + 1, Some(parent)),
                None => (1, None),
            }
        } else {
            (1, None)
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
            duration_seconds: duration_seconds.unwrap_or(0),
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
            retention: RetentionPolicy::Permanent,
            retention_until: None,
            archived_at: None,
            expired_at: None,
            summary: None,
            compressed_at: None,
            version,
            parent_id,
        };
        self.repository.upsert(&record).await?;
        if let Some(parent) = parent_id {
            self.lifecycle
                .insert_lineage(record.id, parent, "parent")
                .await?;
        }
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
            retention: RetentionPolicy::Permanent,
            retention_until: None,
            archived_at: None,
            expired_at: None,
            summary: None,
            compressed_at: None,
            version: 1,
            parent_id: None,
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

        let duration = (progress.updated_at - progress.created_at)
            .num_seconds()
            .max(0) as u64;
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
                duration_seconds: duration,
            },
            goal_embedding: None, // filled by the background indexer
            replay_count: 0,
            created_at: progress.updated_at,
            updated_at: progress.updated_at,
            retention: RetentionPolicy::Permanent,
            retention_until: None,
            archived_at: None,
            expired_at: None,
            summary: None,
            compressed_at: None,
            version: 1,
            parent_id: None,
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
        let filtered = retain_live(filtered);
        let mut hits = rank_records(&request.query, query_embedding.as_deref(), &filtered);
        hits.truncate(request.limit);
        Ok(hits)
    }

    /// Retrieves the most similar *successful* workflows for a goal,
    /// ranked by the learning blend (adaptive weights over similarity,
    /// success history, recency, replay, user acceptance, duration, and
    /// failures, archival-scaled) — RC-6 M3. Every recommendation also
    /// carries a `confidence_score` from the Confidence Engine with
    /// per-factor explanations.
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
        let scoped = retain_live(scoped);
        let acceptance = self.repository.acceptance_map().await?;
        let now_ms = Utc::now().timestamp_millis();
        let weights = learning::learn_weights(&scoped, &acceptance, now_ms);
        let ranked = learning::rank_historical(
            goal,
            query_embedding.as_deref(),
            &scoped,
            false,
            &acceptance,
            &weights,
            now_ms,
        );
        let recommendations: Vec<MemoryRecommendation> = ranked
            .into_iter()
            .take(limit)
            .map(|hit| {
                let confidence = learning::confidence_score(
                    &hit.record,
                    hit.similarity,
                    &scoped,
                    &acceptance,
                    now_ms,
                );
                MemoryRecommendation {
                    score: learning::learned_score(
                        &hit.record,
                        hit.similarity,
                        &scoped,
                        &acceptance,
                        &weights,
                        now_ms,
                    ),
                    replay_count: hit.record.replay_count,
                    confidence_score: confidence.score,
                    explanation: confidence.factors,
                    record: hit.record,
                }
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
            &retain_live(scoped),
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
    // Adaptive learning (RC-6 M3)
    // ------------------------------------------------------------------

    /// Records user acceptance/rejection of a recommendation into the
    /// acceptance ledger, so the recommendation weights and confidence
    /// adapt to what the user actually accepts.
    pub async fn record_acceptance(
        &self,
        memory_id: Uuid,
        accepted: bool,
    ) -> Result<(), DatabaseError> {
        self.repository.record_acceptance(memory_id, accepted).await
    }

    /// The acceptance ledger keyed by memory id (learning + dashboard).
    pub async fn acceptance(
        &self,
    ) -> Result<std::collections::HashMap<Uuid, MemoryAcceptance>, DatabaseError> {
        self.repository.acceptance_map().await
    }

    /// Learning health payload for the dashboard: confidence averages,
    /// workflow quality, success trends, and memory utilization.
    pub async fn learning_health(&self) -> Result<learning::LearningHealth, DatabaseError> {
        let all = self.repository.list_all().await?;
        let acceptance = self.repository.acceptance_map().await?;
        let now_ms = Utc::now().timestamp_millis();
        Ok(learning::learning_health(&all, &acceptance, now_ms))
    }

    /// Detected failure patterns over the whole store (repeated failures,
    /// unstable workflows, low-confidence plans).
    pub async fn failure_patterns(&self) -> Result<Vec<learning::FailurePattern>, DatabaseError> {
        let all = self.repository.list_all().await?;
        let now_ms = Utc::now().timestamp_millis();
        Ok(learning::failure_patterns(&all, now_ms, 50))
    }

    /// Failure patterns relevant to one goal (advisory signal for the
    /// autonomous runtime before it trusts a remembered plan).
    pub async fn failure_patterns_for_goal(
        &self,
        goal: &str,
    ) -> Result<Vec<learning::FailurePattern>, DatabaseError> {
        let all = self.repository.list_all().await?;
        let now_ms = Utc::now().timestamp_millis();
        Ok(learning::failure_patterns_for_goal(goal, &all, now_ms))
    }

    /// Clusters remembered goals into reusable workflow families.
    pub async fn workflow_families(&self) -> Result<Vec<learning::WorkflowFamily>, DatabaseError> {
        let all = self.repository.list_all().await?;
        Ok(learning::workflow_families(&all))
    }

    /// Identical memories detected in the store (duplicate groups).
    pub async fn duplicate_groups(&self) -> Result<Vec<learning::DuplicateGroup>, DatabaseError> {
        let all = self.repository.list_all().await?;
        Ok(learning::duplicate_groups(&all))
    }

    /// Merges identical memories: keeps the best record of each group and
    /// deletes the rest (including their vector index entries).
    pub async fn merge_duplicates(&self) -> Result<learning::MergeResult, DatabaseError> {
        let all = self.repository.list_all().await?;
        let groups = learning::duplicate_groups(&all);
        let plan = learning::merge_plan(&groups);

        let mut result = learning::MergeResult {
            groups_merged: groups.len(),
            records_merged: 0,
        };
        for (keeper_id, removals) in plan {
            for id in removals {
                // RC-6 M4: record the merge in the lineage *before* the
                // deletion so the merged memory's history survives it
                // (the edge references the removed record's id).
                self.lifecycle
                    .insert_lineage(id, keeper_id, "merged")
                    .await?;
                self.vectors.remove(id).await?;
                self.repository.delete(id).await?;
                result.records_merged += 1;
            }
        }
        Ok(result)
    }

    /// Aging summary of the store (fresh / aging / archived buckets).
    pub async fn aging_summary(&self) -> Result<learning::MemoryAgingSummary, DatabaseError> {
        let all = self.repository.list_all().await?;
        let now_ms = Utc::now().timestamp_millis();
        Ok(learning::aging_summary(&all, now_ms))
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

/// Keeps only live (non-expired) records for retrieval: expired memories
/// are deleted by the next cleanup pass, so they must not surface in
/// searches, recommendations, or avoid lists.
fn retain_live(records: Vec<ExecutionMemoryRecord>) -> Vec<ExecutionMemoryRecord> {
    records
        .into_iter()
        .filter(|r| r.retention != RetentionPolicy::Expired)
        .collect()
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
