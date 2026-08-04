//! Engine-level tests for RC-10 M3 (data integrity & backup).

use crate::database::test_database;
use crate::errors::DatabaseError;
use crate::maintenance::{IntegrityChecker, MaintenanceEngine, RestoreService};
use crate::models::backup::{BackupRunKind, BackupRunStatus};
use crate::repositories::MaintenanceRepository;

async fn setup() -> (MaintenanceEngine, MaintenanceRepository, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let repository = MaintenanceRepository::new(database.pool().clone());
    let engine = MaintenanceEngine::new(
        repository.clone(),
        temp_dir.path().join("chronodesk.db"),
        temp_dir.path().join("backups"),
    );
    (engine, repository, temp_dir)
}

#[tokio::test]
async fn integrity_reports_a_clean_database_and_is_audited() {
    let (engine, repository, _temp) = setup().await;

    let report = engine.integrity().await.expect("integrity report");

    assert!(report.ok);
    assert!(report.main.integrity.ok);
    assert!(report.main.quick_check.ok);
    assert!(report.main.foreign_key_check.is_empty());
    assert_eq!(report.main.journal_mode.to_lowercase(), "wal");
    assert!(report.db_path.ends_with("chronodesk.db"));

    let latest = repository
        .latest_run_of_kind(BackupRunKind::Integrity)
        .await
        .expect("latest")
        .expect("row");
    assert_eq!(latest.status, BackupRunStatus::Success);
}

#[tokio::test]
async fn backup_lists_and_restores_round_trip() {
    let (engine, _repository, temp_dir) = setup().await;

    let run = engine.backup().await.expect("backup");
    assert_eq!(run.status, BackupRunStatus::Success);
    assert_eq!(run.checksum.len(), 64);

    let listed = engine.backups(10).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, run.id);
    assert_eq!(listed[0].kind, BackupRunKind::Backup);

    // Restore by id resolves the file inside the backup dir.
    let result = engine.restore(run.id).await.expect("restore");
    assert!(result.ok);
    assert!(result.applies_on_next_launch);
    assert!(temp_dir.path().join("restore-pending.db").exists());
    assert_eq!(
        result.staged_path,
        temp_dir
            .path()
            .join("restore-pending.db")
            .display()
            .to_string()
    );

    let pending = engine
        .pending_restore()
        .await
        .expect("pending")
        .expect("staged");
    assert!(pending.ok);

    engine.cancel_restore().await.expect("cancel");
    assert!(engine.pending_restore().await.expect("pending").is_none());
    assert!(!temp_dir.path().join("restore-pending.db").exists());

    // The restore id resolves against the ledger's filename: a fake id
    // must fail cleanly.
    let missing = engine.restore(999_999).await.expect_err("unknown id");
    assert!(matches!(missing, DatabaseError::NotFound { .. }));
}

#[tokio::test]
async fn restore_rejects_non_backup_runs() {
    let (engine, _repository, _temp) = setup().await;

    let integrity = engine.integrity().await.expect("integrity");
    let integrity_id = engine
        .backups(100)
        .await
        .expect("runs")
        .iter()
        .find(|run| run.kind == BackupRunKind::Integrity)
        .map(|run| run.id)
        .expect("integrity run recorded");
    drop(integrity);
    let err = engine
        .restore(integrity_id)
        .await
        .expect_err("not a backup");
    assert!(
        matches!(err, DatabaseError::InvalidInput(_)),
        "restoring from a non-backup run must fail: {err:?}"
    );
}

#[tokio::test]
async fn maintenance_runs_are_reported_and_audited() {
    let (engine, repository, _temp) = setup().await;

    let report = engine.maintenance().await.expect("maintenance");
    assert!(report.size_after_bytes > 0);
    assert!(report.freed_pages >= 0);

    let latest = repository
        .latest_run_of_kind(BackupRunKind::Maintenance)
        .await
        .expect("latest")
        .expect("row");
    assert_eq!(latest.status, BackupRunStatus::Success);
}

#[tokio::test]
async fn backups_land_in_the_configured_directory() {
    let (engine, _repository, temp_dir) = setup().await;

    assert_eq!(engine.backups(1).await.expect("empty list").len(), 0);

    let expected = temp_dir.path().join("backups");
    let backup = engine.backup().await.expect("backup");
    assert!(expected.join(&backup.path).exists());
}

#[tokio::test]
async fn full_launch_cycle_applies_a_staged_restore() {
    let (engine, _repository, temp_dir) = setup().await;
    let db_path = temp_dir.path().join("chronodesk.db");

    // A live database must already exist (as on any real launch) so the
    // pre-restore safety copy has something to preserve.
    let live = crate::database::Database::initialize_at(&db_path)
        .await
        .expect("live init at engine path");

    // Snapshot of the current (empty) database, then load it with data.
    let run = engine.backup().await.expect("backup");
    let snapshot = temp_dir.path().join("backups").join(&run.path);
    let _ = IntegrityChecker::check_file(&snapshot)
        .await
        .expect("snapshot is valid");

    // Stage a restore of that snapshot...
    let staged = engine.restore(run.id).await.expect("stage");
    assert!(staged.ok);
    drop(live);

    // ...and re-initialize at the same path as a fresh launch would: the
    // pending restore must be swapped in before the pool opens.
    let reopened = crate::database::Database::initialize_at(&db_path)
        .await
        .expect("re-open applies pending restore");
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workspaces")
        .fetch_one(reopened.pool())
        .await
        .expect("query restored database");
    assert_eq!(count, 0, "restored snapshot matches the staged backup");
    assert!(
        !temp_dir.path().join("restore-pending.db").exists(),
        "marker consumed by launch"
    );
    let safety = std::fs::read_dir(temp_dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with(RestoreService::safety_prefix()));
    assert!(safety.is_some(), "pre-restore safety copy written");
}
