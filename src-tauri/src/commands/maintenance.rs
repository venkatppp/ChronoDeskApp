//! RC-10 M3 data integrity & backup IPC commands.
//!
//! Thin wrappers only: every command pulls the [`MaintenanceEngine`]
//! state and forwards to its facade method. Zero business logic lives
//! here — validation, staging, and maintenance policy live in
//! [`crate::maintenance`] and the SQL in
//! [`crate::repositories::MaintenanceRepository`].

use tauri::State;

use crate::errors::DatabaseError;
use crate::maintenance::MaintenanceEngine;
use crate::models::backup::{BackupRun, IntegrityReport, MaintenanceReport, RestoreResult};

/// Runs the full `PRAGMA` battery over the live database.
#[tauri::command]
pub async fn maintenance_integrity(
    engine: State<'_, MaintenanceEngine>,
) -> Result<IntegrityReport, DatabaseError> {
    engine.integrity().await
}

/// Creates a backup snapshot in the backup directory.
#[tauri::command]
pub async fn maintenance_backup(
    engine: State<'_, MaintenanceEngine>,
) -> Result<BackupRun, DatabaseError> {
    engine.backup().await
}

/// The most recent backup/integrity/maintenance ledger rows.
#[tauri::command]
pub async fn maintenance_backups(
    engine: State<'_, MaintenanceEngine>,
    limit: Option<u32>,
) -> Result<Vec<BackupRun>, DatabaseError> {
    engine.backups(limit.unwrap_or(20)).await
}

/// Stages the backup referenced by `backup_id` for restore on the next
/// launch (validated first; the swap happens before the pool opens).
#[tauri::command]
pub async fn maintenance_restore(
    engine: State<'_, MaintenanceEngine>,
    backup_id: i64,
) -> Result<RestoreResult, DatabaseError> {
    engine.restore(backup_id).await
}

/// Whether a staged restore is waiting to be applied on next launch.
#[tauri::command]
pub async fn maintenance_pending_restore(
    engine: State<'_, MaintenanceEngine>,
) -> Result<Option<RestoreResult>, DatabaseError> {
    engine.pending_restore().await
}

/// Discards a staged restore.
#[tauri::command]
pub async fn maintenance_cancel_restore(
    engine: State<'_, MaintenanceEngine>,
) -> Result<(), DatabaseError> {
    engine.cancel_restore().await
}

/// Runs the maintenance pass (checkpoint → maybe VACUUM → optimize).
#[tauri::command]
pub async fn maintenance_optimize(
    engine: State<'_, MaintenanceEngine>,
) -> Result<MaintenanceReport, DatabaseError> {
    engine.maintenance().await
}
