//! Backup service (RC-10 M3).
//!
//! Produces consistent, compacted snapshots of the live database via
//! `VACUUM INTO` (SQLite's online backup: the copy reflects a single
//! consistent database state even while writers are active, and the
//! output file has no WAL sidecars — it is a standalone database file).
//! Every snapshot is SHA-256-hashed with the shared
//! [`crate::hashing::HashingService`] and recorded in the `backup_runs`
//! audit ledger so a later restore can be verified against the checksum.

use std::path::PathBuf;

use chrono::Utc;
use std::time::Instant;

use crate::errors::DatabaseError;
use crate::hashing::HashingService;
use crate::models::backup::{BackupRun, BackupRunKind, BackupRunStatus};
use crate::repositories::MaintenanceRepository;

/// Backs up the live database into the configured backup directory.
#[derive(Debug, Clone)]
pub struct BackupService {
    repository: MaintenanceRepository,
    backup_dir: PathBuf,
    hashing: HashingService,
}

impl BackupService {
    pub fn new(repository: MaintenanceRepository, backup_dir: PathBuf) -> Self {
        Self {
            repository,
            backup_dir,
            hashing: HashingService::new(),
        }
    }

    /// The directory backups are written to.
    pub fn backup_dir(&self) -> &PathBuf {
        &self.backup_dir
    }

    /// Creates a snapshot: `chronodesk-<timestamp>.db` in the backup
    /// directory. Records the run in the audit ledger — a `success` row on
    /// completion, a `failed` row when the snapshot could not be produced.
    pub async fn create(&self) -> Result<BackupRun, DatabaseError> {
        let started_at = Utc::now();
        let started = Instant::now();
        tokio::fs::create_dir_all(&self.backup_dir)
            .await
            .map_err(|e| DatabaseError::IoError(e.to_string()))?;

        let filename = format!("chronodesk-{}.db", started_at.format("%Y%m%dT%H%M%S%.6fZ"));
        let dest = self.backup_dir.join(&filename);

        match self.repository.vacuum_into(&dest).await {
            Ok(()) => {
                let duration_ms = started.elapsed().as_millis() as i64;
                let metadata = tokio::fs::metadata(&dest)
                    .await
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?;
                let checksum = self
                    .hashing
                    .hash_file(&dest)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?;
                let id = self
                    .repository
                    .record_run(
                        BackupRunKind::Backup,
                        BackupRunStatus::Success,
                        &filename,
                        metadata.len() as i64,
                        &checksum,
                        "snapshot created",
                        duration_ms,
                    )
                    .await?;
                Ok(BackupRun {
                    id,
                    kind: BackupRunKind::Backup,
                    status: BackupRunStatus::Success,
                    path: filename,
                    size_bytes: metadata.len() as i64,
                    checksum,
                    detail: "snapshot created".to_string(),
                    duration_ms,
                    started_at,
                    completed_at: Utc::now(),
                })
            }
            Err(error) => {
                let _ = self
                    .repository
                    .record_run(
                        BackupRunKind::Backup,
                        BackupRunStatus::Failed,
                        &filename,
                        0,
                        "",
                        &format!("backup failed: {error}"),
                        started.elapsed().as_millis() as i64,
                    )
                    .await;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn backup_creates_a_valid_checksummed_snapshot_and_records_it() {
        let (database, temp_dir) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let service = BackupService::new(repository.clone(), temp_dir.path().join("backups"));

        let run = service.create().await.expect("backup");

        assert_eq!(run.kind, BackupRunKind::Backup);
        assert_eq!(run.status, BackupRunStatus::Success);
        assert!(run.path.ends_with(".db"));
        assert!(run.size_bytes > 0);
        assert_eq!(run.checksum.len(), 64, "sha256 hex");
        assert!(run.duration_ms >= 0);

        let snapshot = temp_dir.path().join("backups").join(&run.path);
        assert!(snapshot.exists());
        assert_eq!(
            run.size_bytes,
            snapshot.metadata().expect("metadata").len() as i64
        );
        assert_eq!(
            run.checksum,
            HashingService::new().hash_file(&snapshot).expect("rehash"),
            "ledger checksum matches the file on disk"
        );

        let latest = repository
            .latest_run_of_kind(BackupRunKind::Backup)
            .await
            .expect("latest")
            .expect("row");
        assert_eq!(latest.id, run.id);
    }

    #[tokio::test]
    async fn backup_creates_the_backup_directory_when_missing() {
        let (database, temp_dir) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let nested = temp_dir.path().join("a").join("b");
        let service = BackupService::new(repository.clone(), nested.clone());

        let run = service.create().await.expect("backup");
        assert!(
            nested.join(&run.path).exists(),
            "backup dir created on demand"
        );
    }

    #[tokio::test]
    async fn snapshot_reflects_live_data() {
        let (database, temp_dir) = test_database().await;
        sqlx::query(
            "INSERT INTO workspaces (id, name, created_at, updated_at, last_active_at)
             VALUES ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'backup-me', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        )
        .execute(database.pool())
        .await
        .expect("insert workspace");

        let repository = MaintenanceRepository::new(database.pool().clone());
        let service = BackupService::new(repository.clone(), temp_dir.path().join("backups"));
        let run = service.create().await.expect("backup");

        let snapshot_pool = crate::database::connection::create_pool(
            &temp_dir.path().join("backups").join(&run.path),
        )
        .await
        .expect("open snapshot");
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&snapshot_pool)
            .await
            .expect("snapshot contains the live data");
        assert_eq!(count, 1, "backup must include data written before it ran");
    }
}
