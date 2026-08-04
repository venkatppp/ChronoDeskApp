//! Crash detection & startup recovery (RC-10 M2).
//!
//! [`CrashRecoveryService`] runs at every startup: it reads the latest
//! checkpoint from the journal, classifies the previous session's end
//! (`clean` shutdown vs. a crash), and when a crash is detected validates
//! the checkpoint and either resumes the interrupted jobs or — when the
//! checkpoint fails validation — rolls back to the newest valid ancestor.
//! Every decision is recorded as a crash report, a journal entry, and a
//! `recovery_history` run so automatic intervention is auditable.
//!
//! The detection rule itself is pure and deliberately simple: the last
//! checkpoint saying anything other than `clean` means the process died
//! before the clean-shutdown checkpoint was written.

use std::time::Instant;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::recovery::{
    CrashType, JournalEntryType, RecoveryAction, RecoveryJournalEntry, RecoveryOutcome,
    RecoveryRun, RecoveryTrigger,
};
use crate::performance::recovery::{
    checkpoint_validator::CheckpointValidator, journal::Journal, rollback::RollbackService,
    watchdog::default_grace,
};
use crate::repositories::RecoveryRepository;

/// A non-`clean` checkpoint left behind is a crash; the classification
/// separates "killed very recently" from "silently dead for a while".
fn classify_crash(entry: &RecoveryJournalEntry, now: DateTime<Utc>, grace: Duration) -> CrashType {
    if entry.created_at + grace < now {
        CrashType::Timeout
    } else {
        CrashType::Unknown
    }
}

#[derive(Debug, Clone)]
pub struct CrashRecoveryService {
    repository: RecoveryRepository,
    journal: Journal,
    validator: CheckpointValidator,
    rollback: RollbackService,
}

impl CrashRecoveryService {
    pub fn new(repository: RecoveryRepository, journal: Journal) -> Self {
        Self {
            repository: repository.clone(),
            journal: journal.clone(),
            validator: CheckpointValidator,
            rollback: RollbackService::new(repository, journal),
        }
    }

    /// Pure crash detection: `None` when there is no checkpoint yet
    /// (first run) or the last checkpoint recorded a clean shutdown;
    /// otherwise the classified crash type.
    pub fn detect_crash(
        &self,
        latest: Option<&RecoveryJournalEntry>,
        now: DateTime<Utc>,
        grace: Duration,
    ) -> Option<CrashType> {
        match latest {
            None => None,
            Some(entry) if entry.state == "clean" => None,
            Some(entry) => Some(classify_crash(entry, now, grace)),
        }
    }

    /// The startup recovery pass: detect, validate, resume or roll back,
    /// persist the audit trail, and open the new session's checkpoint.
    pub async fn detect_and_recover(&self) -> Result<RecoveryRun, DatabaseError> {
        let started_at = Utc::now();
        let started = Instant::now();
        let latest = self.journal.latest_checkpoint().await?;

        let run = match self.detect_crash(latest.as_ref(), started_at, default_grace()) {
            None => {
                // Clean start: open the new session's checkpoint. The
                // audit trail shows the check happened (outcome
                // `no_action`), so a history row is written every launch
                // — the same cadence `startup_profiles` uses.
                self.journal
                    .checkpoint(
                        "startup",
                        "running",
                        &[],
                        serde_json::json!({ "clean_start": true }),
                    )
                    .await?;
                self.finish_run(
                    started_at,
                    started,
                    RecoveryOutcome::NoAction,
                    "success",
                    vec![RecoveryAction::Checkpoint.as_str().to_string()],
                    vec![],
                    None,
                    vec![],
                )
            }
            Some(crash_type) => {
                let entry = latest.expect("crash detected implies a latest checkpoint");
                let (outcome, status, actions, jobs, rolled_back_to, errors, crash_id) =
                    self.recover_from(&entry, crash_type).await?;

                if let Some(crash_id) = crash_id {
                    if outcome != RecoveryOutcome::Failed {
                        self.repository.mark_crash_recovered(crash_id).await?;
                    }
                }

                self.journal
                    .checkpoint(
                        "startup",
                        "running",
                        &jobs,
                        serde_json::json!({
                            "recovered_from": entry.id,
                            "crash_type": crash_type.as_str(),
                        }),
                    )
                    .await?;

                self.finish_run(
                    started_at,
                    started,
                    outcome,
                    &status,
                    actions,
                    jobs,
                    rolled_back_to,
                    errors,
                )
            }
        };

        // Persisting the audit run is best-effort: a failing history
        // write must never fail the startup recovery itself.
        if let Err(error) = self.repository.record_recovery_run(&run).await {
            tracing::warn!(error = %error, "recovery history run could not be persisted");
        }
        Ok(run)
    }

