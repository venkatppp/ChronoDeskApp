//! Data Integrity & Backup repository (RC-10 M3).
//!
//! Owns the SQL behind the backup/integrity/maintenance surfaces: the
//! `backup_runs` audit ledger (one row per backup, staged restore,
//! integrity check and maintenance run) and the maintenance *statements*
//! (`VACUUM`, `VACUUM INTO`, `PRAGMA optimize`, `wal_checkpoint(TRUNCATE)`).
//!
//! The diagnostic `PRAGMA` battery (`integrity_check`, `quick_check`,
//! `foreign_key_check`, page stats) intentionally does NOT live here: it
//! must run against arbitrary read-only backup files as well as the live
//! pool, which a single-pool repository cannot express. The battery lives
//! in [`crate::maintenance::integrity`] as the single owner of those
//! read-only queries. Policy — when to vacuum, how to stage a restore —
//! lives in [`crate::maintenance`].

use std::path::Path;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::errors::DatabaseError;
use crate::models::backup::{BackupRun, BackupRunKind, BackupRunStatus};

/// Raw `backup_runs` row.
type RunRow = (
    i64,
    String,
    String,
    String,
    i64,
    String,
    String,
    i64,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// Repository for the RC-10 M3 backup/integrity/maintenance surfaces.
#[derive(Debug, Clone)]
pub struct MaintenanceRepository {
    pool: SqlitePool,
}

impl MaintenanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// The pool for this repository, so the maintenance services can run
    /// statements against the live database.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ------------------------------------------------------------------
    // Maintenance statements
    // ------------------------------------------------------------------

    /// `PRAGMA optimize` — runs `ANALYZE`-style statistics collection when
    /// the schema or data has changed enough to be worth it (a no-op
    /// otherwise).
    pub async fn optimize(&self) -> Result<(), DatabaseError> {
        sqlx::query("PRAGMA optimize").execute(&self.pool).await?;
        Ok(())
    }

    /// `PRAGMA wal_checkpoint(TRUNCATE)` — forces the WAL into the main
    /// database file, returning (busy, log_frames, checkpointed_frames).
    pub async fn wal_checkpoint_truncate(&self) -> Result<(i32, i32, i32), DatabaseError> {
        let (busy, log, checkpointed): (i32, i32, i32) =
            sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
                .fetch_one(&self.pool)
                .await?;
        Ok((busy, log, checkpointed))
    }

    /// `VACUUM` — rewrites the database file to reclaim free pages.
    /// Callers should gate this on the free-page ratio (see
    /// [`crate::maintenance::MaintenanceRunner::run`]).
    pub async fn vacuum(&self) -> Result<(), DatabaseError> {
        sqlx::query("VACUUM").execute(&self.pool).await?;
        Ok(())
    }

    /// `VACUUM INTO '<path>'` — produces a consistent, compacted snapshot
    /// of the live database into a new file without disturbing the running
    /// database (an online backup; SQLite itself serializes against active
    /// writers).
    pub async fn vacuum_into(&self, dest: &Path) -> Result<(), DatabaseError> {
        validate_backup_path(dest)?;
        sqlx::query(&format!(
            "VACUUM INTO '{}'",
            quote_sql_string(&dest.display().to_string())
        ))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Audit ledger (`backup_runs`)
    // ------------------------------------------------------------------

    /// Records one ledger row, returning its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_run(
        &self,
        kind: BackupRunKind,
        status: BackupRunStatus,
        path: &str,
        size_bytes: i64,
        checksum: &str,
        detail: &str,
        duration_ms: i64,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO backup_runs
               (kind, status, path, size_bytes, checksum, detail, duration_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(kind.as_str())
        .bind(status.as_str())
        .bind(path)
        .bind(size_bytes)
        .bind(checksum)
        .bind(detail)
        .bind(duration_ms)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// Most recent ledger rows, newest-first.
    pub async fn recent_runs(&self, limit: u32) -> Result<Vec<BackupRun>, DatabaseError> {
        let rows: Vec<RunRow> = sqlx::query_as(
            "SELECT id, kind, status, path, size_bytes, checksum, detail,
                    duration_ms, started_at, completed_at
             FROM backup_runs ORDER BY completed_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::from_row).collect())
    }

    /// One ledger row by id, if it exists.
    pub async fn run_by_id(&self, id: i64) -> Result<Option<BackupRun>, DatabaseError> {
        let row: Option<RunRow> = sqlx::query_as(
            "SELECT id, kind, status, path, size_bytes, checksum, detail,
                    duration_ms, started_at, completed_at
             FROM backup_runs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::from_row))
    }

    /// The most recent run of the given kind, if any.
    pub async fn latest_run_of_kind(
        &self,
        kind: BackupRunKind,
    ) -> Result<Option<BackupRun>, DatabaseError> {
        let row: Option<RunRow> = sqlx::query_as(
            "SELECT id, kind, status, path, size_bytes, checksum, detail,
                    duration_ms, started_at, completed_at
             FROM backup_runs WHERE kind = ?
             ORDER BY completed_at DESC, id DESC LIMIT 1",
        )
        .bind(kind.as_str())
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::from_row))
    }

    fn from_row(row: RunRow) -> BackupRun {
        BackupRun {
            id: row.0,
            kind: BackupRunKind::from(row.1.as_str()),
            status: BackupRunStatus::from(row.2.as_str()),
            path: row.3,
            size_bytes: row.4,
            checksum: row.5,
            detail: row.6,
            duration_ms: row.7,
            started_at: row.8,
            completed_at: row.9,
        }
    }
}

