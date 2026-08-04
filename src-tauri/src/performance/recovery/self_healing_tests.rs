//! SelfHealingService tests (RC-10 M2): the pure `plan` rules and the
//! repository-backed `run` (worker restarts, checkpoint verification with
//! rollback on corrupt state, bounded journal pruning).

use super::*;
use crate::database::test_database;
use crate::models::recovery::{
    HealthSnapshot, HealthStatus, JournalEntryType, WorkerHealth, WorkerStatus,
};
use crate::performance::recovery::journal::Journal;
use chrono::{DateTime, Utc};
use serde_json::json;

async fn setup() -> (
    SelfHealingService,
    Journal,
    RecoveryRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let repository = RecoveryRepository::new(pool.clone());
    let journal = Journal::new(repository.clone());
    (
        SelfHealingService::new(repository.clone(), journal.clone()),
        journal,
        repository,
        pool,
        temp_dir,
    )
}

fn worker(
    id: i64,
    name: &str,
    status: WorkerStatus,
    misses: u64,
    at: DateTime<Utc>,
) -> WorkerHealth {
    WorkerHealth {
        id,
        worker: name.to_string(),
        status,
        last_heartbeat: at,
        consecutive_misses: misses,
        execution_count: 0,
        error_count: 0,
        last_error: String::new(),
        details: serde_json::Value::Null,
        updated_at: at,
    }
}

fn snapshot(
    workers: Vec<WorkerHealth>,
    status: HealthStatus,
    issues: Vec<String>,
) -> HealthSnapshot {
    HealthSnapshot {
        captured_at: Utc::now(),
        status,
        overall_score: 100.0,
        workers,
        issues,
        details: serde_json::Value::Null,
    }
}

#[tokio::test]
async fn plan_is_empty_when_healthy() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let healthy = snapshot(
        vec![worker(1, "a", WorkerStatus::Healthy, 0, Utc::now())],
        HealthStatus::Healthy,
        vec![],
    );
    assert!(service.plan(&healthy).is_empty());
}

#[tokio::test]
async fn plan_restarts_failed_workers_even_with_few_misses() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let failing = snapshot(
        vec![worker(1, "a", WorkerStatus::Failed, 0, Utc::now())],
        HealthStatus::Critical,
        vec!["worker 'a' has failed".to_string()],
    );
    let plan = service.plan(&failing);
    assert!(plan.contains(&"restart_worker:a".to_string()));
    assert!(plan.contains(&"verify_checkpoint".to_string()));
}

#[tokio::test]
async fn plan_waits_for_miss_threshold_before_restart() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let barely_stalled = snapshot(
        vec![worker(1, "a", WorkerStatus::Stalled, 1, Utc::now())],
        HealthStatus::Degraded,
        vec!["worker 'a' is stalled".to_string()],
    );
    let plan = service.plan(&barely_stalled);
    assert!(!plan.contains(&"restart_worker:a".to_string()));
    // The degraded status still asks for a checkpoint verification.
    assert!(plan.contains(&"verify_checkpoint".to_string()));

    let long_stalled = snapshot(
        vec![worker(1, "a", WorkerStatus::Stalled, 3, Utc::now())],
        HealthStatus::Degraded,
        vec!["worker 'a' is stalled".to_string()],
    );
    assert!(service
        .plan(&long_stalled)
        .contains(&"restart_worker:a".to_string()));
}

#[tokio::test]
async fn run_restarts_stalled_workers() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    repository.register_worker("indexer").await.unwrap();
    repository.record_worker_miss("indexer").await.unwrap();
    repository.record_worker_miss("indexer").await.unwrap();
    repository.record_worker_miss("indexer").await.unwrap();

    let workers = repository.all_worker_health().await.unwrap();
    let degraded = snapshot(
        workers,
        HealthStatus::Degraded,
        vec!["worker 'indexer' is stalled".to_string()],
    );
    let report = service.run(&degraded).await.unwrap();

    assert!(report
        .executed
        .contains(&"restart_worker:indexer".to_string()));
    assert_eq!(report.healed_workers, vec!["indexer"]);

    let row = repository.worker_health("indexer").await.unwrap().unwrap();
    assert_eq!(row.status, WorkerStatus::Healthy);
    assert_eq!(row.consecutive_misses, 0);

    // The restart is journaled.
    let recent = repository.recent_journal(10).await.unwrap();
    assert!(recent
        .iter()
        .any(|e| e.entry_type == JournalEntryType::SelfHealing && e.entity == "indexer"));
}

#[tokio::test]
async fn run_verifies_valid_checkpoint_without_side_effects() {
    let (service, journal, repository, _pool, _guard) = setup().await;
    journal
        .checkpoint("startup", "running", &[], json!({}))
        .await
        .unwrap();

    let critical = snapshot(
        vec![],
        HealthStatus::Critical,
        vec!["something".to_string()],
    );
    let report = service.run(&critical).await.unwrap();
    assert!(report.executed.contains(&"verify_checkpoint".to_string()));

    // No rollback happened (the checkpoint was valid).
    let recent = repository.recent_journal(10).await.unwrap();
    assert!(!recent
        .iter()
        .any(|e| e.entry_type == JournalEntryType::Rollback));
}

#[tokio::test]
async fn run_rolls_back_corrupt_checkpoint() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    let valid_id = repository
        .append_journal_entry(
            JournalEntryType::Checkpoint,
            "startup",
            "app",
            "running",
            &json!({ "active_jobs": ["base-job"] }),
            &Journal::checksum("app", "running", &json!({ "active_jobs": ["base-job"] })),
        )
        .await
        .unwrap();
    repository
        .append_journal_entry(
            JournalEntryType::Checkpoint,
            "startup",
            "app",
            "running",
            &json!({ "active_jobs": ["half-written"] }),
            "tampered",
        )
        .await
        .unwrap();

    let critical = snapshot(
        vec![],
        HealthStatus::Critical,
        vec!["corrupt state".to_string()],
    );
    let report = service.run(&critical).await.unwrap();
    assert!(report.executed.contains(&"verify_checkpoint".to_string()));

    // The rollback trail points at the valid ancestor.
    let recent = repository.recent_journal(10).await.unwrap();
    let rollback = recent
        .iter()
        .find(|e| e.entry_type == JournalEntryType::Rollback)
        .expect("a rollback should have been journaled");
    assert_eq!(rollback.payload["to_checkpoint"], json!(valid_id));
}

#[tokio::test]
async fn run_prunes_journal_past_threshold() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    let threshold = crate::performance::recovery::JOURNAL_PRUNE_THRESHOLD;
    // Fill just past the threshold with lightweight heartbeat entries.
    for i in 0..(threshold + 5) {
        repository
            .append_journal_entry(
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

    let report = service
        .run(&snapshot(vec![], HealthStatus::Healthy, vec![]))
        .await
        .unwrap();
    assert!(report.executed.contains(&"prune_history".to_string()));
    assert_eq!(repository.journal_count().await.unwrap(), threshold);
}
