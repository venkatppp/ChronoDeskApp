//! RC-8 M4 graph optimization & scale IPC commands.
//!
//! Thin wrappers only: every command pulls the [`GraphEngine`] state and
//! forwards to its facade method with argument-shape fixes (optional
//! booleans/sizes). Zero business logic lives here.

use tauri::State;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::graph::GraphEngine;
use crate::models::kg::GraphNodeType;
use crate::models::kg_opt::{
    BenchmarkSuiteResult, ConsistencyReport, EdgePage, GraphDiagnostics, GraphMemoryStats,
    IntegrityCheckResult, MaintenanceRun, NeighborPage, NodePage, OrphanCleanupResult,
    OrphanSummary, ParallelWalkResult, QueryMetric, RankedSearchHit, RepairResult,
};

/// One page of graph nodes (progressive loading).
#[tauri::command]
pub async fn graph_nodes_page(
    engine: State<'_, GraphEngine>,
    node_types: Option<Vec<GraphNodeType>>,
    workspace_id: Option<Uuid>,
    offset: Option<u64>,
    limit: Option<u32>,
) -> Result<NodePage, DatabaseError> {
    engine
        .graph_nodes_page(node_types, workspace_id, offset.unwrap_or(0), limit)
        .await
}

/// One page of graph edges (progressive loading).
#[tauri::command]
pub async fn graph_edges_page(
    engine: State<'_, GraphEngine>,
    offset: Option<u64>,
    limit: Option<u32>,
) -> Result<EdgePage, DatabaseError> {
    engine.graph_edges_page(offset.unwrap_or(0), limit).await
}

/// One page of a node's neighbors (relationship inspector pages).
#[tauri::command]
pub async fn graph_neighbors_page(
    engine: State<'_, GraphEngine>,
    node_type: GraphNodeType,
    entity_id: Uuid,
    offset: Option<u64>,
    limit: Option<u32>,
) -> Result<NeighborPage, DatabaseError> {
    engine
        .graph_neighbors_page(node_type, entity_id, offset.unwrap_or(0), limit)
        .await
}

/// Total node count for the given filters (virtualized list header).
#[tauri::command]
pub async fn graph_nodes_total(
    engine: State<'_, GraphEngine>,
    node_types: Option<Vec<GraphNodeType>>,
    workspace_id: Option<Uuid>,
) -> Result<u64, DatabaseError> {
    engine.graph_nodes_total(node_types, workspace_id).await
}

/// Keyword search re-ranked by match quality and recency.
#[tauri::command]
pub async fn graph_ranked_search(
    engine: State<'_, GraphEngine>,
    query: String,
    node_types: Option<Vec<GraphNodeType>>,
    limit: Option<u32>,
) -> Result<Vec<RankedSearchHit>, DatabaseError> {
    engine.graph_ranked_search(&query, node_types, limit).await
}

/// Cosine-ranked vector search over node titles.
#[tauri::command]
pub async fn graph_vector_search(
    engine: State<'_, GraphEngine>,
    query: String,
    node_types: Option<Vec<GraphNodeType>>,
    limit: Option<u32>,
) -> Result<Vec<RankedSearchHit>, DatabaseError> {
    engine.graph_vector_search(&query, node_types, limit).await
}

/// Parallel (rayon) multi-root BFS traversal.
#[tauri::command]
pub async fn graph_parallel_traverse(
    engine: State<'_, GraphEngine>,
    roots: Vec<(GraphNodeType, Uuid)>,
    max_depth: Option<usize>,
    budget: Option<usize>,
) -> Result<ParallelWalkResult, DatabaseError> {
    engine
        .graph_parallel_traverse(roots, max_depth, budget)
        .await
}

/// Drops the `n` oldest cached query entries.
#[tauri::command]
pub async fn graph_cache_trim(
    engine: State<'_, GraphEngine>,
    n: u64,
) -> Result<u64, DatabaseError> {
    engine.graph_cache_trim(n).await
}

/// Drops every cached entry past its TTL.
#[tauri::command]
pub async fn graph_clear_expired_cache(
    engine: State<'_, GraphEngine>,
) -> Result<u64, DatabaseError> {
    engine.graph_clear_expired_cache().await
}

/// Graph memory statistics (registry + cache footprint).
#[tauri::command]
pub async fn graph_memory_stats(
    engine: State<'_, GraphEngine>,
) -> Result<GraphMemoryStats, DatabaseError> {
    engine.graph_memory_stats().await
}

/// Most recent recorded operation metrics.
#[tauri::command]
pub async fn graph_recent_metrics(
    engine: State<'_, GraphEngine>,
    limit: Option<u32>,
) -> Result<Vec<QueryMetric>, DatabaseError> {
    engine.graph_recent_metrics(limit).await
}

/// Runs the four integrity scans and persists new findings.
#[tauri::command]
pub async fn graph_integrity_check(
    engine: State<'_, GraphEngine>,
) -> Result<IntegrityCheckResult, DatabaseError> {
    engine.graph_integrity_check().await
}

/// Repairs every detectable graph problem.
#[tauri::command]
pub async fn graph_repair(engine: State<'_, GraphEngine>) -> Result<RepairResult, DatabaseError> {
    engine.graph_repair().await
}

/// Read-only orphan bookkeeping.
#[tauri::command]
pub async fn graph_orphan_summary(
    engine: State<'_, GraphEngine>,
) -> Result<OrphanSummary, DatabaseError> {
    engine.graph_orphan_summary().await
}

/// Removes every orphan edge and dangling workspace node.
#[tauri::command]
pub async fn graph_orphan_cleanup(
    engine: State<'_, GraphEngine>,
) -> Result<OrphanCleanupResult, DatabaseError> {
    engine.graph_orphan_cleanup().await
}

/// Runs the five consistency probes.
#[tauri::command]
pub async fn graph_consistency_report(
    engine: State<'_, GraphEngine>,
) -> Result<ConsistencyReport, DatabaseError> {
    engine.graph_consistency_report().await
}

/// Most recent maintenance runs, newest first.
#[tauri::command]
pub async fn graph_maintenance_runs(
    engine: State<'_, GraphEngine>,
    limit: Option<u32>,
) -> Result<Vec<MaintenanceRun>, DatabaseError> {
    engine.graph_maintenance_runs(limit).await
}

/// Runs + persists the micro-benchmark suite.
#[tauri::command]
pub async fn graph_benchmark_suite(
    engine: State<'_, GraphEngine>,
    suite_name: Option<String>,
) -> Result<BenchmarkSuiteResult, DatabaseError> {
    engine.graph_benchmark_suite(suite_name).await
}

/// The full graph performance/health diagnostics bundle.
#[tauri::command]
pub async fn graph_diagnostics(
    engine: State<'_, GraphEngine>,
) -> Result<GraphDiagnostics, DatabaseError> {
    engine.graph_diagnostics().await
}
