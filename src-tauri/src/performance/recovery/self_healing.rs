//! Self-healing service (RC-10 M2).
//!
//! [`SelfHealingService`] turns a [`HealthSnapshot`] into safe, bounded
//! remediation. The pure [`plan`](SelfHealingService::plan) pass lists
//! the actions the snapshot calls for (restart a worker whose monitoring
//! state has missed enough heartbeats, verify the latest checkpoint when
//! health is critical); the async [`run`](SelfHealingService::run)
//! executes them through the repository, keeps the ledger bounded by
//! pruning history past its threshold, and returns a report of what was
//! actually done. Never destructive: it never deletes worker rows or
//! touches non-recovery tables.

use chrono::Utc;

use crate::errors::DatabaseError;
use crate::models::recovery::{HealthSnapshot, JournalEntryType, SelfHealingReport, WorkerStatus};
use crate::performance::recovery::{
    checkpoint_validator::CheckpointValidator, journal::Journal, rollback::RollbackService,
};
use crate::repositories::RecoveryRepository;

const RESTART_PREFIX: &str = "restart_worker:";
const VERIFY_ACTION: &str = "verify_checkpoint";
const PRUNE_ACTION: &str = "prune_history";

#[derive(Debug, Clone)]
pub struct SelfHealingService {
    repository: RecoveryRepository,
    journal: Journal,
    validator: CheckpointValidator,
    rollback: RollbackService,
}

impl SelfHealingService {
    pub fn new(repository: RecoveryRepository, journal: Journal) -> Self {
        Self {
            repository: repository.clone(),
            journal: journal.clone(),
            validator: CheckpointValidator,
            rollback: RollbackService::new(repository, journal),
        }
    }

    /// Pure planning: actions the snapshot requires, in a stable order.
    /// A worker whose monitoring state has missed `WORKER_RESTART_AFTER_MISSES`
    /// heartbeats (or has failed outright) is a `restart_worker:<name>`
    /// candidate; a critical/issue-laden snapshot also asks for a
    /// checkpoint verification.
    pub fn plan(&self, snapshot: &HealthSnapshot) -> Vec<String> {
        let mut plan = Vec::new();
        for worker in &snapshot.workers {
            let needs_restart = (worker.status == WorkerStatus::Stalled
                && worker.consecutive_misses
                    >= crate::performance::recovery::WORKER_RESTART_AFTER_MISSES)
                || worker.status == WorkerStatus::Failed;
            if needs_restart {
                plan.push(format!("{RESTART_PREFIX}{}", worker.worker));
            }
        }
        if snapshot.status != crate::models::recovery::HealthStatus::Healthy
            || !snapshot.issues.is_empty()
        {
            plan.push(VERIFY_ACTION.to_string());
        }
        plan
    }

    /// Executes the planned actions against the repository and returns
    /// what actually happened. The journal's size is checked first so a
    /// large ledger is pruned without waiting for its count to be
    /// surfaced as an issue.
    pub async fn run(&self, snapshot: &HealthSnapshot) -> Result<SelfHealingReport, DatabaseError> {
        let mut executed = Vec::new();
        let mut failed = Vec::new();
        let mut healed_workers = Vec::new();

        for action in self.plan(snapshot) {
            if let Some(name) = action.strip_prefix(RESTART_PREFIX) {
                match self.restart_worker(name).await {
                    Ok(()) => {
                        healed_workers.push(name.to_string());
                        executed.push(action);
                    }
                    Err(error) => {
                        tracing::warn!(worker = %name, error = %error, "worker restart failed");
                        failed.push(action);
                    }
                }
            } else if action == VERIFY_ACTION {
                match self.verify_checkpoint().await {
                    Ok(true) => executed.push(action),
                    Ok(false) => failed.push(action),
                    Err(error) => {
                        tracing::warn!(error = %error, "checkpoint verification failed");
                        failed.push(action);
                    }
                }
            }
        }

        let journal_count = self.repository.journal_count().await?;
        if journal_count >= crate::performance::recovery::JOURNAL_PRUNE_THRESHOLD {
            let removed = self.prune_history().await?;
            if removed > 0 {
                tracing::info!(removed, "recovery journal pruned to a bounded size");
                executed.push(PRUNE_ACTION.to_string());
            }
        }

        Ok(SelfHealingReport {
            executed,
            failed,
            healed_workers,
            ran_at: Utc::now(),
        })
    }

    /// Restarts a worker's monitoring state (fresh heartbeat, zero
    /// misses/errors) and journals it.
    async fn restart_worker(&self, worker: &str) -> Result<(), DatabaseError> {
        self.repository.mark_worker_healthy(worker).await?;
        self.journal
            .append(
                JournalEntryType::SelfHealing,
                "self_healing",
                worker,
                "restarted",
                &serde_json::Value::Null,
            )
            .await?;
        Ok(())
    }

    /// Verifies the latest checkpoint; when it fails validation, rolls
    /// back to the newest valid ancestor. Returns `false` only when the
    /// rollback itself failed.
    async fn verify_checkpoint(&self) -> Result<bool, DatabaseError> {
        let Some(latest) = self.journal.latest_checkpoint().await? else {
            return Ok(true);
        };
        if self.validator.validate(&latest, None).valid {
            return Ok(true);
        }
        let candidates = self.rollback.candidates().await?;
        Ok(self.rollback.rollback(&latest, &candidates).await?.ok)
    }

    /// Bounded ledger: drops all but the newest journal entries past the
    /// configured threshold, keeping crash reports bounded the same way.
    async fn prune_history(&self) -> Result<u64, DatabaseError> {
        let keep = crate::performance::recovery::JOURNAL_PRUNE_THRESHOLD;
        let removed_journal = self.repository.prune_journal_excess(keep).await?;
        let crash_count = self.repository.crash_report_count().await?;
        if crash_count > 1000 {
            let since = chrono::Utc::now()
                - chrono::Duration::days(crate::performance::recovery::JOURNAL_PRUNE_KEEP_DAYS);
            self.repository.prune_crash_reports_before(since).await?;
        }
        Ok(removed_journal)
    }
}
#[cfg(test)]
#[path = "self_healing_tests.rs"]
mod tests;
