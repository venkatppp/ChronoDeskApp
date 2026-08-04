//! Restore service (RC-10 M3).
//!
//! Restores are **staged**, never applied live: swapping a database file
//! while its pool has open connections would corrupt the running session.
//! A restore therefore
//! 1. validates the backup file (SQLite header + read-only `quick_check`
//!    and `foreign_key_check` — the same queries that certify the live
//!    database),
//! 2. copies the validated file to the `restore-pending.db` marker next
//!    to the live database,
//! 3. records the stage in the `backup_runs` ledger, and
//! 4. is swapped in by [`crate::database::Database::initialize_at`] on
//!    the next launch, before any connection opens (the previous database
//!    is preserved as a `chronodesk-pre-restore-*.db` safety copy).
//!
//! Staging can be cancelled at any time; a staged restore is reported by
//! [`RestoreService::pending`].

use std::path::{Path, PathBuf};

use std::time::Instant;

use crate::database::{PRE_RESTORE_BACKUP_PREFIX, RESTORE_PENDING_FILE};
use crate::errors::DatabaseError;
use crate::hashing::HashingService;
use crate::maintenance::IntegrityChecker;
use crate::models::backup::{BackupRunKind, BackupRunStatus, RestoreResult};
use crate::repositories::MaintenanceRepository;

/// The first 16 bytes of every SQLite database file.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// Stages a validated backup for restore on the next launch.
#[derive(Debug, Clone)]
pub struct RestoreService {
    repository: MaintenanceRepository,
    db_path: PathBuf,
    hashing: HashingService,
}

impl RestoreService {
    pub fn new(repository: MaintenanceRepository, db_path: PathBuf) -> Self {
        Self {
            repository,
            db_path,
            hashing: HashingService::new(),
        }
    }

    /// The live database path (defines where the marker sits).
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// The pending-restore marker path.
    pub fn pending_path(&self) -> PathBuf {
        self.db_path.with_file_name(RESTORE_PENDING_FILE)
    }

    /// Validates `backup_path` and copies it to the pending marker.
    pub async fn stage(&self, backup_path: &Path) -> Result<RestoreResult, DatabaseError> {
        let started = Instant::now();
        let filename = backup_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| backup_path.display().to_string());

        let header = tokio::fs::read(backup_path)
            .await
            .map_err(|e| DatabaseError::IoError(e.to_string()))?;
        if header.len() < SQLITE_HEADER.len() || !is_sqlite_header(&header[..SQLITE_HEADER.len()]) {
            let _ = self
                .repository
                .record_run(
                    BackupRunKind::Restore,
                    BackupRunStatus::Failed,
                    &filename,
                    0,
                    "",
                    "rejected: not a sqlite database file",
                    started.elapsed().as_millis() as i64,
                )
                .await;
            return Err(DatabaseError::InvalidInput(format!(
                "{filename} is not a sqlite database file"
            )));
        }

        let validated = IntegrityChecker::check_file(backup_path).await?;
        if !validated.quick_check.ok || !validated.foreign_key_check.is_empty() {
            let reason = if !validated.quick_check.ok {
                "backup failed the integrity check"
            } else {
                "backup has foreign-key violations"
            };
            let _ = self
                .repository
                .record_run(
                    BackupRunKind::Restore,
                    BackupRunStatus::Failed,
                    &filename,
                    validated.database_size_bytes,
                    "",
                    &format!("rejected: {reason}"),
                    started.elapsed().as_millis() as i64,
                )
                .await;
            return Err(DatabaseError::InvalidInput(format!("{filename} {reason}")));
        }

        let marker = self.pending_path();
        tokio::fs::copy(backup_path, &marker)
            .await
            .map_err(|e| DatabaseError::IoError(e.to_string()))?;

        let checksum = self
            .hashing
            .hash_file(backup_path)
            .map_err(|e| DatabaseError::IoError(e.to_string()))?;
        self.repository
            .record_run(
                BackupRunKind::Restore,
                BackupRunStatus::Staged,
                &filename,
                validated.database_size_bytes,
                &checksum,
                "validated and staged for next launch",
                started.elapsed().as_millis() as i64,
            )
            .await?;

        Ok(RestoreResult {
            ok: true,
            message: "validated — applies on next launch".to_string(),
            backup_path: backup_path.display().to_string(),
            staged_path: marker.display().to_string(),
            applies_on_next_launch: true,
            validated,
        })
    }

    /// Reports whether a staged restore is waiting to be applied.
    pub async fn pending(&self) -> Result<Option<RestoreResult>, DatabaseError> {
        let marker = self.pending_path();
        if !marker.exists() {
            return Ok(None);
        }
        let validated = IntegrityChecker::check_file(&marker).await?;
        let ok = validated.quick_check.ok && validated.foreign_key_check.is_empty();
        Ok(Some(RestoreResult {
            ok,
            message: if ok {
                "restore staged — restart to apply".to_string()
            } else {
                "staged copy failed validation — restarting will ignore it".to_string()
            },
            backup_path: String::new(),
            staged_path: marker.display().to_string(),
            applies_on_next_launch: ok,
            validated,
        }))
    }

    /// Discards a staged restore (no-op when nothing is staged).
    pub async fn cancel(&self) -> Result<(), DatabaseError> {
        let marker = self.pending_path();
        if marker.exists() {
            for suffix in ["-wal", "-shm"] {
                let sidecar = PathBuf::from(format!("{}{suffix}", marker.display()));
                let _ = tokio::fs::remove_file(&sidecar).await;
            }
            tokio::fs::remove_file(&marker)
                .await
                .map_err(|e| DatabaseError::IoError(e.to_string()))?;
        }
        self.repository
            .record_run(
                BackupRunKind::Restore,
                BackupRunStatus::Success,
                "",
                0,
                "",
                "cancelled before launch",
                0,
            )
            .await?;
        Ok(())
    }

    /// The pre-restore safety backup prefix, for UI copy.
    pub fn safety_prefix() -> &'static str {
        PRE_RESTORE_BACKUP_PREFIX
    }
}

