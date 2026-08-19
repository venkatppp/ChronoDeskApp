//! Performance optimization analysis (RC-10 M1).
//!
//! [`Optimizer`] turns the profiler/diagnostics/startup observations into
//! actionable, severity-tagged recommendations across five surfaces:
//! query optimization, lazy initialization, background workers, caches,
//! and memory. The analysis is pure (no I/O); applying a recommendation
//! is the [`crate::performance::PerformanceEngine`]'s job since it owns
//! the handles.

use crate::models::performance::{
    DiagnosticsSnapshot, OptimizationAction, OptimizationCategory, OptimizationRecommendation,
    ProfileSnapshot, StartupProfile,
};

/// Latency threshold (ms) past which an average operation is a candidate
/// for query optimization.
const QUERY_AVG_WARNING_MS: f64 = 250.0;
const QUERY_AVG_CRITICAL_MS: f64 = 1000.0;
/// Startup stage duration past which deferral to the background pays off.
const STARTUP_SYNC_WARNING_MS: u64 = 400;
const STARTUP_SYNC_CRITICAL_MS: u64 = 1500;
/// Worker error threshold before a recommendation is raised.
const WORKER_ERROR_WARNING: u64 = 1;
const WORKER_AVG_SLOW_MS: f64 = 5000.0;
/// Runtime cache hit rate below which the cache may be undersized.
const CACHE_HIT_RATE_WARNING: f64 = 0.5;
/// Graph cache entries beyond which trimming is suggested.
const GRAPH_CACHE_ENTRY_WARNING: u64 = 5000;
/// Graph cache payload bytes beyond which expired entries are swept.
const GRAPH_CACHE_SIZE_WARNING_BYTES: u64 = 50 * 1024 * 1024;
/// System memory pressure threshold.
const MEMORY_PRESSURE_PERCENT: f64 = 85.0;
/// Database size beyond which compaction is worth reviewing.
const DB_SIZE_WARNING_BYTES: u64 = 1024 * 1024 * 1024;
/// Persisted profile samples beyond which history pruning is offered.
const PROFILE_LEDGER_WARNING: u64 = 10_000;

/// Pure recommendation engine. No state; every rule is a function of the
/// inputs.
pub struct Optimizer;

