//! RC-10 M2 reliability & recovery IPC commands.
//!
//! Thin wrappers only: every command pulls the [`RecoveryManager`] state
//! and forwards to its facade method. Zero business logic lives here —
//! detection, validation, and remediation policy all live in
//! [`crate::performance::recovery`] and the SQL in
//! [`crate::repositories::RecoveryRepository`].

use tauri::State;

use crate::errors::DatabaseError;
use crate::models::recovery::{
    CrashReport, HealthSnapshot, RecoveryHistory, RecoveryJournalEntry, RollbackResult,
    SelfHealingReport,
};
use crate::performance::recovery::RecoveryManager;

/// A fresh aggregate health snapshot (persisted as health history).
#[tauri::command]
pub async fn recovery_status(
    manager: State<'_, RecoveryManager>,
) -> Result<HealthSnapshot, DatabaseError> {
    manager.status().await
}

/// Combined recovery history: runs, crashes, and the recent journal.
#[tauri::command]
pub async fn recovery_history(
    manager: State<'_, RecoveryManager>,
    limit: Option<u32>,
) -> Result<RecoveryHistory, DatabaseError> {
    manager.history(limit.unwrap_or(50)).await
}

/// The most recent crash reports, newest-first.
#[tauri::command]
pub async fn recovery_crash_reports(
    manager: State<'_, RecoveryManager>,
    limit: Option<u32>,
) -> Result<Vec<CrashReport>, DatabaseError> {
    manager.crash_reports(limit.unwrap_or(20)).await
}

/// The latest recovery checkpoint, if any.
#[tauri::command]
pub async fn recovery_latest_checkpoint(
    manager: State<'_, RecoveryManager>,
) -> Result<Option<RecoveryJournalEntry>, DatabaseError> {
    manager.latest_checkpoint().await
}

/// Runs a self-healing pass on demand (manual counterpart of the
/// background watchdog pass).
#[tauri::command]
pub async fn recovery_self_heal(
    manager: State<'_, RecoveryManager>,
) -> Result<SelfHealingReport, DatabaseError> {
    manager.run_self_healing().await
}

/// Rolls back to the newest valid ancestor of the latest checkpoint
/// (manual rollback; automatic rollback still runs on corrupt
/// checkpoints during startup recovery).
#[tauri::command]
pub async fn recovery_rollback(
    manager: State<'_, RecoveryManager>,
) -> Result<RollbackResult, DatabaseError> {
    manager.rollback_now().await
}

/// The watchdog's current tick counter (diagnostics).
#[tauri::command]
pub async fn recovery_tick(manager: State<'_, RecoveryManager>) -> Result<u64, DatabaseError> {
    Ok(manager.tick_count())
}
