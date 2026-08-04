//! Health monitor (RC-10 M2).
//!
//! [`HealthMonitor`] converts the persisted worker liveness into a
//! one-number health view. [`assess`](HealthMonitor::assess) is pure:
//! it scores `0..=100` from the worker set and maps it to a
//! [`HealthStatus`]. [`capture`](HealthMonitor::capture) pulls the live
//! worker rows, assesses them, and persists the snapshot as a journal
//! entry (`entry_type = 'health'`) so health history is queryable the
//! same way the profiler ledger is.

use chrono::Utc;

use crate::errors::DatabaseError;
use crate::models::recovery::{
    HealthSnapshot, HealthStatus, JournalEntryType, WorkerHealth, WorkerStatus,
};
use crate::performance::recovery::journal::Journal;
use crate::repositories::RecoveryRepository;

/// How many points a stalled worker costs.
const STALLED_PENALTY: f64 = 25.0;
/// How many points a failed worker costs (critical).
const FAILED_PENALTY: f64 = 50.0;
/// Snapshot status thresholds.
const DEGRADED_AT: f64 = 80.0;
const CRITICAL_AT: f64 = 40.0;

#[derive(Debug, Clone)]
pub struct HealthMonitor {
    repository: RecoveryRepository,
    journal: Journal,
}

impl HealthMonitor {
    pub fn new(repository: RecoveryRepository, journal: Journal) -> Self {
        Self {
            repository,
            journal,
        }
    }

    /// Pure scoring: `100` minus per-worker penalties, clamped to
    /// `0..=100` and mapped to a status. Workers reporting `healthy` or
    /// `idle` cost nothing; the score is `100` on an empty worker set.
    pub fn assess(&self, workers: &[WorkerHealth]) -> (HealthStatus, f64) {
        let mut score: f64 = 100.0;
        for worker in workers {
            match worker.status {
                WorkerStatus::Healthy | WorkerStatus::Idle => {}
                WorkerStatus::Stalled => score -= STALLED_PENALTY,
                WorkerStatus::Failed => score -= FAILED_PENALTY,
            }
        }
        let score = score.clamp(0.0, 100.0);
        let status = if score >= DEGRADED_AT {
            HealthStatus::Healthy
        } else if score >= CRITICAL_AT {
            HealthStatus::Degraded
        } else {
            HealthStatus::Critical
        };
        (status, score)
    }

    /// Captures and persists a health snapshot, returning it. Issues are
    /// the stalled/failed worker names plus any invalid checkpoint.
    pub async fn capture(
        &self,
        checkpoint_issues: Vec<String>,
    ) -> Result<HealthSnapshot, DatabaseError> {
        let workers = self.repository.all_worker_health().await?;
        let (status, overall_score) = self.assess(&workers);

        let mut issues: Vec<String> = checkpoint_issues;
        for worker in &workers {
            if worker.status == WorkerStatus::Stalled {
                issues.push(format!("worker '{}' is stalled", worker.worker));
            } else if worker.status == WorkerStatus::Failed {
                issues.push(format!("worker '{}' has failed", worker.worker));
            }
        }

        let snapshot = HealthSnapshot {
            captured_at: Utc::now(),
            status,
            overall_score,
            workers,
            issues,
            details: serde_json::Value::Null,
        };

        self.journal
            .append(
                JournalEntryType::Health,
                "health",
                "app",
                status.as_str(),
                &serde_json::to_value(&snapshot)?,
            )
            .await?;
        Ok(snapshot)
    }
}
#[cfg(test)]
#[path = "health_monitor_tests.rs"]
mod tests;
