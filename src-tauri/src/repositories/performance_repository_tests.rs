//! PerformanceRepository tests (RC-10 M1): profile sample persistence,
//! benchmark ledger, startup-profile run grouping, database-size PRAGMAs,
//! and history pruning.

use super::*;
use crate::database::test_database;
use crate::models::performance::{
    BenchmarkCategory, BenchmarkResult, ProfileCategory, StartupStage,
};
use serde_json::json;

async fn setup() -> (PerformanceRepository, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (PerformanceRepository::new(pool.clone()), pool, temp_dir)
}

fn sample_benchmark() -> BenchmarkResult {
    BenchmarkResult {
        id: 0,
        name: "memory_search".into(),
        operation: "search".into(),
        category: BenchmarkCategory::Memory,
        iterations: 5,
        duration_ms: 12,
        throughput_per_sec: Some(83.33),
        ok: true,
        payload: json!({}),
        created_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn record_and_recent_profiles_round_trip() {
    let (repo, _pool, _guard) = setup().await;
    repo.record_profile(
        ProfileCategory::Command,
        "performance_diagnostics",
        5,
        &json!({"n": 1}),
    )
    .await
    .unwrap();
    repo.record_profile(ProfileCategory::Worker, "learning_worker", 90, &json!({}))
        .await
        .unwrap();

    let samples = repo.recent_profiles(10).await.unwrap();
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].name, "learning_worker"); // newest first
    assert_eq!(samples[0].category, ProfileCategory::Worker);
    assert_eq!(samples[1].name, "performance_diagnostics");
    assert_eq!(repo.profile_count().await.unwrap(), 2);
}

#[tokio::test]
async fn recent_profiles_respects_limit() {
    let (repo, _pool, _guard) = setup().await;
    for _ in 0..5 {
        repo.record_profile(ProfileCategory::Engine, "probe", 1, &json!({}))
            .await
            .unwrap();
    }
    assert_eq!(repo.recent_profiles(3).await.unwrap().len(), 3);
}

#[tokio::test]
async fn benchmarks_persist_and_round_trip() {
    let (repo, _pool, _guard) = setup().await;
    let benchmark = sample_benchmark();
    repo.record_benchmark("memory", &benchmark).await.unwrap();
    let recent = repo.recent_benchmarks(10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].name, "memory_search");
}

#[tokio::test]
async fn startup_profiles_group_by_run_and_sort_stages() {
    let (repo, _pool, _guard) = setup().await;
    let run = uuid::Uuid::new_v4();
    let stages = vec![
        StartupStage {
            name: "engines".into(),
            label: "Engine construction".into(),
            duration_ms: 20,
            started_at: chrono::Utc::now() + chrono::Duration::milliseconds(1),
        },
        StartupStage {
            name: "database".into(),
            label: "Database initialization".into(),
            duration_ms: 10,
            started_at: chrono::Utc::now(),
        },
    ];
    repo.record_startup_profile(run, &stages).await.unwrap();

    let runs = repo.recent_startup_profiles(5).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, run);
    // Stages are re-sorted by started_at.
    assert_eq!(runs[0].stages[0].name, "database");
    assert_eq!(runs[0].total_ms, 30);
}

#[tokio::test]
async fn multiple_startup_runs_group_independently() {
    let (repo, _pool, _guard) = setup().await;
    let (first, second) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    repo.record_startup_profile(
        first,
        &[StartupStage {
            name: "a".into(),
            label: "A".into(),
            duration_ms: 5,
            started_at: chrono::Utc::now(),
        }],
    )
    .await
    .unwrap();
    repo.record_startup_profile(
        second,
        &[StartupStage {
            name: "b".into(),
            label: "B".into(),
            duration_ms: 6,
            started_at: chrono::Utc::now(),
        }],
    )
    .await
    .unwrap();

    let runs = repo.recent_startup_profiles(5).await.unwrap();
    assert_eq!(runs.len(), 2);
}

#[tokio::test]
async fn db_size_is_positive_after_migration() {
    let (repo, _pool, _guard) = setup().await;
    let size = repo.db_size_bytes().await.unwrap();
    assert!(size > 0);
}

#[tokio::test]
async fn prune_removes_only_old_rows_and_reports_count() {
    let (repo, _pool, _guard) = setup().await;
    repo.record_profile(
        ProfileCategory::Command,
        "performance_profile",
        1,
        &json!({}),
    )
    .await
    .unwrap();
    let removed = repo.prune_profiles_older_than(0).await.unwrap();
    assert_eq!(removed, 1);
    assert_eq!(repo.profile_count().await.unwrap(), 0);
}

#[tokio::test]
async fn benchmark_decodes_ok_and_throughput() {
    let (repo, _pool, _guard) = setup().await;
    let benchmark = sample_benchmark();
    repo.record_benchmark("memory", &benchmark).await.unwrap();
    let recent = repo.recent_benchmarks(1).await.unwrap();
    assert!(recent[0].ok);
    assert_eq!(recent[0].iterations, 5);
    assert!(recent[0].throughput_per_sec.unwrap() > 0.0);
}
