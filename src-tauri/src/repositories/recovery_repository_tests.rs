//! RecoveryRepository tests (RC-10 M2): journal append/read/prune,
//! checkpoint persistence, crash logging, worker health upserts, recovery
//! history, and health-history reads.

use super::*;
use crate::database::test_database;
use crate::models::recovery::{
    CrashType, JournalEntryType, RecoveryOutcome, RecoveryRun, RecoveryTrigger, WorkerStatus,
};
use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

async fn setup() -> (RecoveryRepository, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (RecoveryRepository::new(pool.clone()), pool, temp_dir)
}

#[tokio::test]
async fn journal_entries_round_trip() {
    let (repo, _pool, _guard) = setup().await;
    let id = repo
        .append_journal_entry(
            JournalEntryType::Checkpoint,
            "startup",
            "app",
            "running",
            &json!({"active_jobs": []}),
            "abc",
        )
        .await
        .unwrap();
    let recent = repo.recent_journal(10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, id);
    assert_eq!(recent[0].entry_type, JournalEntryType::Checkpoint);
    assert_eq!(recent[0].state, "running");
    assert_eq!(recent[0].checksum, "abc");
    assert_eq!(recent[0].payload["active_jobs"], json!([]));
}

#[tokio::test]
async fn journal_entries_respect_limit_and_entity_filter() {
    let (repo, _pool, _guard) = setup().await;
    for i in 0..3 {
        repo.append_journal_entry(
            JournalEntryType::Heartbeat,
            "watchdog",
            "w1",
            "alive",
            &json!({"i": i}),
            "",
        )
        .await
        .unwrap();
    }
    repo.append_journal_entry(
        JournalEntryType::Heartbeat,
        "watchdog",
        "w2",
        "alive",
        &json!({}),
        "",
    )
    .await
    .unwrap();
    assert_eq!(repo.recent_journal(2).await.unwrap().len(), 2);
    let for_w1 = repo.journal_for_entity("w1", 10).await.unwrap();
    assert_eq!(for_w1.len(), 3);
}

#[tokio::test]
async fn checkpoints_persist_and_latest_wins() {
    let (repo, _pool, _guard) = setup().await;
    repo.append_journal_entry(
        JournalEntryType::Checkpoint,
        "startup",
        "app",
        "running",
        &json!({"active_jobs": ["a"]}),
        "s1",
    )
    .await
    .unwrap();
    repo.append_journal_entry(
        JournalEntryType::Checkpoint,
        "runtime",
        "app",
        "clean",
        &json!({"active_jobs": []}),
        "s2",
    )
    .await
    .unwrap();

    let latest = repo.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "clean");
    assert_eq!(latest.checksum, "s2");

    let recent = repo.recent_checkpoints(5).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].id, latest.id); // newest first
}

#[tokio::test]
async fn crash_reports_log_and_recover() {
    let (repo, _pool, _guard) = setup().await;
    let id = repo
        .report_crash(
            "runtime",
            CrashType::Timeout,
            "error",
            "crashed",
            "trace",
            &json!({"n": 1}),
        )
        .await
        .unwrap();
    let reports = repo.recent_crash_reports(10).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].id, id);
    assert_eq!(reports[0].crash_type, CrashType::Timeout);
    assert!(!reports[0].was_recovered);
    assert_eq!(reports[0].metadata["n"], json!(1));

    repo.mark_crash_recovered(id).await.unwrap();
    let reports = repo.recent_crash_reports(10).await.unwrap();
    assert!(reports[0].was_recovered);
    assert!(reports[0].recovered_at.is_some());
}

#[tokio::test]
async fn crash_reports_prune_before_cutoff() {
    let (repo, pool, _guard) = setup().await;
    let id = repo
        .report_crash("runtime", CrashType::Unknown, "error", "x", "", &json!({}))
        .await
        .unwrap();
    // Backdate the row so it lands before the cutoff.
    sqlx::query("UPDATE crash_reports SET reported_at = ? WHERE id = ?")
        .bind(Utc::now() - Duration::days(2))
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let since = Utc::now() - Duration::days(1);
    let removed = repo.prune_crash_reports_before(since).await.unwrap();
    assert_eq!(removed, 1);
    assert_eq!(repo.crash_report_count().await.unwrap(), 0);
}

