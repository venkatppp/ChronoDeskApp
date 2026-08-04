//! Journal tests (RC-10 M2): append/read round trips, checksum
//! determinism, checkpoint payload formatting, and latest-checkpoint
//! selection.

use super::*;
use crate::database::test_database;
use crate::models::recovery::JournalEntryType;
use serde_json::json;

async fn setup() -> (Journal, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (
        Journal::new(RecoveryRepository::new(pool.clone())),
        pool,
        temp_dir,
    )
}

#[tokio::test]
async fn append_and_recent_round_trip() {
    let (journal, _pool, _guard) = setup().await;
    let id = journal
        .append(
            JournalEntryType::Crash,
            "startup",
            "app",
            "detected",
            &json!({"type": "timeout"}),
        )
        .await
        .unwrap();
    let recent = journal.recent(10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id, id);
    assert_eq!(recent[0].entry_type, JournalEntryType::Crash);
    assert_eq!(recent[0].payload["type"], json!("timeout"));
}

#[tokio::test]
async fn checksum_is_deterministic_and_input_sensitive() {
    let payload = json!({ "active_jobs": ["a", "b"] });
    let first = Journal::checksum("app", "running", &payload);
    let second = Journal::checksum("app", "running", &payload);
    assert_eq!(first, second);

    let different_state = Journal::checksum("app", "clean", &payload);
    let different_payload = Journal::checksum("app", "running", &json!({ "active_jobs": ["a"] }));
    assert_ne!(first, different_state);
    assert_ne!(first, different_payload);
}

#[tokio::test]
async fn checkpoint_writes_payload_with_active_jobs() {
    let (journal, _pool, _guard) = setup().await;
    let id = journal
        .checkpoint(
            "runtime",
            "running",
            &["job-1".to_string(), "job-2".to_string()],
            json!({"phase": 2}),
        )
        .await
        .unwrap();
    let latest = journal.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.id, id);
    assert_eq!(latest.state, "running");
    assert_eq!(latest.payload["active_jobs"], json!(["job-1", "job-2"]));
    assert_eq!(latest.payload["metadata"]["phase"], json!(2));
}

#[tokio::test]
async fn latest_checkpoint_is_newest_only() {
    let (journal, _pool, _guard) = setup().await;
    assert!(journal.latest_checkpoint().await.unwrap().is_none());
    journal
        .checkpoint("startup", "running", &[], json!({}))
        .await
        .unwrap();
    journal
        .checkpoint("runtime", "clean", &[], json!({}))
        .await
        .unwrap();
    let latest = journal.latest_checkpoint().await.unwrap().unwrap();
    assert_eq!(latest.state, "clean");
    assert_eq!(journal.count().await.unwrap(), 2);
}
