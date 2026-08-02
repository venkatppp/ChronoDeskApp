//! Execution Memory models - the durable records ChronoDesk learns from,
//! plus the query payloads/ranked hits returned by the retrieval and
//! learning engines.
//!
//! RC-6 M1. Pure data types; all retrieval/ranking lives in `retrieval`
//! and `learning`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::planner::PlannerReport;
use crate::copilot::proactive_models::ExecutionPlan;

/// What kind of execution produced a memory record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    /// A plan execution driven by the `ExecutionEngine`.
    Execution,
    /// A `PlannerReport` attached to an engine execution.
    PlannerReport,
    /// A whole autonomous agent session.
    AutonomousSession,
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryKind::Execution => write!(f, "execution"),
            MemoryKind::PlannerReport => write!(f, "planner_report"),
            MemoryKind::AutonomousSession => write!(f, "autonomous_session"),
        }
    }
}

/// Outcome of a remembered run. `Success` and `Failed` map to the engine
/// terminal states; `Cancelled` also covers operator rejections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Success,
    Failed,
    Cancelled,
}

impl std::fmt::Display for MemoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryStatus::Success => write!(f, "success"),
            MemoryStatus::Failed => write!(f, "failed"),
            MemoryStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Structured outcome accounting stored on every memory record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryOutcome {
    /// Total steps the run consumed.
    pub steps: usize,
    /// Steps that completed successfully.
    pub completed: usize,
    /// Steps replaced by replanning.
    pub replaced: usize,
    /// Replan passes performed during the run.
    pub replan_count: usize,
    /// Retries consumed by an autonomous session.
    pub retries_used: u64,
    /// Plans handed to the engine by an autonomous session.
    pub plans_attempted: u64,
}

/// One durable memory row. Everything needed to learn from a run and to
/// rebuild a reusable plan lives on the record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMemoryRecord {
    pub id: Uuid,
    pub kind: MemoryKind,
    /// The engine execution / autonomous session this memory came from.
    pub source_id: Uuid,
    pub workspace_id: Option<Uuid>,
    /// The goal the run was created for.
    pub goal: String,
    pub status: MemoryStatus,
    /// The plan that ran (rebuilt for planner-created runs).
    pub plan: Option<ExecutionPlan>,
    /// Human-readable step descriptions, in execution order.
    pub steps: Vec<String>,
    /// Reasoning/timeline notes (autonomous sessions).
    pub reasoning: Vec<String>,
    /// Tool names used by the run.
    pub tools_used: Vec<String>,
    /// Tool names / step descriptions that failed.
    pub failed_steps: Vec<String>,
    /// Failure error, when the run did not succeed.
    pub error: Option<String>,
    pub outcome: MemoryOutcome,
    /// Goal embedding (set by the engine when an embedding provider is
    /// available) so retrieval can rank by semantic similarity.
    pub goal_embedding: Option<Vec<f32>>,
    /// How many times this record was recommended/reused by the planner.
    pub replay_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExecutionMemoryRecord {
    /// The first failed tool name, if any (the strategy to avoid).
    pub fn first_failed_tool(&self) -> Option<&str> {
        self.failed_steps
            .iter()
            .find(|step| !step.is_empty())
            .map(String::as_str)
    }
}

/// Filters for a memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    /// Free-form goal/strategy text to match against remembered goals.
    pub query: String,
    /// Restrict to a memory kind (`None` = all kinds).
    pub kind: Option<MemoryKind>,
    /// Restrict to a workspace (`None` = all workspaces).
    pub workspace_id: Option<Uuid>,
    /// Only returns memories with this outcome.
    pub status: Option<MemoryStatus>,
    /// Maximum hits returned.
    pub limit: usize,
}

impl MemorySearchRequest {
    /// Builds a search request with the common defaults.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            kind: None,
            workspace_id: None,
            status: None,
            limit: 10,
        }
    }
}

/// A ranked memory hit: the record plus its similarity score (0..1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHit {
    pub record: ExecutionMemoryRecord,
    /// Similarity between the query and the record's goal (0..1).
    pub similarity: f64,
}

/// A learned recommendation: a previously successful workflow the planner
/// can reuse for the queried goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecommendation {
    pub record: ExecutionMemoryRecord,
    /// Blended score (similarity + outcome history + recency), 0..1.
    pub score: f64,
    /// How many times this workflow was replayed.
    pub replay_count: u64,
}

/// A strategy to avoid: a remembered run that failed (or was cancelled)
/// for a similar goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvoidedStrategy {
    pub record: ExecutionMemoryRecord,
    pub similarity: f64,
    /// What failed, human-readable.
    pub failure: String,
}

/// An aggregated workflow learned from repeated executions: a goal
/// fingerprint with its success/failure history and the best plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedWorkflow {
    /// Normalized goal fingerprint (lowercased, whitespace-collapsed).
    pub goal_fingerprint: String,
    /// A representative goal string (the most recent one).
    pub goal: String,
    pub success_count: u64,
    pub failure_count: u64,
    /// Best remembered plan for this fingerprint, when any succeeded.
    pub best_plan: Option<ExecutionPlan>,
    pub last_success_at: Option<DateTime<Utc>>,
}

