//! Rollback service (RC-10 M2).
//!
//! [`RollbackService`] rolls state back to the newest valid ancestor of a
//! corrupt checkpoint: it finds the newest candidate that passes
//! [`CheckpointValidator`]'s rules (excluding the corrupt one), restores
//! the jobs that checkpoint recorded, and persists the decision as a
//! journal `rollback` entry. It deliberately owns no "what does restoring
//! a job mean" policy — the restored job list is returned for the caller
//! (crash recovery or self-healing) to act on.

use crate::errors::DatabaseError;
use crate::models::recovery::{JournalEntryType, RecoveryJournalEntry, RollbackResult};
use crate::performance::recovery::{checkpoint_validator::CheckpointValidator, journal::Journal};
use crate::repositories::RecoveryRepository;

#[derive(Debug, Clone)]
pub struct RollbackService {
    repository: RecoveryRepository,
    journal: Journal,
}

impl RollbackService {
    pub fn new(repository: RecoveryRepository, journal: Journal) -> Self {
        Self {
            repository,
            journal,
        }
    }

    /// The rollback candidate set: the most recent checkpoints,
    /// newest-first, as the repository stores them.
    pub async fn candidates(&self) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        self.repository.recent_checkpoints(10).await
    }

    /// Rolls back through `current` (the corrupt checkpoint) to the
    /// newest valid ancestor in `candidates` (newest-first, as the
    /// repository returns them). `None` is returned as a failed result
    /// when no valid ancestor exists.
    pub async fn rollback(
        &self,
        current: &RecoveryJournalEntry,
        candidates: &[RecoveryJournalEntry],
    ) -> Result<RollbackResult, DatabaseError> {
        match CheckpointValidator.newest_valid(candidates, Some(current.id), None) {
            Some(target) => {
                let restored = target
                    .payload
                    .get("active_jobs")
                    .and_then(|jobs| jobs.as_array())
                    .map(|jobs| {
                        jobs.iter()
                            .filter_map(|job| job.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();

                self.journal
                    .append(
                        JournalEntryType::Rollback,
                        "rollback",
                        "app",
                        "rolled_back",
                        &serde_json::json!({
                            "from_checkpoint": current.id,
                            "to_checkpoint": target.id,
                            "restored_jobs": restored,
                        }),
                    )
                    .await?;

                Ok(RollbackResult {
                    rolled_back_to: Some(target.id),
                    restored,
                    ok: true,
                    message: format!("rolled back to checkpoint {}", target.id),
                })
            }
            None => Ok(RollbackResult {
                rolled_back_to: None,
                restored: vec![],
                ok: false,
                message: "no valid ancestor checkpoint found to roll back to".to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[path = "rollback_tests.rs"]
mod tests;
