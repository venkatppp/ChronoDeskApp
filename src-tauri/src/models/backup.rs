//! Data Integrity & Backup models (RC-10 M3).
//!
//! DTOs for the backup/introspection surfaces: integrity check reports
//! (the `PRAGMA` battery), backup runs from the audit ledger, staged-restore
//! results and maintenance (VACUUM / `PRAGMA optimize`) reports. Everything
//! here is a plain serializable DTO — the SQL lives in
//! [`crate::repositories::MaintenanceRepository`] and the policy logic in
//! [`crate::maintenance`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ----------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------

/// What kind of intervention a `backup_runs` ledger row records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupRunKind {
    /// A produced backup snapshot.
    Backup,
    /// A staged pending restore.
    Restore,
    /// An integrity check run.
    Integrity,
    /// A maintenance (VACUUM / optimize / checkpoint) run.
    Maintenance,
}

impl BackupRunKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackupRunKind::Backup => "backup",
            BackupRunKind::Restore => "restore",
            BackupRunKind::Integrity => "integrity",
            BackupRunKind::Maintenance => "maintenance",
        }
    }
}

impl From<&str> for BackupRunKind {
    fn from(value: &str) -> Self {
        match value {
            "restore" => BackupRunKind::Restore,
            "integrity" => BackupRunKind::Integrity,
            "maintenance" => BackupRunKind::Maintenance,
            _ => BackupRunKind::Backup,
        }
    }
}

/// Terminal state of a `backup_runs` ledger row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupRunStatus {
    /// The operation completed successfully.
    Success,
    /// The operation failed; `detail` carries the reason.
    Failed,
    /// A restore has been validated and staged for the next launch.
    Staged,
}

impl BackupRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BackupRunStatus::Success => "success",
            BackupRunStatus::Failed => "failed",
            BackupRunStatus::Staged => "staged",
        }
    }
}

impl From<&str> for BackupRunStatus {
    fn from(value: &str) -> Self {
        match value {
            "failed" => BackupRunStatus::Failed,
            "staged" => BackupRunStatus::Staged,
            _ => BackupRunStatus::Success,
        }
    }
}

// ----------------------------------------------------------------------
// Backup run (audit ledger row)
// ----------------------------------------------------------------------

/// One row of the `backup_runs` audit ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRun {
    pub id: i64,
    pub kind: BackupRunKind,
    pub status: BackupRunStatus,
    /// Produced backup filename / staged restore target / empty for in-place ops.
    pub path: String,
    pub size_bytes: i64,
    /// SHA-256 hex of the backup file (empty for in-place ops).
    pub checksum: String,
    pub detail: String,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Integrity checks
// ----------------------------------------------------------------------

/// The parsed result of a single `PRAGMA integrity_check` / `quick_check`
/// run: one line per page scanned plus the "ok" verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityLines {
    pub ok: bool,
    pub lines: Vec<String>,
}

/// The full `PRAGMA` battery for the main database file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityChecks {
    pub database_size_bytes: i64,
    pub page_count: i64,
    pub page_size: i64,
    pub freelist_count: i64,
    pub journal_mode: String,
    pub integrity: IntegrityLines,
    pub quick_check: IntegrityLines,
    pub foreign_key_check: Vec<String>,
}

/// Complete integrity report for a database file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub checked_at: DateTime<Utc>,
    pub db_path: String,
    pub main: IntegrityChecks,
    pub ok: bool,
}

// ----------------------------------------------------------------------
// Restore
// ----------------------------------------------------------------------

/// Result of a staged restore. The validated backup is copied to a
/// pending-restore marker; it is swapped in by `Database::initialize` on the
/// next launch before any connection is opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub ok: bool,
    pub message: String,
    pub backup_path: String,
    pub staged_path: String,
    pub applies_on_next_launch: bool,
    pub validated: IntegrityChecks,
}

// ----------------------------------------------------------------------
// Maintenance
// ----------------------------------------------------------------------

/// Result of a maintenance run (discard-page free, size delta).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceReport {
    pub checked_at: DateTime<Utc>,
    pub freelist_before: i64,
    pub freelist_after: i64,
    pub freed_pages: i64,
    pub size_before_bytes: i64,
    pub size_after_bytes: i64,
    pub recovered_bytes: i64,
    /// Whether the full `VACUUM` (file rewrite) ran, as opposed to only
    /// the WAL checkpoint + `PRAGMA optimize`.
    pub vacuum_ran: bool,
    /// WAL frames moved into the main file by the checkpoint.
    pub checkpointed_frames: i64,
}
