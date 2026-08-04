//! Performance & Profiling engine (RC-10 M1 — Production Hardening).
//!
//! Facade over the five performance subsystems — the live profiler,
//! startup profiler, benchmark engine, system diagnostics, and the
//! optimizer — plus the persisted performance history. `lib.rs` wires
//! one [`PerformanceEngine`] as managed Tauri state; the
//! [`crate::commands::performance`] commands are thin forwards to it.
//!
//! Dependency order (mirroring the rest of the codebase): the engine
//! composes repositories (SQL) and models (DTOs); measurement policy
//! lives in the per-subsystem modules; no SQL is written here.

pub mod benchmark;
pub mod diagnostics;
pub mod optimizer;
pub mod profiler;
pub mod recovery;
pub mod startup;

use chrono::Utc;

use crate::errors::DatabaseError;
use crate::graph::GraphEngine;
use crate::models::performance::{
    BenchmarkCategory, BenchmarkSuiteResult, DiagnosticsSnapshot, OptimizeResult,
    PerformanceHistory, ProfileSnapshot, StartupProfile,
};
use crate::repositories::PerformanceRepository;

pub use benchmark::BenchmarkEngine;
pub use diagnostics::Diagnostics;
pub use optimizer::Optimizer;
pub use profiler::PerformanceProfiler;
pub use startup::StartupProfiler;

/// Facade for all performance & profiling operations.
#[derive(Clone)]
pub struct PerformanceEngine {
    repository: PerformanceRepository,
    profiler: PerformanceProfiler,
    startup_profiler: StartupProfiler,
    benchmark_engine: BenchmarkEngine,
    diagnostics: Diagnostics,
    graph_engine: Option<GraphEngine>,
}

impl PerformanceEngine {
    /// Constructs the engine. The optional handles (graph engine) are
    /// attached once available; every subsystem is functional without
    /// them.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: PerformanceRepository,
        startup_profiler: StartupProfiler,
        profiler: PerformanceProfiler,
        benchmark_engine: BenchmarkEngine,
        diagnostics: Diagnostics,
    ) -> Self {
        Self {
            repository,
            profiler,
            startup_profiler,
            benchmark_engine,
            diagnostics,
            graph_engine: None,
        }
    }

    /// Attaches the graph engine (cache statistics + remediation).
    pub fn with_graph_engine(mut self, graph_engine: GraphEngine) -> Self {
        self.graph_engine = Some(graph_engine.clone());
        self.benchmark_engine = self
            .benchmark_engine
            .with_graph_engine(graph_engine.clone());
        self.diagnostics = self.diagnostics.with_graph_engine(graph_engine);
        self
    }

    /// Persists the startup profile once initialization completes.
    pub async fn record_startup(&self) -> Result<StartupProfile, DatabaseError> {
        self.startup_profiler.finish(&self.repository).await
    }

    /// The most recent startup profile (in-memory, no database read).
    pub async fn startup_profile(&self) -> Result<StartupProfile, DatabaseError> {
        match self.startup_profiler.latest() {
            Some(profile) => Ok(profile),
            None => self
                .repository
                .recent_startup_profiles(1)
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| DatabaseError::NotFound {
                    entity: "startup profile",
                    id: "none recorded yet".to_string(),
                }),
        }
    }

    /// Live profile snapshot (aggregates, recent, slowest).
    pub async fn profile(&self) -> Result<ProfileSnapshot, DatabaseError> {
        self.profiler.snapshot().await
    }

    /// Records one profiler sample (used by the command layer).
    pub async fn record_sample(
        &self,
        category: crate::models::performance::ProfileCategory,
        name: &str,
        duration_ms: u64,
        metadata: serde_json::Value,
    ) -> Result<(), DatabaseError> {
        self.profiler
            .record(category, name, duration_ms, metadata)
            .await
    }

    /// Runs a benchmark suite (or every suite when `category` is `None`).
    pub async fn benchmark(
        &self,
        category: Option<BenchmarkCategory>,
    ) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let started = std::time::Instant::now();
        let result = self.benchmark_engine.run(category).await?;
        let _ = self
            .profiler
            .record(
                crate::models::performance::ProfileCategory::Engine,
                "benchmark",
                started.elapsed().as_millis() as u64,
                serde_json::json!({ "suite": result.suite_name }),
            )
            .await;
        Ok(result)
    }

    /// Full system + application diagnostics snapshot.
    pub async fn diagnostics(&self) -> Result<DiagnosticsSnapshot, DatabaseError> {
        let started = std::time::Instant::now();
        let snapshot = self.diagnostics.capture().await?;
        let _ = self
            .profiler
            .record(
                crate::models::performance::ProfileCategory::Engine,
                "diagnostics",
                started.elapsed().as_millis() as u64,
                serde_json::Value::Null,
            )
            .await;
        Ok(snapshot)
    }

    /// Runs the optimizer analysis; applies safe remediations when
    /// `apply` is true.
    pub async fn optimize(&self, apply: bool) -> Result<OptimizeResult, DatabaseError> {
        let profile = self.profiler.snapshot().await?;
        let diagnostics = self.diagnostics.capture().await?;
        let startup = self.startup_profiler.latest();
        let persisted = self.profiler.persisted_count().await?;

        let recommendations =
            Optimizer::analyze(&profile, &diagnostics, startup.as_ref(), persisted);

        let mut applied = Vec::new();
        if apply {
            for recommendation in &recommendations {
                let Some(action) = recommendation.action else {
                    continue;
                };
                match self.apply_action(action).await {
                    Ok(true) => applied.push(recommendation.id.clone()),
                    _ => tracing::warn!(
                        id = %recommendation.id,
                        "optimization action could not be applied"
                    ),
                }
            }
        }

        Ok(OptimizeResult {
            recommendations,
            applied,
            analyzed_at: Utc::now(),
        })
    }

    /// Combined recent history (profiles + benchmarks + startups).
    pub async fn history(&self, limit: u32) -> Result<PerformanceHistory, DatabaseError> {
        let limit = limit.clamp(1, 500);
        Ok(PerformanceHistory {
            profiles: self.profiler.recent_samples(limit).await?,
            benchmarks: self.benchmark_engine.recent(limit).await?,
            startups: self
                .repository
                .recent_startup_profiles(limit.min(20))
                .await?,
        })
    }

    /// Applies one optimization action; returns whether anything changed.
    pub async fn apply_action(
        &self,
        action: crate::models::performance::OptimizationAction,
    ) -> Result<bool, DatabaseError> {
        match action {
            crate::models::performance::OptimizationAction::ClearExpiredGraphCache => {
                match &self.graph_engine {
                    Some(engine) => Ok(engine.graph_clear_expired_cache().await? > 0),
                    None => Ok(false),
                }
            }
            crate::models::performance::OptimizationAction::TrimGraphCache(n) => {
                match &self.graph_engine {
                    Some(engine) => Ok(engine.graph_cache_trim(n).await? > 0),
                    None => Ok(false),
                }
            }
            crate::models::performance::OptimizationAction::PruneProfileHistory(days) => {
                Ok(self.profiler.prune_older_than(days).await? > 0)
            }
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
