//! Recovery manager (RC-10 M2).
//!
//! [`RecoveryManager`] is the facade `lib.rs` wires as managed Tauri
//! state: it composes the journal, crash-recovery service, checkpoint
//! validator, rollback service, watchdog, health monitor and self-healing
//! service over the [`RecoveryRepository`], and drives the background
//! watchdog loop. No SQL is written here and no policy is duplicated —
//! every operation delegates to one of the components.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::DatabaseError;
use crate::models::recovery::{
    CrashReport, HealthSnapshot, JournalEntryType, RecoveryHistory, RecoveryJournalEntry,
    RecoveryRun, RollbackResult, SelfHealingReport, WatchdogEvent,
};
use crate::performance::recovery::{
    checkpoint_validator::CheckpointValidator, crash_recovery::CrashRecoveryService,
    health_monitor::HealthMonitor, journal::Journal, rollback::RollbackService,
    self_healing::SelfHealingService, watchdog::default_grace, watchdog::WatchdogService,
    WATCHDOG_INTERVAL_SECS,
};
use crate::repositories::RecoveryRepository;

/// The runtime's own liveness marker in `worker_health`.
const RUNTIME_WORKER: &str = "runtime";
/// Journal a heartbeat entry every Nth watchdog tick.
const HEARTBEAT_JOURNAL_EVERY: u64 = 10;

/// Facade for all reliability & recovery operations.
#[derive(Clone)]
pub struct RecoveryManager {
    repository: RecoveryRepository,
    journal: Journal,
    crash_recovery: CrashRecoveryService,
    validator: CheckpointValidator,
    rollback: RollbackService,
    watchdog: WatchdogService,
    health_monitor: HealthMonitor,
    self_healing: SelfHealingService,
    tick: std::sync::Arc<AtomicU64>,
}

impl RecoveryManager {
    pub fn new(repository: RecoveryRepository) -> Self {
        let journal = Journal::new(repository.clone());
        let validator = CheckpointValidator;
        Self {
            crash_recovery: CrashRecoveryService::new(repository.clone(), journal.clone()),
            rollback: RollbackService::new(repository.clone(), journal.clone()),
            watchdog: WatchdogService::new(repository.clone(), journal.clone()),
            health_monitor: HealthMonitor::new(repository.clone(), journal.clone()),
            self_healing: SelfHealingService::new(repository.clone(), journal.clone()),
            repository,
            journal,
            validator,
            tick: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }

    // ------------------------------------------------------------------
    // Startup & shutdown
    // ------------------------------------------------------------------

    /// The startup pass: registers the runtime's own liveness marker and
    /// runs crash detection + startup recovery.
    pub async fn startup(&self) -> Result<RecoveryRun, DatabaseError> {
        self.repository.register_worker(RUNTIME_WORKER).await?;
        self.crash_recovery.detect_and_recover().await
    }

    /// Records the clean-shutdown checkpoint (wired to `RunEvent::Exit`
    /// so the next launch can tell a clean stop from a crash).
    pub async fn record_clean_shutdown(&self) -> Result<(), DatabaseError> {
        self.journal
            .checkpoint("runtime", "clean", &[], serde_json::Value::Null)
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Checkpoints & heartbeats
    // ------------------------------------------------------------------

    /// Persists a running checkpoint with the active job set.
    pub async fn checkpoint(
        &self,
        active_jobs: &[String],
        metadata: serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        self.journal
            .checkpoint("runtime", "running", active_jobs, metadata)
            .await
    }

    /// Refreshes the runtime liveness marker (driven by the watchdog
    /// loop; also the hook integrations call to prove they are alive).
    pub async fn heartbeat(&self) -> Result<(), DatabaseError> {
        self.repository.heartbeat_worker(RUNTIME_WORKER).await
    }

    /// Registers a background worker for watchdog monitoring (opt-in by
    /// integrations; workers then prove liveness via `heartbeat`).
    pub async fn register_worker(&self, name: &str) -> Result<i64, DatabaseError> {
        self.repository.register_worker(name).await
    }

    // ------------------------------------------------------------------
    // Watchdog loop
    // ------------------------------------------------------------------

    /// One watchdog pass: heartbeat the runtime marker, scan worker
    /// health, capture a health snapshot, and run self-healing.
    pub async fn watchdog_tick(&self) -> Result<Vec<WatchdogEvent>, DatabaseError> {
        let tick = self.tick.fetch_add(1, Ordering::SeqCst) + 1;
        self.heartbeat().await?;
        if tick % HEARTBEAT_JOURNAL_EVERY == 0 {
            self.journal
                .append(
                    JournalEntryType::Heartbeat,
                    "watchdog",
                    RUNTIME_WORKER,
                    "alive",
                    &serde_json::json!({ "tick": tick }),
                )
                .await?;
        }

        let events = self.watchdog.scan(default_grace()).await?;
        let snapshot = self.health_monitor.capture(vec![]).await?;
        let report = self.self_healing.run(&snapshot).await?;
        if !report.executed.is_empty() {
            tracing::info!(
                executed = report.executed.len(),
                healed = report.healed_workers.len(),
                "self-healing pass applied"
            );
        }
        Ok(events)
    }

    /// The infinite watchdog loop (spawned from `lib.rs` setup). Every
    /// pass is isolated: one failure is logged and the loop continues,
    /// so a database hiccup never stops monitoring permanently.
    pub async fn watchdog_loop(&self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(WATCHDOG_INTERVAL_SECS)).await;
            if let Err(error) = self.watchdog_tick().await {
                tracing::warn!(error = %error, "watchdog pass failed");
            }
        }
    }