impl Optimizer {
    /// Analyzes the current observations and returns recommendations
    /// ordered by severity (critical first).
    pub fn analyze(
        profile: &ProfileSnapshot,
        diagnostics: &DiagnosticsSnapshot,
        startup: Option<&StartupProfile>,
        persisted_profile_count: u64,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        // 1. Query optimization — slow operations from the live window.
        for aggregate in &profile.aggregates {
            let severity = if aggregate.avg_ms >= QUERY_AVG_CRITICAL_MS {
                "critical"
            } else if aggregate.avg_ms >= QUERY_AVG_WARNING_MS {
                "warning"
            } else {
                continue;
            };
            recommendations.push(OptimizationRecommendation {
                id: format!("query:{}:{}", aggregate.category.as_str(), aggregate.name),
                category: OptimizationCategory::Query,
                severity: severity.to_string(),
                title: format!(
                    "Slow {} operation: {}",
                    aggregate.category.as_str(),
                    aggregate.name
                ),
                detail: format!(
                    "Average {:.0} ms (p95 {:.0} ms) across {} samples. Review the query plan or consider an index.",
                    aggregate.avg_ms, aggregate.p95_ms, aggregate.count
                ),
                action: None,
            });
        }

        // 2. Lazy initialization — heavy synchronous startup stages.
        if let Some(startup) = startup {
            for stage in &startup.stages {
                let severity = if stage.duration_ms >= STARTUP_SYNC_CRITICAL_MS {
                    "critical"
                } else if stage.duration_ms >= STARTUP_SYNC_WARNING_MS {
                    "warning"
                } else {
                    continue;
                };
                recommendations.push(OptimizationRecommendation {
                    id: format!("lazy_init:{}", stage.name),
                    category: OptimizationCategory::LazyInit,
                    severity: severity.to_string(),
                    title: format!("Startup stage \"{}\" blocks launch", stage.label),
                    detail: format!(
                        "Took {} ms during initialization. Deferring it to a background task would shorten perceived startup.",
                        stage.duration_ms
                    ),
                    action: None,
                });
            }
        }

        // 3. Background worker optimization.
        for worker in &diagnostics.workers {
            if worker.error_count >= WORKER_ERROR_WARNING {
                recommendations.push(OptimizationRecommendation {
                    id: format!("worker:errors:{}", worker.name),
                    category: OptimizationCategory::Worker,
                    severity: "warning".to_string(),
                    title: format!("Worker \"{}\" is failing", worker.name),
                    detail: format!(
                        "{} of {} passes errored (avg {:.0} ms/pass). Inspect the worker's last error before tuning its interval.",
                        worker.error_count, worker.execution_count, worker.avg_execution_time_ms
                    ),
                    action: None,
                });
            } else if worker.avg_execution_time_ms > WORKER_AVG_SLOW_MS
                && worker.execution_count > 0
            {
                recommendations.push(OptimizationRecommendation {
                    id: format!("worker:slow:{}", worker.name),
                    category: OptimizationCategory::Worker,
                    severity: "info".to_string(),
                    title: format!("Worker \"{}\" runs slow passes", worker.name),
                    detail: format!(
                        "Average pass takes {:.0} ms; a longer interval may reduce contention.",
                        worker.avg_execution_time_ms
                    ),
                    action: None,
                });
            }
        }

        // 4. Cache optimization.
        if diagnostics.cache.runtime_hit_rate > 0.0
            && diagnostics.cache.runtime_hit_rate < CACHE_HIT_RATE_WARNING
        {
            recommendations.push(OptimizationRecommendation {
                id: "cache:hit_rate".to_string(),
                category: OptimizationCategory::Cache,
                severity: "warning".to_string(),
                title: "Runtime cache hit rate is low".to_string(),
                detail: format!(
                    "{:.0}% of runtime cache lookups miss ({:?} entries). Increasing TTLs would raise the hit rate.",
                    diagnostics.cache.runtime_hit_rate * 100.0,
                    diagnostics.cache.runtime_entries
                ),
                action: None,
            });
        }
        if diagnostics.cache.graph_cache_entries >= GRAPH_CACHE_ENTRY_WARNING {
            recommendations.push(OptimizationRecommendation {
                id: "cache:graph_trim".to_string(),
                category: OptimizationCategory::Cache,
                severity: "warning".to_string(),
                title: "Graph query cache is large".to_string(),
                detail: format!(
                    "{} cached queries. Trimming the oldest 500 releases memory without dropping hot entries.",
                    diagnostics.cache.graph_cache_entries
                ),
                action: Some(OptimizationAction::TrimGraphCache(500)),
            });
        }
        if diagnostics.cache.graph_cache_size_bytes >= GRAPH_CACHE_SIZE_WARNING_BYTES
            && diagnostics.cache.graph_cache_entries > 0
        {
            recommendations.push(OptimizationRecommendation {
                id: "cache:graph_expired".to_string(),
                category: OptimizationCategory::Cache,
                severity: "info".to_string(),
                title: "Expired graph cache entries can be swept".to_string(),
                detail: format!(
                    "{} MB of cached payloads. Sweeping expired entries costs nothing.",
                    diagnostics.cache.graph_cache_size_bytes / (1024 * 1024)
                ),
                action: Some(OptimizationAction::ClearExpiredGraphCache),
            });
        }

        // 5. Memory optimization.
        if diagnostics.memory.percent >= MEMORY_PRESSURE_PERCENT {
            recommendations.push(OptimizationRecommendation {
                id: "memory:pressure".to_string(),
                category: OptimizationCategory::Memory,
                severity: "critical".to_string(),
                title: "System memory pressure".to_string(),
                detail: format!(
                    "{:.0}% of {} RAM is in use. Close other applications or reduce ContextSphere's cache footprint.",
                    diagnostics.memory.percent,
                    format_bytes(diagnostics.memory.total_bytes)
                ),
                action: None,
            });
        }
        if diagnostics.db.size_bytes >= DB_SIZE_WARNING_BYTES {
            recommendations.push(OptimizationRecommendation {
                id: "memory:db_size".to_string(),
                category: OptimizationCategory::Memory,
                severity: "info".to_string(),
                title: "Database is growing large".to_string(),
                detail: format!(
                    "The database is {} on disk. Reviewing pruning policies and running maintenance keeps it bounded.",
                    format_bytes(diagnostics.db.size_bytes)
                ),
                action: None,
            });
        }
        if persisted_profile_count >= PROFILE_LEDGER_WARNING {
            recommendations.push(OptimizationRecommendation {
                id: "memory:profile_ledger".to_string(),
                category: OptimizationCategory::Memory,
                severity: "info".to_string(),
                title: "Profiler history is large".to_string(),
                detail: format!(
                    "{persisted_profile_count} persisted samples. Pruning history older than 30 days keeps the ledger bounded."
                ),
                action: Some(OptimizationAction::PruneProfileHistory(30)),
            });
        }

        recommendations.sort_by(|a, b| {
            severity_rank(b.severity.as_str()).cmp(&severity_rank(a.severity.as_str()))
        });
        recommendations
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 3,
        "warning" => 2,
        _ => 1,
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
#[path = "optimizer_tests.rs"]
mod tests;
