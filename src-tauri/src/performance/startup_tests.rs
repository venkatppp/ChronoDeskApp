//! StartupProfiler tests (RC-10 M1): stage measurement via markers and
//! closures, early-return handling (open stages dropped), persistence
//! of a run, and retrieval of the in-memory latest report.

use super::*;
use crate::database::test_database;

async fn setup() -> (
    StartupProfiler,
    PerformanceRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let repo = PerformanceRepository::new(pool.clone());
    (StartupProfiler::new(), repo, pool, temp_dir)
}

#[tokio::test]
async fn stage_markers_time_and_finish_persists_a_run() {
    let (profiler, repo, _pool, _guard) = setup().await;
    profiler.stage_start("database", "Database initialization");
    std::thread::sleep(std::time::Duration::from_millis(5));
    profiler.stage_end();
    profiler.stage_start("engines", "Engine construction");
    std::thread::sleep(std::time::Duration::from_millis(2));
    profiler.stage_end();

    let profile = profiler.finish(&repo).await.unwrap();
    assert_eq!(profile.stages.len(), 2);
    assert!(profile.stages.iter().all(|s| s.duration_ms > 0));
    assert_eq!(
        profile.total_ms,
        profile.stages.iter().map(|s| s.duration_ms).sum::<u64>()
    );
    assert_eq!(profiler.latest().unwrap().run_id, profile.run_id);

    let runs = repo.recent_startup_profiles(5).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].stages.len(), 2);
}

#[tokio::test]
async fn stage_closure_helper_returns_value_and_times() {
    let (profiler, repo, _pool, _guard) = setup().await;
    let answer = profiler.stage("probe", "Probe stage", || 42);
    assert_eq!(answer, 42);
    let profile = profiler.finish(&repo).await.unwrap();
    assert_eq!(profile.stages[0].name, "probe");
}

#[tokio::test]
async fn open_stages_are_dropped_on_early_finish() {
    let (profiler, repo, _pool, _guard) = setup().await;
    profiler.stage_start("database", "Database initialization");
    std::thread::sleep(std::time::Duration::from_millis(2));
    profiler.stage_end();
    // Simulate an early return: a stage opened but never ended.
    profiler.stage_start("interrupted", "Interrupted stage");

    let profile = profiler.finish(&repo).await.unwrap();
    assert_eq!(profile.stages.len(), 1);
    assert_eq!(profile.stages[0].name, "database");
}

#[tokio::test]
async fn runs_are_grouped_and_ordered_newest_first() {
    let (profiler, repo, _pool, _guard) = setup().await;
    profiler.stage("a", "A", || {});
    profiler.finish(&repo).await.unwrap();
    profiler.stage("b", "B", || {});
    profiler.finish(&repo).await.unwrap();

    let runs = repo.recent_startup_profiles(5).await.unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].stages[0].name, "b");
}
