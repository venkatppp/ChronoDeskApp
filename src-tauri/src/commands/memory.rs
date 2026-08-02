//! Execution Memory IPC Commands - search, recommend, and inspect what
//! ChronoDesk has learned from previous executions (RC-6 M1), plus the
//! vector index status and manual re-index (RC-6 M2).
//!
//! Thin wrappers around the shared `MemoryEngine`; no business logic lives
//! here.

use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::copilot::memory::{
    AvoidedStrategy, CleanupReport, CompressionResult, DuplicateGroup, FailurePattern,
    ImportResult, IndexResult, LearnedWorkflow, LearningHealth, MemoryAgingSummary, MemoryEngine,
    MemoryHit, MemoryKind, MemoryLineage, MemoryRecommendation, MemorySearchRequest,
    MemorySnapshot, MemoryStats, MemoryStatus, MemoryStorageStats, MergeResult, RestoreResult,
    RetentionPolicy, VectorIndexStatus, WorkflowFamily,
};

/// Searches remembered runs by goal similarity, with optional filters.
#[tauri::command]
pub async fn memory_search(
    engine: State<'_, Arc<MemoryEngine>>,
    query: String,
    kind: Option<MemoryKind>,
    workspace_id: Option<String>,
    status: Option<MemoryStatus>,
    limit: Option<usize>,
) -> Result<Vec<MemoryHit>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let request = MemorySearchRequest {
        query,
        kind,
        workspace_id,
        status,
        limit: limit.unwrap_or(10),
    };
    engine.search(&request).await.map_err(|e| e.to_string())
}

