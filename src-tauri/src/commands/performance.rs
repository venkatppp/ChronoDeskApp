//! RC-10 M1 performance & profiling IPC commands.
//!
//! Thin wrappers only: every command pulls the [`PerformanceEngine`]
//! state and forwards to its facade method, timing its own execution so
//! the command-timing surface of the profiler is populated from real
//! invocations. Zero business logic lives here.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::errors::DatabaseError;
use crate::models::performance::{
    BenchmarkCategory, BenchmarkSuiteResult, DiagnosticsSnapshot, OptimizeResult,
    PerformanceHistory, ProfileCategory, ProfileSnapshot, StartupProfile,
};
use crate::performance::PerformanceEngine;

/// Executes the engine call and records the command's own duration into
/// the profiler, then returns the result unchanged.
async fn timed<T>(
    name: &str,
    engine: &PerformanceEngine,
    op: impl std::future::Future<Output = T>,
) -> T {
    let started = Instant::now();
    let result = op.await;
    let duration = started.elapsed().as_millis() as u64;
    let _ = engine
        .record_sample(ProfileCategory::Command, name, duration, json!({}))
        .await;
    result
}

/// Live profile snapshot (aggregates, recent samples, slowest).
#[tauri::command]
pub async fn performance_profile(
    engine: State<'_, PerformanceEngine>,
) -> Result<ProfileSnapshot, DatabaseError> {
    timed("performance_profile", &engine, engine.profile()).await
}

/// The most recent startup profile (per-stage breakdown + total).
#[tauri::command]
pub async fn performance_startup(
    engine: State<'_, PerformanceEngine>,
) -> Result<StartupProfile, DatabaseError> {
    timed("performance_startup", &engine, engine.startup_profile()).await
}

/// Runs one benchmark suite, or every suite when `category` is omitted.
#[tauri::command]
pub async fn performance_benchmark(
    engine: State<'_, PerformanceEngine>,
    category: Option<BenchmarkCategory>,
) -> Result<BenchmarkSuiteResult, DatabaseError> {
    timed("performance_benchmark", &engine, engine.benchmark(category)).await
}

/// System + application diagnostics snapshot.
#[tauri::command]
pub async fn performance_diagnostics(
    engine: State<'_, PerformanceEngine>,
) -> Result<DiagnosticsSnapshot, DatabaseError> {
    timed("performance_diagnostics", &engine, engine.diagnostics()).await
}

/// Runs the optimizer analysis and (optionally) applies safe actions.
#[tauri::command]
pub async fn performance_optimize(
    engine: State<'_, PerformanceEngine>,
    apply: Option<bool>,
) -> Result<OptimizeResult, DatabaseError> {
    let apply = apply.unwrap_or(false);
    timed("performance_optimize", &engine, engine.optimize(apply)).await
}

/// Combined recent history: profile samples, benchmarks, startup runs.
#[tauri::command]
pub async fn performance_history(
    engine: State<'_, PerformanceEngine>,
    limit: Option<u32>,
) -> Result<PerformanceHistory, DatabaseError> {
    timed(
        "performance_history",
        &engine,
        engine.history(limit.unwrap_or(50)),
    )
    .await
}