#[tokio::test]
async fn worker_health_register_heartbeat_and_misses() {
    let (repo, _pool, _guard) = setup().await;
    repo.register_worker("indexer").await.unwrap();
    let row = repo.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Healthy);

    repo.record_worker_miss("indexer").await.unwrap();
    repo.record_worker_miss("indexer").await.unwrap();
    let row = repo.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Stalled);
    assert_eq!(row.consecutive_misses, 2);

    repo.heartbeat_worker("indexer").await.unwrap();
    let row = repo.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Healthy);
    assert_eq!(row.consecutive_misses, 0);
}

#[tokio::test]
async fn worker_health_restart_and_fail_and_prune() {
    let (repo, _pool, _guard) = setup().await;
    repo.register_worker("w").await.unwrap();
    repo.mark_worker_failed("w", "boom").await.unwrap();
    let row = repo.worker_health("w").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Failed);
    assert_eq!(row.last_error, "boom");

    repo.mark_worker_healthy("w").await.unwrap();
    let row = repo.worker_health("w").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Healthy);
    assert_eq!(row.last_error, "");

    let removed = repo
        .prune_workers_inactive_since(Utc::now() + Duration::seconds(1))
        .await
        .unwrap();
    assert_eq!(removed, 1);
}

#[tokio::test]
async fn worker_register_is_idempotent_upsert() {
    let (repo, _pool, _guard) = setup().await;
    let first = repo.register_worker("w").await.unwrap();
    let second = repo.register_worker("w").await.unwrap();
    assert_eq!(first, second);
    assert_eq!(repo.all_worker_health().await.unwrap().len(), 1);
}

#[tokio::test]
async fn recovery_runs_round_trip() {
    let (repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let run = RecoveryRun {
        id: 0,
        run_id: Uuid::new_v4(),
        trigger: RecoveryTrigger::Startup,
        outcome: RecoveryOutcome::Recovered,
        status: "success".to_string(),
        actions: vec!["revalidate".to_string(), "resume".to_string()],
        recovered_jobs: vec!["job-1".to_string()],
        rolled_back_to: None,
        errors: vec![],
        duration_ms: 12,
        started_at: now,
        completed_at: now,
    };
    let id = repo.record_recovery_run(&run).await.unwrap();
    let runs = repo.recent_recovery_runs(10).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, id);
    assert_eq!(runs[0].run_id, run.run_id);
    assert_eq!(runs[0].outcome, RecoveryOutcome::Recovered);
    assert_eq!(runs[0].actions, run.actions);
    assert_eq!(runs[0].recovered_jobs, run.recovered_jobs);
}

#[tokio::test]
async fn health_snapshots_read_back_from_journal() {
    let (repo, _pool, _guard) = setup().await;
    repo.append_journal_entry(
        JournalEntryType::Health,
        "health",
        "app",
        "healthy",
        &json!({"score": 100}),
        "h",
    )
    .await
    .unwrap();
    let snapshots = repo.recent_health_snapshots(5).await.unwrap();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].state, "healthy");
    assert_eq!(snapshots[0].payload["score"], json!(100));
}

#[tokio::test]
async fn journal_prune_excess_keeps_newest() {
    let (repo, _pool, _guard) = setup().await;
    for i in 0..5 {
        repo.append_journal_entry(
            JournalEntryType::Heartbeat,
            "watchdog",
            "app",
            "alive",
            &json!({"i": i}),
            "",
        )
        .await
        .unwrap();
    }
    let removed = repo.prune_journal_excess(2).await.unwrap();
    assert_eq!(removed, 3);
    assert_eq!(repo.journal_count().await.unwrap(), 2);
}
