//! CheckpointValidator tests (RC-10 M2): the pure trust rules — checksum
//! match, state presence, payload shape, entry type, timestamp ordering —
//! and the newest-valid-ancestor selection used by rollback.

use super::*;
use crate::performance::recovery::journal::Journal;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

use crate::models::recovery::JournalEntryType;

fn valid_entry(
    id: i64,
    state: &str,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
) -> RecoveryJournalEntry {
    RecoveryJournalEntry {
        id,
        entry_type: JournalEntryType::Checkpoint,
        scope: "startup".to_string(),
        entity: "app".to_string(),
        state: state.to_string(),
        checksum: Journal::checksum("app", state, &payload),
        payload,
        created_at,
    }
}

fn entry(id: i64, created_at: DateTime<Utc>) -> RecoveryJournalEntry {
    valid_entry(
        id,
        "running",
        json!({ "active_jobs": ["job-1"] }),
        created_at,
    )
}

#[test]
fn valid_checkpoint_passes() {
    let validator = CheckpointValidator;
    let result = validator.validate(&entry(1, Utc::now()), None);
    assert!(result.valid);
    assert!(result.issues.is_empty());
    assert_eq!(result.entry_id, Some(1));
}

#[test]
fn checksum_mismatch_fails() {
    let validator = CheckpointValidator;
    let mut corrupt = entry(1, Utc::now());
    corrupt.checksum = "deadbeef".to_string();
    let result = validator.validate(&corrupt, None);
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("checksum")));
}

#[test]
fn empty_state_fails() {
    let validator = CheckpointValidator;
    let corrupt = valid_entry(1, "", json!({ "active_jobs": [] }), Utc::now());
    let result = validator.validate(&corrupt, None);
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("state")));
}

#[test]
fn missing_active_jobs_fails() {
    let validator = CheckpointValidator;
    let corrupt = valid_entry(1, "running", json!({ "jobs": [] }), Utc::now());
    let result = validator.validate(&corrupt, None);
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("active_jobs")));
}

#[test]
fn non_object_payload_fails() {
    let validator = CheckpointValidator;
    let corrupt = valid_entry(1, "running", serde_json::Value::Null, Utc::now());
    let result = validator.validate(&corrupt, None);
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("payload")));
}

#[test]
fn wrong_entry_type_fails() {
    let validator = CheckpointValidator;
    let mut wrong = entry(1, Utc::now());
    wrong.entry_type = JournalEntryType::Heartbeat;
    wrong.checksum = Journal::checksum("app", "running", &wrong.payload);
    let result = validator.validate(&wrong, None);
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("not a checkpoint")));
}

#[test]
fn out_of_order_timestamps_fail() {
    let validator = CheckpointValidator;
    let previous = entry(2, Utc::now());
    let later = entry(1, Utc::now() - Duration::minutes(5));
    let result = validator.validate(&later, Some(&previous));
    assert!(!result.valid);
    assert!(result.issues.iter().any(|i| i.contains("precedes")));
}

#[test]
fn newest_valid_skips_excluded_and_invalid() {
    let validator = CheckpointValidator;
    let newest = entry(3, Utc::now());
    let older = entry(2, Utc::now() - Duration::minutes(1));
    let oldest = entry(1, Utc::now() - Duration::minutes(2));

    // Newest-first candidates; the newest is corrupt, the next valid.
    let mut corrupt = newest.clone();
    corrupt.checksum = "bad".to_string();
    let candidates = vec![corrupt.clone(), older.clone(), oldest.clone()];

    let target = validator.newest_valid(&candidates, Some(corrupt.id), None);
    assert_eq!(target.unwrap().id, older.id);

    // Excluding the corrupt id is what skips it; a valid newest is picked
    // when present.
    let candidates = vec![older.clone(), oldest];
    assert_eq!(
        validator.newest_valid(&candidates, None, None).unwrap().id,
        older.id
    );

    // Nothing valid left -> None.
    let candidates = vec![corrupt];
    assert!(validator.newest_valid(&candidates, Some(3), None).is_none());
}