    // ------------------------------------------------------------------
    // Status & history
    // ------------------------------------------------------------------

    /// A fresh health snapshot (persisted as health history).
    pub async fn status(&self) -> Result<HealthSnapshot, DatabaseError> {
        self.health_monitor.capture(vec![]).await
    }

    /// Combined recovery history: runs, crashes, and recent journal.
    pub async fn history(&self, limit: u32) -> Result<RecoveryHistory, DatabaseError> {
        let limit = limit.clamp(1, 500);
        Ok(RecoveryHistory {
            runs: self.repository.recent_recovery_runs(limit).await?,
            crashes: self.repository.recent_crash_reports(limit).await?,
            journal: self.repository.recent_journal(limit).await?,
        })
    }

    /// The most recent crash reports, newest-first.
    pub async fn crash_reports(&self, limit: u32) -> Result<Vec<CrashReport>, DatabaseError> {
        self.repository.recent_crash_reports(limit).await
    }

    /// The latest checkpoint, if any.
    pub async fn latest_checkpoint(&self) -> Result<Option<RecoveryJournalEntry>, DatabaseError> {
        self.journal.latest_checkpoint().await
    }

    // ------------------------------------------------------------------
    // Manual interventions
    // ------------------------------------------------------------------

    /// Runs a self-healing pass on demand.
    pub async fn run_self_healing(&self) -> Result<SelfHealingReport, DatabaseError> {
        let snapshot = self.health_monitor.capture(vec![]).await?;
        self.self_healing.run(&snapshot).await
    }

    /// Rolls back to the newest valid ancestor of the latest checkpoint
    /// (the manual counterpart of the automatic corrupt-checkpoint path).
    pub async fn rollback_now(&self) -> Result<RollbackResult, DatabaseError> {
        let Some(latest) = self.journal.latest_checkpoint().await? else {
            return Ok(RollbackResult {
                rolled_back_to: None,
                restored: vec![],
                ok: false,
                message: "no checkpoint to roll back from".to_string(),
            });
        };
        let candidates = self.repository.recent_checkpoints(10).await?;
        self.rollback.rollback(&latest, &candidates).await
    }

    /// Exposes the validator for call sites that need an explicit
    /// validation verdict (e.g. the check before a manual resume).
    pub fn validator(&self) -> CheckpointValidator {
        self.validator
    }

    /// Current watchdog tick count (diagnostics).
    pub fn tick_count(&self) -> u64 {
        self.tick.load(Ordering::SeqCst)
    }
}
#[cfg(test)]
#[path = "recovery_manager_tests.rs"]
mod tests;
