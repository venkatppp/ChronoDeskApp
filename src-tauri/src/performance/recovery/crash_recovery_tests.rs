//! CrashRecoveryService tests (RC-10 M2): the pure detection rule
//! (first-run / clean / crash, with recent-vs-timeout classification) and
//! the full startup recovery flow — resume, corrupt-checkpoint rollback,
//! the no-ancestor failure path, and the crash-report audit trail.

use super::*;
use crate::database::test_database;
use crate::models::recovery::{CrashType, JournalEntryType, RecoveryOutcome};
use crate::performance::recovery::journal::Journal;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

async fn setup() -> (
    CrashRecoveryService,
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
        CrashRecoveryService::new(repository.clone(), journal.clone()),
        journal,
        repository,
        pool,
        temp_dir,
    )
}

fn checkpoint(at: DateTime<Utc>, state: &str, payload: serde_json::Value) -> RecoveryJournalEntry {
    RecoveryJournalEntry {
        id: 0,
        entry_type: JournalEntryType::Checkpoint,
        scope: "test".to_string(),
        entity: "app".to_string(),
        state: state.to_string(),
        checksum: Journal::checksum("app", state, &payload),
        payload,
        created_at: at,
    }
}

#[tokio::test]
async fn detect_crash_is_rule_correct() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();

    // First run: no checkpoint at all -> no crash.
    assert_eq!(
        service.detect_crash(None, now, Duration::seconds(120)),
        None
    );

    // A clean shutdown checkpoint -> no crash, however old.
    let clean = checkpoint(now, "clean", json!({ "active_jobs": [] }));
    assert_eq!(
        service.detect_crash(
            Some(&clean),
            now + Duration::hours(5),
            Duration::seconds(120)
        ),
        None
    );

    // A recent running checkpoint -> crash (unk Kill-classified).
    let fresh = checkpoint(now, "running", json!({ "active_jobs": [] }));
    assert_eq!(
        service.detect_crash(Some(&fresh), now, Duration::seconds(120)),
        Some(CrashType::Unknown)
    );

    // A stale running checkpoint -> crash classified as a timeout.
    let stale = checkpoint(
        now - Duration::minutes(10),
        "running",
        json!({ "active_jobs": [] }),
    );
    assert_eq!(
        service.detect_crash(Some(&stale), now, Duration::seconds(120)),
        Some(CrashType::Timeout)
    );
}

#[tokio::test]
async fn first_run_records_no_action_run_and_checkpoint() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    let run = service.detect_and_recover().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::NoAction);
    assert_eq!(run.status, "success");

    let latest = repository.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "running");

    // The audit run is persisted; no crash was filed (first run).
    let runs = repository.recent_recovery_runs(5).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].outcome, RecoveryOutcome::NoAction);
    assert!(repository.recent_crash_reports(5).await.unwrap().is_empty());

    // The checkpoint is the new no-action session (no jobs in flight).
    assert_eq!(latest.payload["active_jobs"], json!([]));
}

#[tokio::test]
async fn clean_shutdown_yields_no_crash() {
    let (service, journal, repository, _pool, _guard) = setup().await;
    service.detect_and_recover().await.unwrap();
    journal
        .checkpoint("runtime", "clean", &[], json!({}))
        .await
        .unwrap();

    let run = service.detect_and_recover().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::NoAction);
    assert!(repository.recent_crash_reports(5).await.unwrap().is_empty());
}

#[tokio::test]
async fn interrupted_session_is_detected_and_resumed() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    // A previous session left a running checkpoint with in-flight jobs.
    service
        .journal
        .checkpoint(
            "runtime",
            "running",
            &["job-1".to_string(), "job-2".to_string()],
            json!({}),
        )
        .await
        .unwrap();

    let run = service.detect_and_recover().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::Recovered);
    assert_eq!(run.recovered_jobs, vec!["job-1", "job-2"]);
    assert_eq!(run.actions, vec!["revalidate", "resume"]);

    // A crash report was filed and handled (was_recovered).
    let reports = repository.recent_crash_reports(5).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].was_recovered);
    assert_eq!(reports[0].component, "runtime");

    // The new session's checkpoint carries the resumed jobs.
    let latest = repository.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "running");
    assert_eq!(latest.payload["active_jobs"], json!(["job-1", "job-2"]));

    let runs = repository.recent_recovery_runs(5).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].outcome, RecoveryOutcome::Recovered);
}

#[tokio::test]
async fn corrupt_checkpoint_rolls_back_to_valid_ancestor() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    // A valid checkpoint exists, then a *corrupt* one (bad checksum) is
    // appended — exactly what a half-written row looks like after a crash.
    let valid_id = journal_checkpoint(&service, &["base-job".to_string()]).await;
    let corrupt_id = repository
        .append_journal_entry(
            JournalEntryType::Checkpoint,
            "runtime",
            "app",
            "running",
            &json!({ "active_jobs": ["half-written-job"] }),
            "tampered-checksum",
        )
        .await
        .unwrap();
    assert_ne!(valid_id, corrupt_id);

    let run = service.detect_and_recover().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::RolledBack);
    assert_eq!(run.rolled_back_to, Some(valid_id));
    assert_eq!(run.recovered_jobs, vec!["base-job"]);
    assert_eq!(run.actions, vec!["revalidate", "rollback"]);

    // Recovery handled the crash report.
    assert!(repository.recent_crash_reports(5).await.unwrap()[0].was_recovered);
}

#[tokio::test]
async fn corrupt_checkpoint_without_ancestor_fails_openly() {
    let (service, _journal, repository, _pool, _guard) = setup().await;
    // Only a corrupt checkpoint exists — nothing valid to roll back to.
    repository
        .append_journal_entry(
            JournalEntryType::Checkpoint,
            "startup",
            "app",
            "running",
            &json!({ "active_jobs": [] }),
            "tampered",
        )
        .await
        .unwrap();

    let run = service.detect_and_recover().await.unwrap();
    assert_eq!(run.outcome, RecoveryOutcome::Failed);
    assert_eq!(run.rolled_back_to, None);
    assert_eq!(run.status, "failed");
    assert_eq!(run.errors.len(), 1);

    // The crash report stays open (recovery did not succeed).
    let reports = repository.recent_crash_reports(5).await.unwrap();
    assert!(!reports[0].was_recovered);
}

async fn journal_checkpoint(service: &CrashRecoveryService, jobs: &[String]) -> i64 {
    service
        .journal
        .checkpoint("startup", "running", jobs, json!({}))
        .await
        .unwrap()
}
