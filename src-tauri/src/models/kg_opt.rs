//! Knowledge Graph Optimization & Scale models (RC-8 M4).
//!
//! Additive to `models::kg` (M1) and `models::kg_live` (M2): paginated
//! graph loading (node/edge/neighbor pages), integrity + health models
//! (issues, repair, orphans, consistency reports), performance/memory
//! statistics, maintenance history, persisted query metrics, and
//! benchmark results. Everything here is a plain serializable DTO — the
//! SQL lives in [`crate::repositories::KgOptRepository`], the algorithms
//! (ranking, parallel traversal, benchmark timing) in
//! [`crate::services::KgOptService`] and
//! [`crate::services::GraphHealthService`].
//!
//! Enums are decoded via the codebase's `*Row` + `TryFrom` pattern when
//! they cross the database; the lightweight enums below (`IssueType`,
//! `IssueSeverity`, `MaintenanceRunType`, `CheckStatus`) carry only
//! string dumps/serialization and need no fallible decode.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::kg::{KgEdge, KgNode};

// ----------------------------------------------------------------------
// Pagination
// ----------------------------------------------------------------------

/// One page of graph nodes (progressive / virtualized loading).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePage {
    pub nodes: Vec<KgNode>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub has_more: bool,
}

/// One page of graph edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgePage {
    pub edges: Vec<KgEdge>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub has_more: bool,
}

/// One neighbor of a node: the edge that connects them plus the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborRow {
    pub edge: KgEdge,
    pub neighbor: KgNode,
}

/// One page of a node's neighbors.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborPage {
    pub neighbors: Vec<NeighborRow>,
    pub total: u64,
    pub offset: u64,
    pub limit: u32,
    pub has_more: bool,
}

/// One ranked hit from the optimization search surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedSearchHit {
    pub node: KgNode,
    /// Normalized rank score in `[0, 1]`, sorted descending.
    pub score: f64,
    /// Which matcher produced the hit (`keyword` | `vector`).
    pub method: String,
    /// Human-readable reason for the rank.
    pub reason: String,
}

// ----------------------------------------------------------------------
// Integrity & health
// ----------------------------------------------------------------------

/// The four integrity categories the health service can detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// A `graph_relationships` row whose endpoint node is missing.
    OrphanEdge,
    /// A workspace-scoped node whose `workspace_id` no longer exists.
    DanglingWorkspace,
    /// A node row with an empty title/summary or unknown node type.
    MalformedNode,
    /// An edge whose `confidence`/`weight` fell outside `[0, 1]`.
    InvalidConfidence,
}

impl IssueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueType::OrphanEdge => "orphan_edge",
            IssueType::DanglingWorkspace => "dangling_workspace",
            IssueType::MalformedNode => "malformed_node",
            IssueType::InvalidConfidence => "invalid_confidence",
        }
    }
}

impl IssueType {
    /// Parses a stored `issue_type` column value back into the enum.
    pub fn from_stored(value: &str) -> Option<Self> {
        match value {
            "orphan_edge" => Some(IssueType::OrphanEdge),
            "dangling_workspace" => Some(IssueType::DanglingWorkspace),
            "malformed_node" => Some(IssueType::MalformedNode),
            "invalid_confidence" => Some(IssueType::InvalidConfidence),
            _ => None,
        }
    }
}

/// Severity attached to an integrity finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Info,
    Warning,
    Critical,
}

impl IssueSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueSeverity::Info => "info",
            IssueSeverity::Warning => "warning",
            IssueSeverity::Critical => "critical",
        }
    }
}

/// One persisted integrity finding (`graph_integrity_issues` row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphIntegrityIssue {
    pub id: i64,
    pub issue_type: IssueType,
    pub severity: IssueSeverity,
    /// Node type of the affected node, when applicable.
    pub node_type: Option<crate::models::kg::GraphNodeType>,
    /// The affected node/edge id.
    pub entity_id: Option<Uuid>,
    pub detail: String,
    /// `open` | `resolved`.
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Accounting + findings of one integrity check pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityCheckResult {
    pub issues: Vec<GraphIntegrityIssue>,
    pub issue_type_counts: Vec<crate::models::kg::TypeCount>,
    pub checked_at: DateTime<Utc>,
}