    /// The recovery decision for a detected crash: validate the
    /// checkpoint and resume, or roll back through it when it is corrupt.
    #[allow(clippy::type_complexity)]
    async fn recover_from(
        &self,
        entry: &RecoveryJournalEntry,
        crash_type: CrashType,
    ) -> Result<
        (
            RecoveryOutcome,
            String,
            Vec<String>,
            Vec<String>,
            Option<i64>,
            Vec<String>,
            Option<i64>,
        ),
        DatabaseError,
    > {
        let severity = if matches!(
            crash_type,
            CrashType::Database | CrashType::CheckpointCorrupt
        ) {
            "critical"
        } else {
            "error"
        };
        let crash_id = self
            .repository
            .report_crash(
                "runtime",
                crash_type,
                severity,
                &format!(
                    "previous session ended without a clean shutdown (checkpoint {} was '{}')",
                    entry.id, entry.state
                ),
                "",
                &serde_json::json!({ "checkpoint_id": entry.id }),
            )
            .await?;

        let validation = self.validator.validate(entry, None);
        let mut actions = vec![RecoveryAction::Revalidate.as_str().to_string()];

        if validation.valid {
            // Resume the interrupted jobs from the checkpoint payload.
            let jobs = self.active_jobs(entry);
            actions.push(RecoveryAction::Resume.as_str().to_string());
            self.journal
                .append(
                    JournalEntryType::Recovery,
                    "startup",
                    "app",
                    "resumed",
                    &serde_json::json!({ "checkpoint_id": entry.id, "jobs": jobs }),
                )
                .await?;
            Ok((
                RecoveryOutcome::Recovered,
                "success".to_string(),
                actions,
                jobs,
                None,
                vec![],
                Some(crash_id),
            ))
        } else {
            // Corrupt checkpoint: roll back to the newest valid ancestor.
            let candidates = self.rollback.candidates().await?;
            let result = self.rollback.rollback(entry, &candidates).await?;
            actions.push(RecoveryAction::Rollback.as_str().to_string());
            if result.ok {
                Ok((
                    RecoveryOutcome::RolledBack,
                    "success".to_string(),
                    actions,
                    result.restored,
                    result.rolled_back_to,
                    vec![],
                    Some(crash_id),
                ))
            } else {
                Ok((
                    RecoveryOutcome::Failed,
                    "failed".to_string(),
                    actions,
                    vec![],
                    None,
                    vec![result.message],
                    Some(crash_id),
                ))
            }
        }
    }

    /// The `active_jobs` array from a checkpoint payload, when present.
    fn active_jobs(&self, entry: &RecoveryJournalEntry) -> Vec<String> {
        entry
            .payload
            .get("active_jobs")
            .and_then(|jobs| jobs.as_array())
            .map(|jobs| {
                jobs.iter()
                    .filter_map(|job| job.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Builds the audit run struct (persisted by the caller).
    #[allow(clippy::too_many_arguments)]
    fn finish_run(
        &self,
        started_at: DateTime<Utc>,
        started: Instant,
        outcome: RecoveryOutcome,
        status: &str,
        actions: Vec<String>,
        recovered_jobs: Vec<String>,
        rolled_back_to: Option<i64>,
        errors: Vec<String>,
    ) -> RecoveryRun {
        RecoveryRun {
            id: 0,
            run_id: Uuid::new_v4(),
            trigger: RecoveryTrigger::Startup,
            outcome,
            status: status.to_string(),
            actions,
            recovered_jobs,
            rolled_back_to,
            errors,
            duration_ms: started.elapsed().as_millis() as u64,
            started_at,
            completed_at: Utc::now(),
        }
    }
}
#[cfg(test)]
#[path = "crash_recovery_tests.rs"]
mod tests;
