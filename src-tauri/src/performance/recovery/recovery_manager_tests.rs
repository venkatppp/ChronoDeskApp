//! RecoveryManager tests (RC-10 M2): the facade end-to-end — startup
//! pass with first run and crash round trips across fresh managers, clean
//! shutdown recording, health status, history aggregation, watchdog ticks
//! and manual rollback.

use super::*;
use crate::database::test_database;
use crate::models::recovery::RecoveryOutcome;
use serde_json::json;

async fn setup() -> (RecoveryManager, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (
        RecoveryManager::new(RecoveryRepository::new(pool.clone())),
        pool,
        temp_dir,
    )
}

#[tokio::test]
async fn startup_first_run_opens_session() {
    let (manager, _pool, _guard) = setup().await;
    let run = manager.startup().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::NoAction);
    assert_eq!(run.actions, vec!["checkpoint"]);

    // The runtime liveness worker is registered.
    let row = manager
        .repository
        .worker_health("runtime")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, crate::models::recovery::WorkerStatus::Healthy);

    let latest = manager.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "running");
}

#[tokio::test]
async fn clean_shutdown_is_not_a_crash() {
    let (manager, _pool, _guard) = setup().await;
    manager.startup().await.unwrap();
    manager.record_clean_shutdown().await.unwrap();

    // If a running app restarts, usually both use the same db. We verify
    // the *type-level* fact here: a clean checkpoint was written and no
    // crash is flagged on the next pass over the same data.
    let latest = manager.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "clean");
    assert!(manager.crash_reports(5).await.unwrap().is_empty());
}

#[tokio::test]
async fn interrupted_session_is_recovered_on_restart() {
    let (manager, pool, _guard) = setup().await;
    manager.startup().await.unwrap(); // writes a `running` checkpoint
    manager
        .checkpoint(&["job-1".to_string(), "job-2".to_string()], json!({}))
        .await
        .unwrap();

    // Same database, fresh manager (a new process would do exactly this).
    let repo = RecoveryRepository::new(pool.clone());
    let fresh = RecoveryManager::new(repo);
    let run = fresh.startup().await.unwrap();

    assert_eq!(run.outcome, RecoveryOutcome::Recovered);
    assert_eq!(run.recovered_jobs, vec!["job-1", "job-2"]);
    assert_eq!(fresh.history(5).await.unwrap().runs.len(), 2);
}

#[tokio::test]
async fn status_and_history_aggregate() {
    let (manager, _pool, _guard) = setup().await;
    manager.startup().await.unwrap();

    let status = manager.status().await.unwrap();
    assert_eq!(
        status.status,
        crate::models::recovery::HealthStatus::Healthy
    );

    let history = manager.history(10).await.unwrap();
    assert_eq!(history.runs.len(), 1);
    assert_eq!(history.crashes.len(), 0);
    assert!(!history.journal.is_empty());
}

#[tokio::test]
async fn watchdog_tick_heartbeats_runtime_and_captures_health() {
    let (manager, _pool, _guard) = setup().await;
    manager.startup().await.unwrap();

    let events = manager.watchdog_tick().await.unwrap();
    assert!(events.is_empty(), "a fresh runtime marker must not stall");

    let row = manager
        .repository
        .worker_health("runtime")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.consecutive_misses, 0);
    assert!(manager.tick_count() >= 1);

    // A health snapshot was persisted on this pass.
    let snapshots = manager.repository.recent_health_snapshots(5).await.unwrap();
    assert_eq!(snapshots.len(), 1);
}

#[tokio::test]
async fn manual_rollback_requires_an_ancestor() {
    let (manager, _pool, _guard) = setup().await;
    manager.startup().await.unwrap();
    let result = manager.rollback_now().await.unwrap();
    // Only one checkpoint exists; nothing to roll back to.
    assert!(!result.ok);
    assert_eq!(result.rolled_back_to, None);
}

#[tokio::test]
async fn run_self_healing_on_clean_state_is_a_noop() {
    let (manager, _pool, _guard) = setup().await;
    manager.startup().await.unwrap();
    let report = manager.run_self_healing().await.unwrap();
    assert!(report.executed.is_empty());
    assert!(report.failed.is_empty());
}
