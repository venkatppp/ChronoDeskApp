//! Checkpoint validation (RC-10 M2).
//!
//! [`CheckpointValidator`] decides whether a persisted checkpoint is
//! trustworthy enough to resume from after a crash. It is pure: all rules
//! are functions of the entries handed to it. A checkpoint is valid when
//! its checksum matches a recomputation over `(entity, state, payload)`,
//! its state is non-empty, its payload carries an `active_jobs` array,
//! and its timestamp does not precede the previous checkpoint's. Validation
//! runs before *either* path of startup recovery: a valid checkpoint is
//! resumed from; an invalid one triggers a rollback to the last valid
//! ancestor instead of a blind resume.

use chrono::Utc;

use crate::models::recovery::{CheckpointValidationResult, JournalEntryType, RecoveryJournalEntry};

/// Pure rules deciding whether a checkpoint entry can be trusted.
#[derive(Debug, Clone, Copy)]
pub struct CheckpointValidator;

impl CheckpointValidator {
    /// Validates `entry` against the checksum rule and, when `previous`
    /// is supplied, the monotonic-ordering rule.
    pub fn validate(
        &self,
        entry: &RecoveryJournalEntry,
        previous: Option<&RecoveryJournalEntry>,
    ) -> CheckpointValidationResult {
        let mut issues: Vec<String> = Vec::new();

        if entry.entry_type != JournalEntryType::Checkpoint {
            issues.push(format!("entry {} is not a checkpoint", entry.id,));
        }

        if entry.state.is_empty() {
            issues.push("checkpoint state is empty".to_string());
        }

        let expected_checksum = crate::performance::recovery::journal::Journal::checksum(
            &entry.entity,
            &entry.state,
            &entry.payload,
        );
        if entry.checksum.is_empty() || entry.checksum != expected_checksum {
            issues.push(
                "checksum mismatch — checkpoint may be half-written after a crash".to_string(),
            );
        }

        match &entry.payload {
            serde_json::Value::Object(map) => {
                if !map.contains_key("active_jobs") {
                    issues.push("checkpoint payload has no active_jobs array".to_string());
                }
            }
            _ => issues.push("checkpoint payload is not a JSON object".to_string()),
        }

        if let Some(previous) = previous {
            if entry.created_at < previous.created_at {
                issues.push("checkpoint timestamp precedes the previous checkpoint".to_string());
            }
        }

        CheckpointValidationResult {
            valid: issues.is_empty(),
            issues,
            entry_id: Some(entry.id),
            checked_at: Utc::now(),
        }
    }

    /// Finds the newest valid checkpoint in `candidates` (newest-first,
    /// as returned by the repository), optionally skipping `exclude_id`
    /// (the corrupt checkpoint being rolled back through). Returns `None`
    /// when nothing survives the rules.
    pub fn newest_valid(
        &self,
        candidates: &[RecoveryJournalEntry],
        exclude_id: Option<i64>,
        previous: Option<&RecoveryJournalEntry>,
    ) -> Option<RecoveryJournalEntry> {
        candidates
            .iter()
            .filter(|entry| Some(entry.id) != exclude_id)
            .find(|entry| self.validate(entry, previous).valid)
            .cloned()
    }
}

#[cfg(test)]
#[path = "checkpoint_validator_tests.rs"]
mod tests;