/// Aggregate statistics about the memory store, for the dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_records: u64,
    pub successful: u64,
    pub failed: u64,
    pub cancelled: u64,
    pub executions: u64,
    pub planner_reports: u64,
    pub autonomous_sessions: u64,
    pub total_replays: u64,
    pub learned_workflows: u64,
}

/// Normalizes a goal into a stable fingerprint used to aggregate learning:
/// lowercased, trimmed, and whitespace-collapsed.
pub fn goal_fingerprint(goal: &str) -> String {
    goal.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Summarizes a planner report into outcome accounting for a memory row.
pub fn outcome_from_report(report: &PlannerReport) -> MemoryOutcome {
    MemoryOutcome {
        steps: report.plan.tasks.len(),
        completed: report.completed.len(),
        replaced: report.replaced.len(),
        replan_count: report.replan_count,
        retries_used: 0,
        plans_attempted: 1,
    }
}

/// Encodes an embedding vector into the little-endian BLOB format stored
/// in SQLite (shared by every repository that persists embeddings, so the
/// wire format lives in exactly one place).
pub fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Decodes an embedding BLOB written by [`embedding_to_blob`]. Returns an
/// empty vector for an empty or misaligned BLOB.
pub fn embedding_from_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Stable hash of a text, used as the key for the embedding cache and the
/// vector index. `DefaultHasher::new()` uses fixed keys, so the value is
/// deterministic across processes (unlike `HashMap`'s random state).
pub fn text_hash(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_fingerprint_is_normalized() {
        assert_eq!(
            goal_fingerprint("  Resume   My Focus  Session "),
            "resume my focus session"
        );
        assert_eq!(
            goal_fingerprint("Resume my focus session"),
            goal_fingerprint("  RESUME my focus session  ")
        );
    }

    #[test]
    fn outcome_from_report_counts_steps() {
        let report = PlannerReport {
            plan: crate::copilot::proactive_models::ExecutionPlan {
                id: Uuid::new_v4(),
                workspace_id: None,
                goal: "g".into(),
                tasks: vec![
                    crate::copilot::proactive_models::PlanTask {
                        id: Uuid::new_v4(),
                        description: "a".into(),
                        dependencies: vec![],
                        estimated_minutes: 1,
                        required_files: vec![],
                        tool_name: None,
                        arguments: None,
                        completed: false,
                        condition: None,
                    },
                    crate::copilot::proactive_models::PlanTask {
                        id: Uuid::new_v4(),
                        description: "b".into(),
                        dependencies: vec![],
                        estimated_minutes: 1,
                        required_files: vec![],
                        tool_name: None,
                        arguments: None,
                        completed: false,
                        condition: None,
                    },
                ],
                estimated_duration_minutes: 2,
                required_files: vec![],
                checkpoints: vec![],
                confidence: 0.8,
                reasoning: "".into(),
                status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
                created_at: Utc::now(),
            },
            execution_id: None,
            completed: vec![Uuid::new_v4()],
            skipped: vec![],
            replaced: vec![Uuid::new_v4()],
            replan_count: 1,
            error: None,
        };
        let outcome = outcome_from_report(&report);
        assert_eq!(outcome.steps, 2);
        assert_eq!(outcome.completed, 1);
        assert_eq!(outcome.replaced, 1);
        assert_eq!(outcome.replan_count, 1);
    }

    #[test]
    fn memory_kind_round_trips() {
        let json = serde_json::to_value(MemoryKind::AutonomousSession).unwrap();
        assert_eq!(json, "autonomous_session");
        let back: MemoryKind = serde_json::from_value(json).unwrap();
        assert!(matches!(back, MemoryKind::AutonomousSession));
    }

    #[test]
    fn embedding_blob_round_trips() {
        let embedding = vec![0.1, -0.5, 0.0, 1.0];
        let blob = embedding_to_blob(&embedding);
        assert_eq!(blob.len(), embedding.len() * 4);
        let decoded = embedding_from_blob(&blob);
        assert_eq!(decoded.len(), embedding.len());
        for (a, b) in decoded.iter().zip(embedding.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        assert!(embedding_from_blob(&[1, 2, 3]).is_empty(), "misaligned");
    }

    #[test]
    fn text_hash_is_stable_and_distinct() {
        assert_eq!(
            text_hash("resume my focus session"),
            text_hash("resume my focus session")
        );
        assert_ne!(
            text_hash("resume my focus session"),
            text_hash("organize receipts")
        );
        // `DefaultHasher::new()` uses fixed keys: the same text hashes to
        // the same value on every process run (SQL cache key stability).
        assert_eq!(text_hash("persist me"), text_hash("persist me"));
    }
}
