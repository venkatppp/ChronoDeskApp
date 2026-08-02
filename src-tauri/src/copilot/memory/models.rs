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
use crate::learning::models::ExplanationReason;

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

/// Retention policy of a memory record (RC-6 M4): how long the record
/// is kept and whether the cleanup worker may remove it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicy {
    /// Kept indefinitely (default).
    #[default]
    Permanent,
    /// Kept until `retention_until`; the cleanup worker then marks it
    /// `Expired` and eventually deletes it.
    Temporary,
    /// Kept but out of active circulation (archived by the user or the
    /// aging rules); cleaned up when it duplicates a live memory.
    Archived,
    /// Past its retention; removed by the next cleanup pass.
    Expired,
}

impl std::fmt::Display for RetentionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetentionPolicy::Permanent => write!(f, "permanent"),
            RetentionPolicy::Temporary => write!(f, "temporary"),
            RetentionPolicy::Archived => write!(f, "archived"),
            RetentionPolicy::Expired => write!(f, "expired"),
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
    /// Wall-clock completion time of the run, seconds (0 = unknown;
    /// RC-6 M3, used by the duration factor of the learned blend).
    pub duration_seconds: u64,
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

    // --- RC-6 M4: lifecycle ---
    /// Retention policy of the record.
    pub retention: RetentionPolicy,
    /// When a `Temporary` record expires (`None` otherwise).
    pub retention_until: Option<DateTime<Utc>>,
    /// When the record was archived, if ever.
    pub archived_at: Option<DateTime<Utc>>,
    /// When the record was marked expired, if ever.
    pub expired_at: Option<DateTime<Utc>>,
    /// Summary generated by compression (`Some` when compressed).
    pub summary: Option<String>,
    /// When the record was compressed, if ever.
    pub compressed_at: Option<DateTime<Utc>>,
    /// Version of this workflow: 1 for the first run, incremented for
    /// every run derived from a reused workflow (RC-6 M4).
    pub version: u64,
    /// The workflow version this record was derived from, if any.
    pub parent_id: Option<Uuid>,
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
    /// Blended score (similarity + outcome history + recency + replay +
    /// acceptance + duration), 0..1, archival-scaled.
    pub score: f64,
    /// How many times this workflow was replayed.
    pub replay_count: u64,
    /// Confidence Engine score (RC-6 M3): similarity + success history +
    /// replay history + freshness + usage count, archival-scaled.
    pub confidence_score: f64,
    /// Why the confidence is what it is, per factor (RC-6 M3).
    pub explanation: Vec<ExplanationReason>,
}

/// User acceptance ledger for a remembered run (RC-6 M3): how often the
/// user accepted/rejected the recommendation to reuse this memory.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MemoryAcceptance {
    pub accepted: u64,
    pub rejected: u64,
}

impl MemoryAcceptance {
    /// Acceptance rate (0..1); 0.5 when there is no feedback yet.
    pub fn rate(&self) -> f64 {
        let total = self.accepted + self.rejected;
        if total == 0 {
            0.5
        } else {
            self.accepted as f64 / total as f64
        }
    }
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
        duration_seconds: 0, // planner reports carry no wall-clock duration
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

// ---------------------------------------------------------------------
// RC-6 M4: memory lifecycle types
// ---------------------------------------------------------------------

/// Outcome of one lifecycle pass (manual "clean up now" or the
/// background cleanup worker).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanupReport {
    /// Temporary memories past their retention, marked `Expired`.
    pub expired_marked: u64,
    /// Expired memories deleted (with their vectors and ledger rows).
    pub removed_expired: u64,
    /// Archived memories that duplicated a live one, deleted.
    pub removed_duplicate_archives: u64,
    /// Orphaned vector index rows (no memory row), removed.
    pub removed_orphaned_vectors: u64,
    /// Records compressed in this pass.
    pub compressed: u64,
    /// When the pass ran.
    pub ran_at: String,
}

/// Outcome of a compression pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Records examined for compression.
    pub examined: u64,
    /// Records compressed.
    pub compressed: u64,
    /// Records already compressed, skipped.
    pub already_compressed: u64,
}

/// Storage statistics for the dashboard (RC-6 M4): how much space the
/// memory system occupies and how much of it is archived/expired.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStorageStats {
    /// Total size of the SQLite database file, bytes.
    pub database_size_bytes: u64,
    /// Size of the stored vectors (embeddings + text), bytes.
    pub vector_index_size_bytes: u64,
    /// Persistent embedding cache entries and their byte size.
    pub cache_entries: u64,
    pub cache_size_bytes: u64,
    /// In-memory embedding cache occupancy (dashboard only).
    pub cache_capacity: usize,
    pub cache_occupancy: usize,
    /// Records by retention policy.
    pub archived_memories: u64,
    pub expired_memories: u64,
    pub temporary_memories: u64,
    pub permanent_memories: u64,
    /// Snapshots stored and their byte size.
    pub snapshots: u64,
    pub snapshot_size_bytes: u64,
    /// Compressed records and their preserved originals.
    pub compressed_records: u64,
    pub compression_archive_count: u64,
}

/// How one lineage edge relates its two memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageRelation {
    /// `memory_id` is a new run derived from a reused workflow
    /// (`parent_id`).
    Parent,
    /// `memory_id` was merged into `parent_id` (duplicate merge).
    Merged,
}

/// One node in a lineage graph: a memory with its version and how it
/// relates to the queried memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: Uuid,
    pub goal: String,
    pub status: MemoryStatus,
    pub retention: RetentionPolicy,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    /// `None` for the queried memory itself.
    pub relation: Option<LineageRelation>,
}

/// The full lineage of one memory: ancestry (versions), descendants,
/// and merge history — the workflow's evolution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryLineage {
    pub memory_id: Uuid,
    /// Root of the version chain (the original run).
    pub root_id: Option<Uuid>,
    pub version: u64,
    /// Ancestors, oldest first (the version chain).
    pub ancestors: Vec<LineageNode>,
    /// Descendants, newest first.
    pub children: Vec<LineageNode>,
    /// Records merged *into* this memory (relation `merged`).
    pub merged_into: Vec<LineageNode>,
    /// This memory merged into another record (when it was a duplicate
    /// that got removed).
    pub merged_into_id: Option<Uuid>,
}

/// One stored snapshot (metadata; the payload is the export JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    /// Records captured in the snapshot payload.
    pub record_count: u64,
}

/// Outcome of restoring a snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Records restored into the store.
    pub records_restored: u64,
    /// Acceptance ledger entries restored.
    pub acceptance_restored: u64,
    /// Snapshots kept after the restore (pruned by age).
    pub snapshots_kept: u64,
}

/// An acceptance ledger entry carried by exports/imports (RC-6 M4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAcceptanceEntry {
    pub memory_id: Uuid,
    pub acceptance: MemoryAcceptance,
}

/// The portable, versioned export format. Also the snapshot payload, so
/// exports, imports, snapshots, and restores stay fully compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExport {
    /// Format version (1 today); imports reject newer versions.
    pub schema_version: u32,
    /// When the export was produced.
    pub exported_at: DateTime<Utc>,
    pub records: Vec<ExecutionMemoryRecord>,
    pub acceptance: Vec<MemoryAcceptanceEntry>,
}

/// Current export format version.
pub const MEMORY_EXPORT_SCHEMA_VERSION: u32 = 1;

/// Outcome of importing an export payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportResult {
    /// Records inserted (ids not already present).
    pub imported: u64,
    /// Records skipped because an id already existed.
    pub skipped: u64,
    /// Acceptance entries restored.
    pub acceptance_restored: u64,
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