/// Recommends previously successful workflows for a goal, ranked by the
/// learning blend.
#[tauri::command]
pub async fn memory_recommend(
    engine: State<'_, Arc<MemoryEngine>>,
    goal: String,
    workspace_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<MemoryRecommendation>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    engine
        .recommend(&goal, workspace_id, limit.unwrap_or(5))
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves failed/cancelled strategies relevant to a goal — what the
/// runtime should avoid repeating.
#[tauri::command]
pub async fn memory_avoid(
    engine: State<'_, Arc<MemoryEngine>>,
    goal: String,
    workspace_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AvoidedStrategy>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    engine
        .avoid(&goal, workspace_id, limit.unwrap_or(5))
        .await
        .map_err(|e| e.to_string())
}

/// Aggregated workflows learned from repeated executions.
#[tauri::command]
pub async fn memory_learned_workflows(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<LearnedWorkflow>, String> {
    engine.learned_workflows().await.map_err(|e| e.to_string())
}

/// Dashboard statistics over the memory store.
#[tauri::command]
pub async fn memory_stats(engine: State<'_, Arc<MemoryEngine>>) -> Result<MemoryStats, String> {
    engine.stats().await.map_err(|e| e.to_string())
}

/// Status of the vector index and embedding cache (RC-6 M2).
#[tauri::command]
pub async fn memory_index_status(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<VectorIndexStatus, String> {
    engine.vector_status().await.map_err(|e| e.to_string())
}

/// Runs an index pass now (dashboard "index now" action); re-indexes
/// everything when the store has drifted (RC-6 M2).
#[tauri::command]
pub async fn memory_reindex(engine: State<'_, Arc<MemoryEngine>>) -> Result<IndexResult, String> {
    engine.reindex().await.map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------
// RC-6 M3: adaptive learning commands (thin wrappers only)
// ----------------------------------------------------------------------

/// Records user acceptance/rejection of a recommendation, feeding the
/// acceptance ledger the adaptive weights and confidence learn from.
#[tauri::command]
pub async fn memory_recommendation_feedback(
    engine: State<'_, Arc<MemoryEngine>>,
    memory_id: String,
    accepted: bool,
) -> Result<(), String> {
    let memory_id = Uuid::parse_str(&memory_id).map_err(|e| e.to_string())?;
    engine
        .record_acceptance(memory_id, accepted)
        .await
        .map_err(|e| e.to_string())
}

/// Learning health: confidence averages, workflow quality, success
/// trends, and memory utilization.
#[tauri::command]
pub async fn memory_learning_health(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<LearningHealth, String> {
    engine.learning_health().await.map_err(|e| e.to_string())
}

/// Detected failure patterns (repeated failures, unstable workflows,
/// low-confidence plans).
#[tauri::command]
pub async fn memory_failure_patterns(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<FailurePattern>, String> {
    engine.failure_patterns().await.map_err(|e| e.to_string())
}

/// Workflow families learned by clustering remembered goals.
#[tauri::command]
pub async fn memory_workflow_families(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<WorkflowFamily>, String> {
    engine.workflow_families().await.map_err(|e| e.to_string())
}

/// Memory aging summary (fresh / aging / archived buckets).
#[tauri::command]
pub async fn memory_aging_summary(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<MemoryAgingSummary, String> {
    engine.aging_summary().await.map_err(|e| e.to_string())
}

/// Identical memories detected in the store.
#[tauri::command]
pub async fn memory_duplicate_groups(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<DuplicateGroup>, String> {
    engine.duplicate_groups().await.map_err(|e| e.to_string())
}

/// Merges identical memories, keeping the best record of each group.
#[tauri::command]
pub async fn memory_merge_duplicates(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<MergeResult, String> {
    engine.merge_duplicates().await.map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------
// RC-6 M4: memory lifecycle commands (thin wrappers only)
// ----------------------------------------------------------------------

/// Sets a record's retention policy (permanent / temporary + deadline /
/// archived / expired).
#[tauri::command]
pub async fn memory_set_retention(
    engine: State<'_, Arc<MemoryEngine>>,
    memory_id: String,
    policy: RetentionPolicy,
    retention_until: Option<String>,
) -> Result<(), String> {
    let memory_id = Uuid::parse_str(&memory_id).map_err(|e| e.to_string())?;
    let retention_until = retention_until
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
        .transpose()
        .map_err(|e| e.to_string())?
        .map(|dt| dt.with_timezone(&chrono::Utc));
    engine
        .set_retention(memory_id, policy, retention_until)
        .await
        .map_err(|e| e.to_string())
}

/// Runs one cleanup pass now (expire, delete, dedupe archives, remove
/// orphaned vectors, compress).
#[tauri::command]
pub async fn memory_cleanup_now(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<CleanupReport, String> {
    engine.run_cleanup().await.map_err(|e| e.to_string())
}

/// Compresses oversized reasoning histories (budgeted pass).
#[tauri::command]
pub async fn memory_compress_oversized(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<CompressionResult, String> {
    engine
        .compress_oversized(32)
        .await
        .map_err(|e| e.to_string())
}

/// Restores a compressed record from its preservation archive.
#[tauri::command]
pub async fn memory_restore_compressed(
    engine: State<'_, Arc<MemoryEngine>>,
    memory_id: String,
) -> Result<bool, String> {
    let memory_id = Uuid::parse_str(&memory_id).map_err(|e| e.to_string())?;
    engine
        .restore_compressed(memory_id)
        .await
        .map_err(|e| e.to_string())
}

/// The full lineage of a memory: version ancestry, descendants, merges.
#[tauri::command]
pub async fn memory_lineage(
    engine: State<'_, Arc<MemoryEngine>>,
    memory_id: String,
) -> Result<Option<MemoryLineage>, String> {
    let memory_id = Uuid::parse_str(&memory_id).map_err(|e| e.to_string())?;
    engine.lineage(memory_id).await.map_err(|e| e.to_string())
}

/// Exports the whole memory store as JSON (snapshot-compatible format).
#[tauri::command]
pub async fn memory_export_json(engine: State<'_, Arc<MemoryEngine>>) -> Result<String, String> {
    engine.export_json().await.map_err(|e| e.to_string())
}

/// Imports an export payload (idempotent by record id).
#[tauri::command]
pub async fn memory_import_json(
    engine: State<'_, Arc<MemoryEngine>>,
    content: String,
) -> Result<ImportResult, String> {
    engine
        .import_json(&content)
        .await
        .map_err(|e| e.to_string())
}

/// Creates a memory snapshot (full-store export under a label).
#[tauri::command]
pub async fn memory_snapshot_create(
    engine: State<'_, Arc<MemoryEngine>>,
    label: Option<String>,
) -> Result<MemorySnapshot, String> {
    engine
        .create_snapshot(label.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Lists stored snapshots, newest first.
#[tauri::command]
pub async fn memory_snapshot_list(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<MemorySnapshot>, String> {
    engine.list_snapshots().await.map_err(|e| e.to_string())
}

/// Restores the store from a snapshot (rebuilding the vector index).
#[tauri::command]
pub async fn memory_snapshot_restore(
    engine: State<'_, Arc<MemoryEngine>>,
    snapshot_id: String,
) -> Result<RestoreResult, String> {
    let snapshot_id = Uuid::parse_str(&snapshot_id).map_err(|e| e.to_string())?;
    engine
        .restore_snapshot(snapshot_id)
        .await
        .map_err(|e| e.to_string())
}

/// Storage statistics: database / vector index / cache sizes and
/// retention counts.
#[tauri::command]
pub async fn memory_storage_stats(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<MemoryStorageStats, String> {
    engine.storage_stats().await.map_err(|e| e.to_string())
}
