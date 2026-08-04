//! Data Integrity & Backup engine (RC-10 M3 — Production Hardening).
//!
//! Facade over the four maintenance subsystems — the integrity checker,
//! backup service, restore service and the maintenance runner — plus the
//! `backup_runs` audit ledger. `lib.rs` wires one [`MaintenanceEngine`]
//! as managed Tauri state; the [`crate::commands::maintenance`] commands
//! are thin forwards to it.
//!
//! Layout mirrors `performance` (RC-10 M1): the engine composes
//! repositories (SQL) and models (DTOs); the SQL for the audit ledger and
//! the maintenance statements lives in
//! [`crate::repositories::MaintenanceRepository`], the diagnostic `PRAGMA`
//! battery in [`integrity`], and the policy (when to vacuum, how restores
//! are staged) in the per-subsystem modules.

pub mod backup;
pub mod integrity;
pub mod restore;
pub mod runner;

use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;

use crate::errors::DatabaseError;
use crate::models::backup::{
    BackupRun, BackupRunKind, BackupRunStatus, IntegrityReport, MaintenanceReport, RestoreResult,
};
use crate::repositories::MaintenanceRepository;

pub use backup::BackupService;
pub use integrity::IntegrityChecker;
pub use restore::RestoreService;
pub use runner::MaintenanceRunner;

/// Facade for all data integrity & backup operations.
#[derive(Clone)]
pub struct MaintenanceEngine {
    repository: MaintenanceRepository,
    checker: IntegrityChecker,
    backup_service: BackupService,
    restore_service: RestoreService,
    runner: MaintenanceRunner,
    backup_dir: PathBuf,
}

impl MaintenanceEngine {
    /// Constructs the engine. `db_path` is the live database file (its
    /// directory also holds the pending-restore marker); `backup_dir` is
    /// where snapshots are written.
    pub fn new(repository: MaintenanceRepository, db_path: PathBuf, backup_dir: PathBuf) -> Self {
        let checker = IntegrityChecker::new(repository.clone(), db_path.display().to_string());
        let backup_service = BackupService::new(repository.clone(), backup_dir.clone());
        let restore_service = RestoreService::new(repository.clone(), db_path);
        let runner = MaintenanceRunner::new(repository.clone());
        Self {
            repository,
            checker,
            backup_service,
            restore_service,
            runner,
            backup_dir,
        }
    }

    /// The full `PRAGMA` battery over the live database, audited.
    pub async fn integrity(&self) -> Result<IntegrityReport, DatabaseError> {
        let started = Instant::now();
        let main = self.checker.check_live().await?;
        let ok = main.integrity.ok && main.quick_check.ok && main.foreign_key_check.is_empty();
        let _ = self
            .repository
            .record_run(
                BackupRunKind::Integrity,
                if ok {
                    BackupRunStatus::Success
                } else {
                    BackupRunStatus::Failed
                },
                "",
                0,
                "",
                if ok {
                    "all checks passed"
                } else {
                    "integrity violations detected"
                },
                started.elapsed().as_millis() as i64,
            )
            .await;
        Ok(IntegrityReport {
            checked_at: Utc::now(),
            db_path: self.checker.db_path().to_string(),
            main,
            ok,
        })
    }

    /// Creates a snapshot and records it in the ledger.
    pub async fn backup(&self) -> Result<BackupRun, DatabaseError> {
        self.backup_service.create().await
    }

    /// Most recent ledger rows (all kinds), newest-first.
    pub async fn backups(&self, limit: u32) -> Result<Vec<BackupRun>, DatabaseError> {
        self.repository.recent_runs(limit.clamp(1, 500)).await
    }

    /// Stages the backup referenced by `backup_id` for restore on the
    /// next launch. The id must reference a completed backup run; its
    /// stored filename is resolved against the backup directory.
    pub async fn restore(&self, backup_id: i64) -> Result<RestoreResult, DatabaseError> {
        let run =
            self.repository
                .run_by_id(backup_id)
                .await?
                .ok_or_else(|| DatabaseError::NotFound {
                    entity: "backup run",
                    id: backup_id.to_string(),
                })?;
        if run.kind != BackupRunKind::Backup {
            return Err(DatabaseError::InvalidInput(format!(
                "run {backup_id} is not a backup"
            )));
        }
        if run.status != BackupRunStatus::Success {
            return Err(DatabaseError::InvalidInput(format!(
                "backup {backup_id} did not complete successfully"
            )));
        }
        let backup_path = self.backup_dir.join(&run.path);
        self.restore_service.stage(&backup_path).await
    }

    /// Whether a staged restore is waiting to be applied on next launch.
    pub async fn pending_restore(&self) -> Result<Option<RestoreResult>, DatabaseError> {
        self.restore_service.pending().await
    }

    /// Discards a staged restore.
    pub async fn cancel_restore(&self) -> Result<(), DatabaseError> {
        self.restore_service.cancel().await
    }

    /// Runs the maintenance pass (checkpoint → maybe VACUUM → optimize).
    pub async fn maintenance(&self) -> Result<MaintenanceReport, DatabaseError> {
        self.runner.run().await
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