/// Type-level resolution counts of one repair pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairResult {
    pub orphan_edges_removed: u64,
    pub dangling_workspaces_removed: u64,
    pub malformed_nodes_fixed: u64,
    pub invalid_confidence_fixed: u64,
    pub issues_resolved: u64,
}

/// Live orphan bookkeeping without mutating anything.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanSummary {
    pub orphan_edges: u64,
    pub dangling_workspaces: u64,
}

/// Accounting of one orphan cleanup pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanCleanupResult {
    pub orphan_edges_removed: u64,
    pub dangling_workspaces_removed: u64,
    pub issues_resolved: u64,
}

/// One consistency verification question (e.g. "no forward-reference
/// edges", "cache key sanity") with a pass/fail verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Aggregate consistency report for the diagnostics panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyReport {
    pub checks: Vec<ConsistencyCheck>,
    pub passed: bool,
    pub checked_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Performance observability
// ----------------------------------------------------------------------

/// One recorded operation metric (`graph_query_metrics` row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryMetric {
    pub id: i64,
    pub operation: String,
    pub scope: Option<String>,
    pub query: Option<String>,
    pub duration_ms: i64,
    pub rows_returned: i64,
    pub hit_cache: bool,
    pub occurred_at: DateTime<Utc>,
}

/// Cached + persisted graph memory bookkeeping for the dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphMemoryStats {
    pub node_count: u64,
    pub edge_count: u64,
    pub cache_entries: u64,
    /// Total bytes of cached payloads (sum of `LENGTH(payload)`).
    pub cache_size_bytes: u64,
    /// Rough in-memory footprint estimate of the node/edge registry.
    pub estimated_bytes: u64,
}

/// Maintenance run record for the history panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceRun {
    pub id: i64,
    pub run_type: String,
    pub status: String,
    pub issues_found: u64,
    pub issues_resolved: u64,
    pub duration_ms: u64,
    pub summary: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

// ----------------------------------------------------------------------
// Benchmarks
// ----------------------------------------------------------------------

/// One micro-benchmark result within a suite run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphBenchmarkResult {
    pub name: String,
    pub operation: String,
    pub node_count: u64,
    pub edge_count: u64,
    /// Wall time of the benchmarked call.
    pub duration_ms: u64,
    /// Throughput (rows/ops per second) where meaningful.
    pub throughput_per_sec: Option<u64>,
    /// Suite name grouping this benchmark.
    pub suite_name: String,
    pub created_at: DateTime<Utc>,
}

/// Aggregate payload of one benchmark suite run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkSuiteResult {
    pub suite_name: String,
    pub benchmarks: Vec<GraphBenchmarkResult>,
    pub total_duration_ms: u64,
    pub ran_at: DateTime<Utc>,
}

/// Accounting + payload of a parallel multi-root traversal pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelWalkResult {
    pub roots: usize,
    /// Unique nodes reached from all roots, deduplicated.
    pub nodes: Vec<KgNode>,
    /// Edges whose both endpoints were reached.
    pub edges: Vec<KgEdge>,
    pub node_count: u64,
    pub edge_count: u64,
    /// Maximum BFS depth explored from any root.
    pub max_depth: usize,
    pub duration_ms: u64,
}

// ----------------------------------------------------------------------
// Diagnostics bundle
// ----------------------------------------------------------------------

/// The full graph performance/health bundle the frontend page renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDiagnostics {
    pub integrity: IntegrityCheckResult,
    pub consistency: ConsistencyReport,
    pub memory: GraphMemoryStats,
    pub recent_maintenance: Vec<MaintenanceRun>,
    pub recent_benchmarks: Vec<GraphBenchmarkResult>,
    pub recent_metrics: Vec<QueryMetric>,
}
