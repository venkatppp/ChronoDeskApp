//! PerformanceEngine integration tests (RC-10 M1): the facade wires
//! profiler, startup profiler, benchmark engine, diagnostics, optimizer,
//! and history together over a disposable database.

use super::*;
use crate::database::test_database;
use crate::models::performance::{BenchmarkCategory, ProfileCategory};
use serde_json::json;

async fn setup() -> (PerformanceEngine, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let repository = PerformanceRepository::new(pool.clone());

    let startup_profiler = StartupProfiler::new();
    let profiler = PerformanceProfiler::new(repository.clone());
    let benchmark_engine = BenchmarkEngine::new(repository.clone(), None, None, None, None, None);
    let diagnostics = Diagnostics::new(repository.clone()).with_db_path("test.db");

    let engine = PerformanceEngine::new(
        repository,
        startup_profiler,
        profiler,
        benchmark_engine,
        diagnostics,
    );
    (engine, pool, temp_dir)
}

#[tokio::test]
async fn record_startup_persists_and_is_retrievable() {
    let (engine, _pool, _guard) = setup().await;
    engine
        .startup_profiler
        .stage("database", "Database initialization", || {});
    engine
        .startup_profiler
        .stage("engines", "Engine construction", || {});

    let profile = engine.record_startup().await.unwrap();
    assert_eq!(profile.stages.len(), 2);

    let latest = engine.startup_profile().await.unwrap();
    assert_eq!(latest.run_id, profile.run_id);
}

#[tokio::test]
async fn profile_snapshot_round_trips_through_profiler() {
    let (engine, _pool, _guard) = setup().await;
    engine
        .record_sample(
            ProfileCategory::Command,
            "performance_profile",
            3,
            json!({}),
        )
        .await
        .unwrap();
    let snapshot = engine.profile().await.unwrap();
    assert!(snapshot
        .aggregates
        .iter()
        .any(|a| a.name == "performance_profile" && a.count == 1));
}

#[tokio::test]
async fn benchmark_runs_and_is_recorded() {
    let (engine, _pool, _guard) = setup().await;
    let result = engine
        .benchmark(Some(BenchmarkCategory::Graph))
        .await
        .unwrap();
    // No graph engine attached: the suite reports a skipped entry.
    assert_eq!(result.benchmarks.len(), 1);
    assert!(!result.benchmarks[0].ok);
}

#[tokio::test]
async fn diagnostics_capture_is_reported() {
    let (engine, _pool, _guard) = setup().await;
    let snapshot = engine.diagnostics().await.unwrap();
    assert!(snapshot.cpu.cores >= 1);
    assert_eq!(snapshot.db.path, "test.db");
}

#[tokio::test]
async fn optimize_returns_recommendations_and_applies_none_when_disabled() {
    let (engine, _pool, _guard) = setup().await;
    let result = engine.optimize(false).await.unwrap();
    assert!(result.applied.is_empty());
    // A fresh database with no activity yields no recommendations.
    assert!(result.recommendations.is_empty());
}

#[tokio::test]
async fn history_groups_profiles_benchmarks_and_startups() {
    let (engine, _pool, _guard) = setup().await;
    engine
        .record_sample(
            ProfileCategory::Service,
            "workspace_service.list",
            2,
            json!({}),
        )
        .await
        .unwrap();
    let _ = engine
        .benchmark(Some(BenchmarkCategory::Memory))
        .await
        .unwrap();
    engine.startup_profiler.stage("probe", "Probe", || {});
    engine.record_startup().await.unwrap();

    let history = engine.history(50).await.unwrap();
    assert!(!history.profiles.is_empty());
    assert!(!history.benchmarks.is_empty());
    assert_eq!(history.startups.len(), 1);
}

#[tokio::test]
async fn prune_action_applies_when_history_exists() {
    let (engine, _pool, _guard) = setup().await;
    engine
        .record_sample(
            ProfileCategory::Command,
            "performance_profile",
            1,
            json!({}),
        )
        .await
        .unwrap();
    let applied = engine
        .apply_action(crate::models::performance::OptimizationAction::PruneProfileHistory(0))
        .await
        .unwrap();
    assert!(applied);
    assert_eq!(engine.profiler.persisted_count().await.unwrap(), 0);
}
