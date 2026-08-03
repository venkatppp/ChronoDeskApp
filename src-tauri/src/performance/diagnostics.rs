//! System diagnostics (RC-10 M1).
//!
//! [`Diagnostics`] assembles a point-in-time snapshot of machine, process,
//! and in-app health: CPU/RAM/threads via `sysinfo`, on-disk database
//! size via the performance repository, cache usage from the runtime
//! intelligence cache and the knowledge-graph query cache, and worker
//! status from the runtime health service. Everything here is read-only.

use chrono::Utc;
use sysinfo::System;

use crate::errors::DatabaseError;
use crate::graph::GraphEngine;
use crate::models::performance::{
    CacheUsage, CpuUsage, DbUsage, DiagnosticsSnapshot, MemoryUsage, ThreadUsage, WorkerInfo,
};
use crate::repositories::PerformanceRepository;
use crate::runtime::{IntelligenceCache, RuntimeHealthService};

/// Assembles system + application diagnostics on demand.
#[derive(Clone)]
pub struct Diagnostics {
    repository: PerformanceRepository,
    graph_engine: Option<GraphEngine>,
    runtime_health: Option<RuntimeHealthService>,
    intelligence_cache: Option<IntelligenceCache>,
    db_path: Option<String>,
}

impl Diagnostics {
    pub fn new(repository: PerformanceRepository) -> Self {
        Self {
            repository,
            graph_engine: None,
            runtime_health: None,
            intelligence_cache: None,
            db_path: None,
        }
    }

    /// Attaches the graph engine (graph query-cache statistics).
    pub fn with_graph_engine(mut self, graph_engine: GraphEngine) -> Self {
        self.graph_engine = Some(graph_engine);
        self
    }

    /// Attaches the runtime health service (worker status + hit rate).
    pub fn with_runtime_health(mut self, service: RuntimeHealthService) -> Self {
        self.runtime_health = Some(service);
        self
    }

    /// Attaches the runtime intelligence cache for entry counts.
    pub fn with_intelligence_cache(mut self, cache: IntelligenceCache) -> Self {
        self.intelligence_cache = Some(cache);
        self
    }

    /// Attaches the on-disk database path for the size report.
    pub fn with_db_path(mut self, path: impl Into<String>) -> Self {
        self.db_path = Some(path.into());
        self
    }

    /// Captures a full diagnostics snapshot.
    pub async fn capture(&self) -> Result<DiagnosticsSnapshot, DatabaseError> {
        let mut system = System::new_all();
        system.refresh_cpu_usage();
        system.refresh_memory();

        let cpu = CpuUsage {
            usage_percent: system.global_cpu_usage(),
            cores: num_cpus::get(),
            cpu_parallelism: num_cpus::get_physical(),
        };
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory = MemoryUsage {
            total_bytes: total_memory,
            used_bytes: used_memory,
            percent: if total_memory > 0 {
                used_memory as f64 / total_memory as f64 * 100.0
            } else {
                0.0
            },
        };

        let db = DbUsage {
            size_bytes: self.repository.db_size_bytes().await?,
            path: self
                .db_path
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        };

        let mut graph_cache_entries = 0;
        let mut graph_cache_size_bytes = 0;
        if let Some(graph_engine) = &self.graph_engine {
            if let Ok(stats) = graph_engine.graph_memory_stats().await {
                graph_cache_entries = stats.cache_entries;
                graph_cache_size_bytes = stats.cache_size_bytes;
            }
        }

        let runtime_hit_rate = self
            .runtime_health
            .as_ref()
            .map(|h| h.cache_hit_rate())
            .unwrap_or(0.0);
        let cache = CacheUsage {
            runtime_entries: self
                .intelligence_cache
                .as_ref()
                .map(|c| c.entry_count())
                .unwrap_or(0),
            runtime_hit_rate,
            graph_cache_entries,
            graph_cache_size_bytes,
        };

        let workers = match &self.runtime_health {
            Some(health) => {
                let snapshot = health.get_health().await;
                snapshot
                    .components
                    .into_iter()
                    .map(|c| WorkerInfo {
                        name: c.name,
                        status: format!("{:?}", c.status).to_lowercase(),
                        execution_count: c.execution_count,
                        error_count: c.error_count,
                        avg_execution_time_ms: c.avg_execution_time_ms,
                        last_execution: c.last_execution,
                    })
                    .collect()
            }
            None => Vec::new(),
        };

        let thread_count: usize = system
            .processes()
            .values()
            .map(|process| process.tasks().map(|tasks| tasks.len()).unwrap_or(0))
            .sum();
        let threads = ThreadUsage {
            total_threads: thread_count,
            process_count: system.processes().len(),
        };

        Ok(DiagnosticsSnapshot {
            captured_at: Utc::now(),
            cpu,
            memory,
            db,
            cache,
            workers,
            threads,
        })
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
