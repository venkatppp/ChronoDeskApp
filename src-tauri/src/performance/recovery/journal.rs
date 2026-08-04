//! Append-only reliability journal (RC-10 M2).
//!
//! [`Journal`] is the single writer to the `recovery_journal` table: it
//! formats every entry, computes the SHA-256 checksum over
//! `(entity, state, payload)` that [`CheckpointValidator`] later verifies,
//! and persists via [`crate::repositories::RecoveryRepository`]. No policy
//! decisions live here — every call site decides *what* happened; the
//! journal decides how it is recorded.

use sha2::{Digest, Sha256};

use crate::errors::DatabaseError;
use crate::models::recovery::{JournalEntryType, RecoveryJournalEntry};
use crate::repositories::RecoveryRepository;

/// Writes formatted, checksummed entries to the recovery journal.
#[derive(Debug, Clone)]
pub struct Journal {
    repository: RecoveryRepository,
}

impl Journal {
    pub fn new(repository: RecoveryRepository) -> Self {
        Self { repository }
    }

    /// The checksum recorded with an entry: SHA-256 hex over the
    /// canonical `entity|state|payload` string. `payload` is serialized
    /// deterministically by `serde_json` (a single string per value), so
    /// the validator can recompute the exact same digest from a read
    /// row.
    pub fn checksum(entity: &str, state: &str, payload: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(entity.as_bytes());
        hasher.update(*b"|");
        hasher.update(state.as_bytes());
        hasher.update(*b"|");
        hasher.update(payload.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Appends one entry and returns its row id.
    pub async fn append(
        &self,
        entry_type: JournalEntryType,
        scope: &str,
        entity: &str,
        state: &str,
        payload: &serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let checksum = Self::checksum(entity, state, payload);
        self.repository
            .append_journal_entry(entry_type, scope, entity, state, payload, &checksum)
            .await
    }

    /// Appends a checkpoint entry carrying the active-job payload. The
    /// checksum makes the checkpoint verifiable after a crash — a
    /// half-written row fails [`CheckpointValidator`] and triggers a
    /// rollback instead of a blind resume.
    pub async fn checkpoint(
        &self,
        scope: &str,
        state: &str,
        active_jobs: &[String],
        metadata: serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let payload = serde_json::json!({ "active_jobs": active_jobs, "metadata": metadata });
        self.append(JournalEntryType::Checkpoint, scope, "app", state, &payload)
            .await
    }

    /// The most recent entries (newest-first).
    pub async fn recent(&self, limit: u32) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        self.repository.recent_journal(limit).await
    }

    /// The most recent checkpoint entry, if any.
    pub async fn latest_checkpoint(&self) -> Result<Option<RecoveryJournalEntry>, DatabaseError> {
        self.repository.latest_checkpoint().await
    }

    /// Total entry count (self-healing pruning trigger).
    pub async fn count(&self) -> Result<u64, DatabaseError> {
        self.repository.journal_count().await
    }
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
