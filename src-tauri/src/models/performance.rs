//! Performance & Profiling models (RC-10 M1).
//!
//! DTOs for the production-hardening surfaces: profiler samples and
//! aggregates, startup stage timelines, benchmark results/suites, system
//! diagnostics, optimization recommendations, and the combined history
//! payload. Everything here is a plain serializable DTO — the SQL lives
//! in [`crate::repositories::PerformanceRepository`] and the measurement
//! logic in [`crate::performance`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ----------------------------------------------------------------------
// Profiling
// ----------------------------------------------------------------------

/// What kind of operation a sample captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileCategory {
    /// A Tauri IPC command handler.
    Command,
    /// A business-logic service call.
    Service,
    /// A repository (SQL) access.
    Repository,
    /// A background worker pass.
    Worker,
    /// An engine facade operation.
    Engine,
}

impl ProfileCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileCategory::Command => "command",
            ProfileCategory::Service => "service",
            ProfileCategory::Repository => "repository",
            ProfileCategory::Worker => "worker",
            ProfileCategory::Engine => "engine",
        }
    }
}

impl From<&str> for ProfileCategory {
    fn from(value: &str) -> Self {
        match value {
            "command" => ProfileCategory::Command,
            "service" => ProfileCategory::Service,
            "worker" => ProfileCategory::Worker,
            "engine" => ProfileCategory::Engine,
            _ => ProfileCategory::Repository,
        }
    }
}

/// One measured operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSample {
    pub id: i64,
    pub category: ProfileCategory,
    pub name: String,
    pub duration_ms: u64,
    pub metadata: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

/// Per-operation aggregate over a window (live ring + persisted history).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileAggregate {
    pub category: ProfileCategory,
    pub name: String,
    pub count: u64,
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
    /// 95th percentile latency over the live window.
    pub p95_ms: f64,
}

/// A point-in-time view of the profiler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSnapshot {
    pub captured_at: DateTime<Utc>,
    /// Aggregate per (category, name) over the live in-memory window.
    pub aggregates: Vec<ProfileAggregate>,
    /// Recent samples, newest-first, from the live window.
    pub recent: Vec<ProfileSample>,
    /// Slowest samples from the live window.
    pub slowest: Vec<ProfileSample>,
}

// ----------------------------------------------------------------------
// Startup profiling
// ----------------------------------------------------------------------

/// One timed startup phase (`database`, `graph_sync`, `copilot`, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStage {
    pub name: String,
    pub label: String,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
}

/// The full report of one application launch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupProfile {
    pub run_id: Uuid,
    pub total_ms: u64,
    pub stages: Vec<StartupStage>,
    pub recorded_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Benchmarks
// ----------------------------------------------------------------------

/// Which subsystem a benchmark suite exercises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkCategory {
    Planner,
    Execution,
    Memory,
    Graph,
    Vector,
}

impl BenchmarkCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            BenchmarkCategory::Planner => "planner",
            BenchmarkCategory::Execution => "execution",
            BenchmarkCategory::Memory => "memory",
            BenchmarkCategory::Graph => "graph",
            BenchmarkCategory::Vector => "vector",
        }
    }
}

impl From<&str> for BenchmarkCategory {
    fn from(value: &str) -> Self {
        match value {
            "execution" => BenchmarkCategory::Execution,
            "memory" => BenchmarkCategory::Memory,
            "graph" => BenchmarkCategory::Graph,
            "vector" => BenchmarkCategory::Vector,
            _ => BenchmarkCategory::Planner,
        }
    }
}

/// One measured micro-benchmark within a suite run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub id: i64,
    pub name: String,
    pub operation: String,
    pub category: BenchmarkCategory,
    /// Iterations run; `duration_ms` is the mean per iteration.
    pub iterations: u32,
    pub duration_ms: u64,
    /// Operations per second (`1000 / avg_ms`), when measurable.
    pub throughput_per_sec: Option<f64>,
    /// `false` when the benchmark could not complete (recorded anyway).
    pub ok: bool,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// The result of running one or more benchmark suites.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkSuiteResult {
    pub suite_name: String,
    pub benchmarks: Vec<BenchmarkResult>,
    pub total_duration_ms: u64,
    pub ran_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Diagnostics
// ----------------------------------------------------------------------

/// CPU-side system facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuUsage {
    /// Whole-system utilization since the last refresh, in `[0, 100]`.
    pub usage_percent: f32,
    pub cores: usize,
    pub cpu_parallelism: usize,
}

/// Physical memory usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub percent: f64,
}

/// On-disk database footprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbUsage {
    pub size_bytes: u64,
    pub path: String,
}

/// In-process cache health.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheUsage {
    /// Entries currently held by the runtime intelligence cache.
    pub runtime_entries: usize,
    /// Hits / (hits + misses) across the runtime cache.
    pub runtime_hit_rate: f64,
    /// Persisted graph query-cache entries.
    pub graph_cache_entries: u64,
    /// Bytes of cached graph query payloads.
    pub graph_cache_size_bytes: u64,
}

/// One background worker's observable state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerInfo {
    pub name: String,
    pub status: String,
    pub execution_count: u64,
    pub error_count: u64,
    pub avg_execution_time_ms: f64,
    pub last_execution: Option<DateTime<Utc>>,
}

/// Concurrency/process facts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadUsage {
    /// Sum of per-process thread counts. `0` on platforms `sysinfo`
    /// cannot enumerate threads for (e.g. macOS); the memory/CPU figures
    /// remain meaningful there.
    pub total_threads: usize,
    pub process_count: usize,
}

/// A point-in-time snapshot of the whole application + machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSnapshot {
    pub captured_at: DateTime<Utc>,
    pub cpu: CpuUsage,
    pub memory: MemoryUsage,
    pub db: DbUsage,
    pub cache: CacheUsage,
    pub workers: Vec<WorkerInfo>,
    pub threads: ThreadUsage,
}

// ----------------------------------------------------------------------
// Optimization
// ----------------------------------------------------------------------

/// The optimization surface a recommendation belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationCategory {
    Query,
    LazyInit,
    Worker,
    Cache,
    Memory,
}

impl OptimizationCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            OptimizationCategory::Query => "query",
            OptimizationCategory::LazyInit => "lazy_init",
            OptimizationCategory::Worker => "worker",
            OptimizationCategory::Cache => "cache",
            OptimizationCategory::Memory => "memory",
        }
    }
}

/// A remediation the optimizer can perform on the user's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationAction {
    /// Drop every expired graph query-cache entry.
    ClearExpiredGraphCache,
    /// Trim the `n` oldest graph query-cache entries.
    TrimGraphCache(u64),
    /// Prune `performance_profiles` history older than `days`.
    PruneProfileHistory(u64),
}

/// One actionable finding from the optimizer analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationRecommendation {
    pub id: String,
    pub category: OptimizationCategory,
    /// `info` | `warning` | `critical`.
    pub severity: String,
    pub title: String,
    pub detail: String,
    /// The safe remediation to apply, when one exists.
    pub action: Option<OptimizationAction>,
}

/// Output of an optimizer run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizeResult {
    pub recommendations: Vec<OptimizationRecommendation>,
    /// Recommendation ids whose action was applied.
    pub applied: Vec<String>,
    pub analyzed_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// History
// ----------------------------------------------------------------------

/// Combined recent history for the performance dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceHistory {
    pub profiles: Vec<ProfileSample>,
    pub benchmarks: Vec<BenchmarkResult>,
    pub startups: Vec<StartupProfile>,
}