/// True when `bytes` begins with the SQLite file magic.
pub fn is_sqlite_header(bytes: &[u8]) -> bool {
    bytes.len() >= SQLITE_HEADER.len() && bytes[..SQLITE_HEADER.len()] == *SQLITE_HEADER
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn stage_validates_copies_and_records() {
        let (database, temp_dir) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let db_path = temp_dir.path().join("chronodesk.db");

        // Produce a backup of a live database into the backup dir.
        let backup_dir = temp_dir.path().join("backups");
        let backup = crate::maintenance::BackupService::new(repository.clone(), backup_dir.clone());
        let run = backup.create().await.expect("backup");
        let backup_file = backup_dir.join(&run.path);

        let restore = RestoreService::new(repository.clone(), db_path);
        let result = restore.stage(&backup_file).await.expect("stage");

        assert!(result.ok);
        assert!(result.applies_on_next_launch);
        assert!(result.validated.quick_check.ok);
        let marker = restore.pending_path();
        assert!(marker.exists(), "marker must be copied");

        let pending = restore.pending().await.expect("pending").expect("staged");
        assert!(pending.ok);
        assert!(pending.applies_on_next_launch);
        assert_eq!(pending.staged_path, marker.display().to_string());

        let staged_run = repository
            .latest_run_of_kind(BackupRunKind::Restore)
            .await
            .expect("latest")
            .expect("row");
        assert_eq!(staged_run.status, BackupRunStatus::Staged);
        assert_eq!(
            staged_run.path, run.path,
            "ledger names the staged backup file"
        );
    }

    #[tokio::test]
    async fn stage_rejects_non_sqlite_and_invalid_files() {
        let (database, temp_dir) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let restore =
            RestoreService::new(repository.clone(), temp_dir.path().join("chronodesk.db"));

        let junk = temp_dir.path().join("junk.db");
        std::fs::write(&junk, b"not a database").expect("write junk");
        let err = restore.stage(&junk).await.expect_err("junk must fail");
        assert!(matches!(err, DatabaseError::InvalidInput(_)), "{err:?}");
        assert!(
            !restore.pending_path().exists(),
            "no marker may be left behind by a failed stage"
        );

        let failed_run = repository
            .latest_run_of_kind(BackupRunKind::Restore)
            .await
            .expect("latest")
            .expect("row");
        assert_eq!(failed_run.status, BackupRunStatus::Failed);
        assert!(
            failed_run.detail.contains("not a sqlite"),
            "{}",
            failed_run.detail
        );
    }

    #[tokio::test]
    async fn cancel_removes_the_marker_and_records() {
        let (database, temp_dir) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let backup_dir = temp_dir.path().join("backups");
        let backup = crate::maintenance::BackupService::new(repository.clone(), backup_dir.clone());
        let run = backup.create().await.expect("backup");

        let restore =
            RestoreService::new(repository.clone(), temp_dir.path().join("chronodesk.db"));
        restore
            .stage(&backup_dir.join(&run.path))
            .await
            .expect("stage");
        assert!(restore.pending_path().exists());

        restore.cancel().await.expect("cancel");
        assert!(!restore.pending_path().exists(), "marker removed by cancel");
        assert!(
            restore.pending().await.expect("pending").is_none(),
            "nothing staged after cancel"
        );

        let latest = repository
            .latest_run_of_kind(BackupRunKind::Restore)
            .await
            .expect("latest")
            .expect("row");
        assert_eq!(latest.status, BackupRunStatus::Success);
        assert!(latest.detail.contains("cancelled"), "{}", latest.detail);
    }

    #[test]
    fn sqlite_header_detection() {
        let real = b"SQLite format 3\x00";
        assert!(is_sqlite_header(real));
        assert!(!is_sqlite_header(b"SQLite"));
        assert!(!is_sqlite_header(b"not sqlite format 3\x00"));
        assert!(!is_sqlite_header(b""), "empty input is not sqlite");
    }

    #[tokio::test]
    async fn pending_path_sits_next_to_the_database() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("chronodesk.db");
        let repository = MaintenanceRepository::new(
            sqlx::SqlitePool::connect("sqlite::memory:")
                .await
                .expect("memory pool"),
        );
        let restore = RestoreService::new(repository, db_path);
        let pending = restore.pending_path();
        assert_eq!(pending.file_name().expect("name"), RESTORE_PENDING_FILE);
        assert_eq!(pending.parent(), Some(temp_dir.path()));
    }
}
