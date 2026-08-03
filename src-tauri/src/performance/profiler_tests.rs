//! PerformanceProfiler tests (RC-10 M1): the live ring aggregates,
//! p95 latency, ordering of recent/slowest views, persistence to the
//! `performance_profiles` ledger, and pruning.

use super::*;
use crate::database::test_database;
use serde_json::json;

async fn setup() -> (PerformanceProfiler, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (
        PerformanceProfiler::new(PerformanceRepository::new(pool.clone())),
        pool,
        temp_dir,
    )
}

#[tokio::test]
async fn record_persists_and_count_tracks() {
    let (profiler, _pool, _guard) = setup().await;
    profiler
        .record(
            ProfileCategory::Command,
            "performance_profile",
            12,
            json!({"source": "test"}),
        )
        .await
        .unwrap();
    profiler
        .record(ProfileCategory::Repository, "db_size_bytes", 3, Value::Null)
        .await
        .unwrap();
    assert_eq!(profiler.persisted_count().await.unwrap(), 2);
}

#[tokio::test]
async fn snapshot_aggegates_durations_and_reports_p95() {
    let (profiler, _pool, _guard) = setup().await;
    // 100 samples of the same operation: 90 of 1ms + 10 of 100ms, so the
    // 95th percentile lands on the slow cluster.
    for i in 0..100 {
        let ms = if i < 10 { 100 } else { 1 };
        profiler
            .record(
                ProfileCategory::Service,
                "workspace_service.list",
                ms,
                Value::Null,
            )
            .await
            .unwrap();
    }
    let snapshot = profiler.snapshot().await.unwrap();
    let agg = snapshot
        .aggregates
        .iter()
        .find(|a| a.name == "workspace_service.list")
        .expect("aggregate present");
    assert_eq!(agg.count, 100);
    assert_eq!(agg.min_ms, 1);
    assert_eq!(agg.max_ms, 100);
    assert!(agg.avg_ms > 10.0 && agg.avg_ms < 11.0);
    assert_eq!(agg.p95_ms, 100.0);
}

#[tokio::test]
async fn snapshot_recent_is_newest_first_and_slowest_sorted_desc() {
    let (profiler, _pool, _guard) = setup().await;
    profiler
        .record(ProfileCategory::Worker, "learning_worker", 5, Value::Null)
        .await
        .unwrap();
    profiler
        .record(ProfileCategory::Worker, "learning_worker", 500, Value::Null)
        .await
        .unwrap();
    profiler
        .record(ProfileCategory::Worker, "learning_worker", 9, Value::Null)
        .await
        .unwrap();

    let snapshot = profiler.snapshot().await.unwrap();
    assert_eq!(snapshot.slowest.len(), 3);
    assert_eq!(snapshot.slowest[0].duration_ms, 500);
    assert!(
        snapshot.recent[0].occurred_at >= snapshot.recent[snapshot.recent.len() - 1].occurred_at
    );
}

#[tokio::test]
async fn time_measures_and_records_a_sync_closure() {
    let (profiler, _pool, _guard) = setup().await;
    let value = profiler.time(ProfileCategory::Engine, "probe", || 42).await;
    assert_eq!(value, 42);
    let snapshot = profiler.snapshot().await.unwrap();
    assert!(snapshot
        .aggregates
        .iter()
        .any(|a| a.name == "probe" && a.category == ProfileCategory::Engine));
}

#[tokio::test]
async fn prune_older_than_removes_old_ledger_rows() {
    let (profiler, _pool, _guard) = setup().await;
    profiler
        .record(
            ProfileCategory::Command,
            "performance_profile",
            2,
            Value::Null,
        )
        .await
        .unwrap();
    // A negative-day window removes everything older than now.
    let removed = profiler.prune_older_than(0).await.unwrap();
    assert_eq!(removed, 1);
    assert_eq!(profiler.persisted_count().await.unwrap(), 0);
}
