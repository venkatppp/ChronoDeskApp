//! WatchdogService tests (RC-10 M2): the pure `evaluate` pass (stale vs
//! fresh heartbeats) and the repository-backed `scan` pass (miss
//! counters, status flips, journaled events).

use super::*;
use crate::database::test_database;
use crate::models::recovery::{JournalEntryType, WorkerStatus};
use crate::performance::recovery::journal::Journal;
use chrono::{DateTime, Duration, Utc};

async fn setup() -> (
    WatchdogService,
    RecoveryRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let repository = RecoveryRepository::new(pool.clone());
    let journal = Journal::new(repository.clone());
    (
        WatchdogService::new(repository.clone(), journal),
        repository,
        pool,
        temp_dir,
    )
}

fn worker(id: i64, name: &str, status: WorkerStatus, heartbeat: DateTime<Utc>) -> WorkerHealth {
    WorkerHealth {
        id,
        worker: name.to_string(),
        status,
        last_heartbeat: heartbeat,
        consecutive_misses: 0,
        execution_count: 0,
        error_count: 0,
        last_error: String::new(),
        details: serde_json::Value::Null,
        updated_at: heartbeat,
    }
}

#[tokio::test]
async fn fresh_heartbeats_produce_no_events() {
    let (service, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![
        worker(1, "a", WorkerStatus::Healthy, now),
        worker(2, "b", WorkerStatus::Idle, now),
    ];
    let events = service.evaluate(&workers, now, Duration::seconds(120));
    assert!(events.is_empty());
}

#[tokio::test]
async fn stale_heartbeat_marks_stalled() {
    let (service, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![worker(
        1,
        "a",
        WorkerStatus::Healthy,
        now - Duration::minutes(10),
    )];
    let events = service.evaluate(&workers, now, Duration::seconds(120));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].worker, "a");
    assert_eq!(events[0].kind, "stalled");
}

#[tokio::test]
async fn resumed_heartbeat_reports_recovered() {
    let (service, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![worker(
        1,
        "a",
        WorkerStatus::Stalled,
        now - Duration::seconds(30), // within grace
    )];
    let events = service.evaluate(&workers, now, Duration::seconds(120));
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "recovered");
}

#[tokio::test]
async fn already_stalled_worker_stays_quiet() {
    let (service, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let workers = vec![worker(
        1,
        "a",
        WorkerStatus::Stalled,
        now - Duration::hours(1),
    )];
    // No double-reporting for an already-stalled worker.
    assert!(service
        .evaluate(&workers, now, Duration::seconds(120))
        .is_empty());
}

#[tokio::test]
async fn scan_records_misses_and_journals_events() {
    let (service, repository, _pool, _guard) = setup().await;
    repository.register_worker("indexer").await.unwrap();

    // Backdate the heartbeat so the next scan sees a stale worker.
    sqlx::query("UPDATE worker_health SET last_heartbeat = ? WHERE worker = 'indexer'")
        .bind(Utc::now() - Duration::minutes(10))
        .execute(&_pool)
        .await
        .unwrap();

    let events = service.scan(Duration::seconds(120)).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "stalled");

    let row = repository.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Stalled);
    assert_eq!(row.consecutive_misses, 1);

    // A second pass increments the miss counter.
    let events = service.scan(Duration::seconds(120)).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        repository
            .worker_health("indexer")
            .await
            .unwrap()
            .unwrap()
            .consecutive_misses,
        2
    );

    // Journaled events are readable.
    let recent = repository.recent_journal(10).await.unwrap();
    let events: Vec<_> = recent
        .iter()
        .filter(|e| e.entry_type == JournalEntryType::SelfHealing)
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].entity, "indexer");
}

#[tokio::test]
async fn scan_recovers_worker_after_heartbeat_resumes() {
    let (service, repository, _pool, _guard) = setup().await;
    repository.register_worker("indexer").await.unwrap();
    sqlx::query("UPDATE worker_health SET last_heartbeat = ? WHERE worker = 'indexer'")
        .bind(Utc::now() - Duration::minutes(10))
        .execute(&_pool)
        .await
        .unwrap();
    service.scan(Duration::seconds(120)).await.unwrap();

    // The worker heartbeats again -> the next scan reports recovery.
    repository.heartbeat_worker("indexer").await.unwrap();
    let events = service.scan(Duration::seconds(120)).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "recovered");
    let row = repository.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Healthy);
    assert_eq!(row.consecutive_misses, 0);
}
