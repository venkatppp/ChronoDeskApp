//! HealthMonitor tests (RC-10 M2): the pure `assess` scoring and the
//! repository-backed `capture` (persists a journaled health snapshot).

use super::*;
use crate::database::test_database;
use crate::models::recovery::{HealthStatus, JournalEntryType, WorkerStatus};
use crate::performance::recovery::journal::Journal;
use chrono::{DateTime, Utc};

async fn setup() -> (
    HealthMonitor,
    RecoveryRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let repository = RecoveryRepository::new(pool.clone());
    let journal = Journal::new(repository.clone());
    (
        HealthMonitor::new(repository.clone(), journal),
        repository,
        pool,
        temp_dir,
    )
}

fn worker(id: i64, name: &str, status: WorkerStatus, at: DateTime<Utc>) -> WorkerHealth {
    WorkerHealth {
        id,
        worker: name.to_string(),
        status,
        last_heartbeat: at,
        consecutive_misses: if status == WorkerStatus::Stalled {
            1
        } else {
            0
        },
        execution_count: 0,
        error_count: 0,
        last_error: String::new(),
        details: serde_json::Value::Null,
        updated_at: at,
    }
}

#[tokio::test]
async fn assess_scores_all_healthy_at_100() {
    let (monitor, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![
        worker(1, "a", WorkerStatus::Healthy, now),
        worker(2, "b", WorkerStatus::Idle, now),
    ];
    let (status, score) = monitor.assess(&workers);
    assert_eq!(status, HealthStatus::Healthy);
    assert_eq!(score, 100.0);
}

#[tokio::test]
async fn assess_degrades_for_stalled_workers() {
    let (monitor, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![worker(1, "a", WorkerStatus::Stalled, now)];
    let (status, score) = monitor.assess(&workers);
    assert_eq!(status, HealthStatus::Degraded);
    assert_eq!(score, 75.0);
}

#[tokio::test]
async fn assess_turns_critical_for_failed_workers() {
    let (monitor, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![
        worker(1, "a", WorkerStatus::Failed, now),
        worker(2, "b", WorkerStatus::Failed, now),
    ];
    let (status, score) = monitor.assess(&workers);
    assert_eq!(status, HealthStatus::Critical);
    assert_eq!(score, 0.0);
}

#[tokio::test]
async fn assess_clamps_score_to_zero() {
    let (monitor, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![
        worker(1, "a", WorkerStatus::Failed, now),
        worker(2, "b", WorkerStatus::Failed, now),
        worker(3, "c", WorkerStatus::Stalled, now),
    ];
    let (_, score) = monitor.assess(&workers);
    assert_eq!(score, 0.0);
}

#[tokio::test]
async fn capture_persists_snapshot_and_issues() {
    let (monitor, repository, _pool, _guard) = setup().await;
    repository.register_worker("healthy-worker").await.unwrap();
    repository.register_worker("stale-worker").await.unwrap();
    sqlx::query("UPDATE worker_health SET status = 'stalled', consecutive_misses = 2 WHERE worker = 'stale-worker'")
        .execute(&_pool)
        .await
        .unwrap();

    let snapshot = monitor.capture(vec![]).await.unwrap();
    assert_eq!(snapshot.status, HealthStatus::Degraded);
    assert_eq!(snapshot.overall_score, 75.0);
    assert!(snapshot.issues.iter().any(|i| i.contains("stale-worker")));

    let history = repository.recent_health_snapshots(5).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].entry_type, JournalEntryType::Health);
    assert_eq!(history[0].state, "degraded");
}