/// Validates a backup destination path before interpolating it into
/// `VACUUM INTO`: must be absolute (never relative to the database's own
/// directory) and free of characters that could break out of the SQL
/// literal (single quotes are escaped, NUL is rejected outright).
fn validate_backup_path(path: &Path) -> Result<(), DatabaseError> {
    if path.as_os_str().to_string_lossy().contains('\0') {
        return Err(DatabaseError::InvalidInput(
            "backup path contains a NUL byte".to_string(),
        ));
    }
    if !path.is_absolute() {
        return Err(DatabaseError::InvalidInput(
            "backup path must be absolute".to_string(),
        ));
    }
    Ok(())
}

/// Escapes a value for use inside a single-quoted SQLite literal.
fn quote_sql_string(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn optimize_and_checkpoint_are_no_ops_on_clean_database() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());

        repository.optimize().await.expect("optimize");
        let (busy, log, checkpointed) = repository
            .wal_checkpoint_truncate()
            .await
            .expect("checkpoint");
        assert!(busy == 0, "no busy readers expected: {busy}");
        assert!(log >= 0 && checkpointed >= 0);
    }

    #[tokio::test]
    async fn vacuum_into_produces_a_valid_compact_file() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let dest = _temp.path().join("snapshot.db");

        repository.vacuum_into(&dest).await.expect("vacuum into");
        assert!(dest.exists(), "snapshot file should be created");
        assert!(dest.metadata().expect("metadata").len() > 0);

        let snapshot_pool = crate::database::connection::create_pool(&dest)
            .await
            .expect("snapshot opens as a valid sqlite database");
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&snapshot_pool)
            .await
            .expect("query snapshot");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn vacuum_into_rejects_relative_and_nul_paths() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());

        let relative = Path::new("snapshot.db");
        assert!(
            repository.vacuum_into(relative).await.is_err(),
            "relative paths must be rejected"
        );
        let nul = _temp.path().join("snap\0shot.db");
        assert!(
            repository.vacuum_into(&nul).await.is_err(),
            "NUL bytes must be rejected"
        );
    }

    #[tokio::test]
    async fn record_run_and_recent_runs_round_trip() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());

        let first = repository
            .record_run(
                BackupRunKind::Backup,
                BackupRunStatus::Success,
                "chronodesk-20260804T010203000000Z.db",
                1234,
                "deadbeef",
                "created",
                42,
            )
            .await
            .expect("record backup");
        let second = repository
            .record_run(
                BackupRunKind::Restore,
                BackupRunStatus::Staged,
                "chronodesk-20260804T010203000000Z.db",
                0,
                "deadbeef",
                "staged for next launch",
                7,
            )
            .await
            .expect("record restore");
        let failed = repository
            .record_run(
                BackupRunKind::Backup,
                BackupRunStatus::Failed,
                "",
                0,
                "",
                "disk full",
                1,
            )
            .await
            .expect("record failure");

        let runs = repository.recent_runs(10).await.expect("recent runs");
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].id, failed, "newest first");
        assert_eq!(runs[0].kind, BackupRunKind::Backup);
        assert_eq!(runs[0].status, BackupRunStatus::Failed);
        assert_eq!(runs[1].id, second);
        assert_eq!(runs[1].kind, BackupRunKind::Restore);
        assert_eq!(runs[1].status, BackupRunStatus::Staged);
        assert_eq!(runs[1].path, "chronodesk-20260804T010203000000Z.db");
        assert_eq!(runs[2].id, first);
        assert_eq!(runs[2].size_bytes, 1234);
        assert_eq!(runs[2].checksum, "deadbeef");
        assert_eq!(runs[2].duration_ms, 42);

        let limited = repository.recent_runs(2).await.expect("limited");
        assert_eq!(limited.len(), 2, "limit is honored");

        let by_id = repository
            .run_by_id(first)
            .await
            .expect("by id")
            .expect("row");
        assert_eq!(by_id.kind, BackupRunKind::Backup);
        assert!(
            repository
                .run_by_id(999_999)
                .await
                .expect("missing")
                .is_none(),
            "unknown ids return None"
        );
    }

    #[tokio::test]
    async fn latest_run_of_kind_filters_and_orders() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());

        repository
            .record_run(
                BackupRunKind::Integrity,
                BackupRunStatus::Success,
                "",
                0,
                "",
                "first",
                1,
            )
            .await
            .expect("first integrity");
        repository
            .record_run(
                BackupRunKind::Backup,
                BackupRunStatus::Success,
                "a.db",
                10,
                "",
                "backup",
                1,
            )
            .await
            .expect("backup");
        repository
            .record_run(
                BackupRunKind::Integrity,
                BackupRunStatus::Success,
                "",
                0,
                "",
                "second",
                1,
            )
            .await
            .expect("second integrity");

        let latest = repository
            .latest_run_of_kind(BackupRunKind::Integrity)
            .await
            .expect("latest integrity")
            .expect("some run");
        assert_eq!(latest.detail, "second");

        let none = repository
            .latest_run_of_kind(BackupRunKind::Restore)
            .await
            .expect("latest restore");
        assert!(none.is_none(), "no restore rows recorded yet");
    }

    #[test]
    fn quote_sql_string_escapes_single_quotes() {
        assert_eq!(quote_sql_string("plain"), "plain");
        assert_eq!(quote_sql_string("O'Brien"), "O''Brien");
        assert_eq!(quote_sql_string("a'b'c"), "a''b''c");
    }
}
