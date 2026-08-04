//! Maintenance runner (RC-10 M3).
//!
//! The safe maintenance pass: checkpoint the WAL into the main file,
//! `VACUUM` only when the free-page ratio justifies a full file rewrite
//! (a rewrite of a large database is not something to do on every click),
//! then `PRAGMA optimize` for statistics. Every run is measured
//! (before/after free pages and file size) and recorded in the
//! `backup_runs` ledger.

use chrono::Utc;
use std::time::Instant;

use crate::errors::DatabaseError;
use crate::maintenance::IntegrityChecker;
use crate::models::backup::{BackupRunKind, BackupRunStatus, MaintenanceReport};
use crate::repositories::MaintenanceRepository;

/// A full `VACUUM` (file rewrite) runs only when the free pages exceed
/// both a hard floor and a share of the file — see [`should_vacuum`].
const VACUUM_MIN_FREE_PAGES: i64 = 64;
/// Free pages must be at least this share of the file before a rewrite.
const VACUUM_MIN_RATIO: f64 = 0.10;

/// Runs the maintenance pass against the live database.
#[derive(Debug, Clone)]
pub struct MaintenanceRunner {
    repository: MaintenanceRepository,
}

impl MaintenanceRunner {
    pub fn new(repository: MaintenanceRepository) -> Self {
        Self { repository }
    }

    /// Checkpoint → (maybe) VACUUM → optimize, measured and audited.
    pub async fn run(&self) -> Result<MaintenanceReport, DatabaseError> {
        let started = Instant::now();
        let before = IntegrityChecker::page_stats(self.repository.pool()).await?;

        let (_, _, checkpointed_frames) = self.repository.wal_checkpoint_truncate().await?;

        let vacuum_ran = should_vacuum(before.freelist_count, before.page_count);
        if vacuum_ran {
            self.repository.vacuum().await?;
        }
        self.repository.optimize().await?;

        let after = IntegrityChecker::page_stats(self.repository.pool()).await?;
        let freed_pages = (before.freelist_count - after.freelist_count).max(0);
        let recovered_bytes = (before.page_count - after.page_count).max(0) * before.page_size;
        let report = MaintenanceReport {
            checked_at: Utc::now(),
            freelist_before: before.freelist_count,
            freelist_after: after.freelist_count,
            freed_pages,
            size_before_bytes: before.size_bytes(),
            size_after_bytes: after.size_bytes(),
            recovered_bytes,
            vacuum_ran,
            checkpointed_frames: checkpointed_frames as i64,
        };

        let detail = format!(
            "vacuum={vacuum_ran} freed_pages={freed_pages} checkpointed_frames={checkpointed_frames}"
        );
        let _ = self
            .repository
            .record_run(
                BackupRunKind::Maintenance,
                BackupRunStatus::Success,
                "",
                0,
                "",
                &detail,
                started.elapsed().as_millis() as i64,
            )
            .await;

        Ok(report)
    }
}

/// Policy: only rewrite the file when the free-page share is worth it.
pub fn should_vacuum(freelist_count: i64, page_count: i64) -> bool {
    let share = if page_count > 0 {
        freelist_count as f64 / page_count as f64
    } else {
        0.0
    };
    freelist_count >= VACUUM_MIN_FREE_PAGES && share >= VACUUM_MIN_RATIO
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn maintenance_pass_reports_and_is_audited() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let runner = MaintenanceRunner::new(repository.clone());

        let report = runner.run().await.expect("maintenance run");

        assert!(report.freelist_before >= 0);
        assert!(report.freelist_after >= 0);
        assert!(report.freed_pages >= 0);
        assert!(report.size_after_bytes > 0);
        assert!(report.checkpointed_frames >= 0);
        assert_eq!(report.checked_at.date_naive(), Utc::now().date_naive());

        let latest = repository
            .latest_run_of_kind(BackupRunKind::Maintenance)
            .await
            .expect("latest")
            .expect("row");
        assert_eq!(latest.status, BackupRunStatus::Success);
        assert!(latest.detail.contains("vacuum="), "{}", latest.detail);
    }

    #[tokio::test]
    async fn maintenance_keeps_the_database_healthy() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let runner = MaintenanceRunner::new(repository.clone());

        runner.run().await.expect("maintenance run");
        let checks = IntegrityChecker::new(repository.clone(), "/tmp/chronodesk.db")
            .check_live()
            .await
            .expect("battery");
        assert!(
            checks.integrity.ok,
            "database still passes integrity after maintenance"
        );
    }

    #[test]
    fn vacuum_policy_gates_on_free_page_share() {
        assert!(!should_vacuum(0, 1_000));
        assert!(!should_vacuum(63, 1_000), "below the page floor");
        assert!(!should_vacuum(64, 10_000), "below the ratio gate");
        assert!(should_vacuum(1_000, 10_000), "both gates satisfied");
        assert!(should_vacuum(500, 5_000));
        assert!(!should_vacuum(1_000, 0), "no pages to rewrite");
    }
}
