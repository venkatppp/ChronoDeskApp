//! RollbackService tests (RC-10 M2): rollback to the newest valid
//! ancestor, restored job lists, the no-ancestor failure path, and the
//! journaled rollback trail.

use super::*;
use crate::database::test_database;
use crate::models::recovery::{JournalEntryType, RecoveryJournalEntry};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

async fn setup() -> (
    RollbackService,
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
        RollbackService::new(repository.clone(), journal.clone()),
        journal,
        repository,
        pool,
        temp_dir,
    )
}

fn valid_entry(
    id: i64,
    state: &str,
    jobs: serde_json::Value,
    at: DateTime<Utc>,
) -> RecoveryJournalEntry {
    let payload = json!({ "active_jobs": jobs });
    RecoveryJournalEntry {
        id,
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
async fn rolls_back_to_newest_valid_ancestor() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let corrupt = valid_entry(3, "running", json!(["job-3"]), now);
    let valid = valid_entry(
        2,
        "running",
        json!(["job-1", "job-2"]),
        now - Duration::minutes(1),
    );
    let oldest = valid_entry(1, "running", json!(["job-0"]), now - Duration::minutes(2));

    let result = service
        .rollback(&corrupt, &[corrupt.clone(), valid.clone(), oldest])
        .await
        .unwrap();
    assert!(result.ok);
    assert_eq!(result.rolled_back_to, Some(2));
    assert_eq!(result.restored, vec!["job-1", "job-2"]);
}

#[tokio::test]
async fn no_valid_ancestor_fails_cleanly() {
    let (service, _journal, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let corrupt = valid_entry(3, "running", json!([]), now);
    let mut corrupt2 = valid_entry(2, "running", json!([]), now - Duration::minutes(1));
    corrupt2.checksum = "tampered".to_string();

    let result = service
        .rollback(&corrupt, &[corrupt.clone(), corrupt2])
        .await
        .unwrap();
    assert!(!result.ok);
    assert_eq!(result.rolled_back_to, None);
    assert!(result.restored.is_empty());
}

#[tokio::test]
async fn rollback_is_journaled() {
    let (service, journal, _repo, _pool, _guard) = setup().await;
    let now = Utc::now();
    let corrupt = valid_entry(3, "running", json!([]), now);
    let valid = valid_entry(2, "running", json!(["job-1"]), now - Duration::minutes(1));

    service
        .rollback(&corrupt, &[corrupt.clone(), valid])
        .await
        .unwrap();

    let recent = journal.recent(10).await.unwrap();
    assert!(recent
        .iter()
        .any(|e| e.entry_type == JournalEntryType::Rollback));
    let rollback = recent
        .iter()
        .find(|e| e.entry_type == JournalEntryType::Rollback)
        .unwrap();
    assert_eq!(rollback.payload["from_checkpoint"], json!(3));
    assert_eq!(rollback.payload["to_checkpoint"], json!(2));
}
