//! Watchdog service (RC-10 M2).
//!
//! [`WatchdogService`] monitors the persisted `worker_health` rows: a pure
//! [`evaluate`](WatchdogService::evaluate) pass turns each row into a
//! `stalled`/`recovered` event based on the heartbeat grace window, and
//! an async [`scan`](WatchdogService::scan) pass applies those events to
//! the repository (incrementing `consecutive_misses`, flipping status)
//! and records each event in the journal. The recovery manager's loop
//! drives `scan` on a fixed interval; self-healing consumes the
//! accumulated misses afterwards.
//!
//! [`WATCHDOG_HEARTBEAT_GRACE_SECS`]: crate::performance::recovery::WATCHDOG_HEARTBEAT_GRACE_SECS

use chrono::{DateTime, Duration, Utc};

use crate::errors::DatabaseError;
use crate::models::recovery::{WatchdogEvent, WorkerHealth, WorkerStatus};
use crate::performance::recovery::journal::Journal;
use crate::repositories::RecoveryRepository;

/// Detects stale heartbeats and turns them into journaled events.
#[derive(Debug, Clone)]
pub struct WatchdogService {
    repository: RecoveryRepository,
    journal: Journal,
}

impl WatchdogService {
    pub fn new(repository: RecoveryRepository, journal: Journal) -> Self {
        Self {
            repository,
            journal,
        }
    }

    /// Pure evaluation: one event per worker whose heartbeat is stale
    /// (and not already stalled/failed), plus a `recovered` event for a
    /// previously stalled/failed worker whose heartbeat is fresh again.
    pub fn evaluate(
        &self,
        workers: &[WorkerHealth],
        now: DateTime<Utc>,
        grace: Duration,
    ) -> Vec<WatchdogEvent> {
        let mut events = Vec::new();
        for worker in workers {
            let stale = worker.last_heartbeat + grace < now;
            match worker.status {
                WorkerStatus::Healthy | WorkerStatus::Idle if stale => {
                    events.push(WatchdogEvent {
                        worker: worker.worker.clone(),
                        kind: "stalled".to_string(),
                        detail: format!(
                            "no heartbeat since {} (grace {}s elapsed)",
                            worker.last_heartbeat,
                            grace.num_seconds()
                        ),
                        occurred_at: now,
                    });
                }
                WorkerStatus::Stalled | WorkerStatus::Failed if !stale => {
                    events.push(WatchdogEvent {
                        worker: worker.worker.clone(),
                        kind: "recovered".to_string(),
                        detail: "heartbeat resumed within the grace window".to_string(),
                        occurred_at: now,
                    });
                }
                _ => {}
            }
        }
        events
    }

    /// Applies one evaluation pass: persists each event to the worker
    /// rows and the journal, returning the events for the caller to
    /// surface. Idle workers are skipped (they never heartbeat by
    /// design).
    ///
    /// In addition to the [`evaluate`](WatchdogService::evaluate)
    /// transition events, `scan` synthesizes two continuation signals
    /// that the pure pass deliberately does not produce:
    ///
    /// - A worker that *stays* stale re-reports `stalled` on every pass,
    ///   so `consecutive_misses` keeps climbing until self-healing's
    ///   restart threshold (`WORKER_RESTART_AFTER_MISSES`) is reached.
    ///   Without this, misses would cap at 1 and a genuinely dead worker
    ///   would never be restarted.
    /// - A worker that is healthy again after a fresh heartbeat reports
    ///   `recovered` when the newest journal entry about it says it was
    ///   still `stalled`. `heartbeat_worker` immediately returns a row to
    ///   `healthy`, so the transition back can only be observed through
    ///   the journal — one `recovered` event per recovery.
    pub async fn scan(&self, grace: Duration) -> Result<Vec<WatchdogEvent>, DatabaseError> {
        let workers = self.repository.all_worker_health().await?;
        let now = Utc::now();
        let mut events = self.evaluate(&workers, now, grace);

        for worker in &workers {
            let stale = worker.last_heartbeat + grace < now;
            if worker.status == WorkerStatus::Stalled && stale {
                events.push(WatchdogEvent {
                    worker: worker.worker.clone(),
                    kind: "stalled".to_string(),
                    detail: format!(
                        "worker remains stalled since {} ({} consecutive watchdog passes)",
                        worker.last_heartbeat,
                        worker.consecutive_misses + 1
                    ),
                    occurred_at: now,
                });
            } else if worker.status == WorkerStatus::Healthy
                && !stale
                && self.was_stalled(&worker.worker).await?
            {
                events.push(WatchdogEvent {
                    worker: worker.worker.clone(),
                    kind: "recovered".to_string(),
                    detail: "heartbeat resumed within the grace window".to_string(),
                    occurred_at: now,
                });
            }
        }

        for event in &events {
            match event.kind.as_str() {
                "stalled" => self.repository.record_worker_miss(&event.worker).await?,
                "recovered" => self.repository.mark_worker_healthy(&event.worker).await?,
                _ => {}
            }
            self.journal
                .append(
                    crate::models::recovery::JournalEntryType::SelfHealing,
                    "watchdog",
                    &event.worker,
                    &event.kind,
                    &serde_json::json!({ "detail": event.detail }),
                )
                .await?;
        }
        Ok(events)
    }

    /// Whether the newest journal entry about `worker` says it was still
    /// stalled — the evidence that a fresh, healthy row represents a
    /// *recovery* rather than a worker that never left `healthy`.
    async fn was_stalled(&self, worker: &str) -> Result<bool, DatabaseError> {
        let recent = self.repository.journal_for_entity(worker, 1).await?;
        Ok(recent.first().is_some_and(|entry| entry.state == "stalled"))
    }
}

/// Convenience for tests and call sites that need the default grace.
pub fn default_grace() -> Duration {
    Duration::seconds(crate::performance::recovery::WATCHDOG_HEARTBEAT_GRACE_SECS)
}

#[cfg(test)]
#[path = "watchdog_tests.rs"]
mod tests;
