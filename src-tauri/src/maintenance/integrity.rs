//! Integrity checker (RC-10 M3).
//!
//! The single owner of the diagnostic `PRAGMA` battery: `integrity_check`,
//! `quick_check`, `foreign_key_check`, and the page statistics
//! (`page_count`, `page_size`, `freelist_count`) and journal mode. The
//! battery runs against two kinds of connection:
//! - the live pool (the integrity-check command), and
//! - a read-only, immutable connection over a backup file (restore
//!   validation), which the single-pool
//!   [`crate::repositories::MaintenanceRepository`] cannot express.
//!
//! Both paths parse through the same helpers, so a backup is validated
//! with exactly the same queries that certify the live database.

use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::errors::DatabaseError;
use crate::models::backup::{IntegrityChecks, IntegrityLines};
use crate::repositories::MaintenanceRepository;

/// Size/count snapshot of a database file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageStats {
    pub page_count: i64,
    pub page_size: i64,
    pub freelist_count: i64,
}

impl PageStats {
    /// File size in bytes (`page_count × page_size`, free pages included).
    pub fn size_bytes(&self) -> i64 {
        self.page_count.max(0).saturating_mul(self.page_size.max(0))
    }

    /// Ratio of free pages to total pages (0.0..=1.0).
    pub fn freelist_ratio(&self) -> f64 {
        let total = self.page_count.max(1) as f64;
        (self.freelist_count.max(0) as f64) / total
    }
}

/// Runs the `PRAGMA` battery against the live database.
#[derive(Debug, Clone)]
pub struct IntegrityChecker {
    repository: MaintenanceRepository,
    db_path: String,
}

impl IntegrityChecker {
    pub fn new(repository: MaintenanceRepository, db_path: impl Into<String>) -> Self {
        Self {
            repository,
            db_path: db_path.into(),
        }
    }

    /// The path of the live database, for reports.
    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    /// The full battery against the live pool.
    pub async fn check_live(&self) -> Result<IntegrityChecks, DatabaseError> {
        let stats = Self::page_stats(self.repository.pool()).await?;
        let journal_mode = Self::journal_mode(self.repository.pool()).await?;
        let integrity = Self::check_lines(self.repository.pool(), false).await?;
        let quick_check = Self::check_lines(self.repository.pool(), true).await?;
        let foreign_key_check = Self::foreign_key_violations(self.repository.pool()).await?;
        Ok(IntegrityChecks {
            database_size_bytes: stats.size_bytes(),
            page_count: stats.page_count,
            page_size: stats.page_size,
            freelist_count: stats.freelist_count,
            journal_mode,
            integrity,
            quick_check,
            foreign_key_check,
        })
    }

    /// The read-only battery over a backup file (no journal-mode probe:
    /// immutable connections report nothing meaningful for it).
    pub async fn check_file(path: &Path) -> Result<IntegrityChecks, DatabaseError> {
        let connect_options = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .immutable(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with(connect_options)
            .await
            .map_err(DatabaseError::Connection)?;

        let stats = Self::page_stats(&pool).await?;
        let quick_check = Self::check_lines(&pool, true).await?;
        let foreign_key_check = Self::foreign_key_violations(&pool).await?;
        Ok(IntegrityChecks {
            database_size_bytes: stats.size_bytes(),
            page_count: stats.page_count,
            page_size: stats.page_size,
            freelist_count: stats.freelist_count,
            journal_mode: String::new(),
            integrity: IntegrityLines::default(),
            quick_check,
            foreign_key_check,
        })
    }

    /// `PRAGMA page_count` / `page_size` / `freelist_count` in one read.
    pub async fn page_stats(pool: &SqlitePool) -> Result<PageStats, DatabaseError> {
        let (page_count, page_size, freelist_count): (i64, i64, i64) = sqlx::query_as(
            "SELECT pc.page_count, ps.page_size, fl.freelist_count
               FROM pragma_page_count() pc, pragma_page_size() ps,
                    pragma_freelist_count() fl",
        )
        .fetch_one(pool)
        .await?;
        Ok(PageStats {
            page_count: page_count.max(0),
            page_size: page_size.max(0),
            freelist_count: freelist_count.max(0),
        })
    }

    /// The live journal mode (should always be WAL).
    async fn journal_mode(pool: &SqlitePool) -> Result<String, DatabaseError> {
        let (mode,): (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(pool)
            .await?;
        Ok(mode)
    }

    /// `PRAGMA integrity_check` (or `quick_check`): one line per scanned
    /// page plus the trailing "ok" verdict.
    async fn check_lines(pool: &SqlitePool, quick: bool) -> Result<IntegrityLines, DatabaseError> {
        let pragma = if quick {
            "quick_check"
        } else {
            "integrity_check"
        };
        let lines: Vec<String> = sqlx::query_scalar(&format!("PRAGMA {pragma}"))
            .fetch_all(pool)
            .await?;
        let ok = lines.last().map(String::as_str) == Some("ok");
        Ok(IntegrityLines { ok, lines })
    }

    /// `PRAGMA foreign_key_check`, one human-readable line per violation.
    async fn foreign_key_violations(pool: &SqlitePool) -> Result<Vec<String>, DatabaseError> {
        let rows: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
            .fetch_all(pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(table, rowid, parent, fkid)| {
                format!("{table}: row {rowid} references {parent} (fk {fkid})")
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn live_battery_reports_clean_healthy_database() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let checker = IntegrityChecker::new(repository, "/tmp/chronodesk.db");

        let checks = checker.check_live().await.expect("live battery");

        assert_eq!(checks.journal_mode.to_lowercase(), "wal");
        assert!(checks.database_size_bytes > 0);
        assert!(checks.page_count > 0);
        assert!(checks.page_size > 0);
        assert!(
            checks.integrity.ok,
            "full integrity check passes: {:?}",
            checks.integrity.lines
        );
        assert!(checks.quick_check.ok, "quick check passes");
        assert!(
            checks.foreign_key_check.is_empty(),
            "{:?}",
            checks.foreign_key_check
        );
        assert_eq!(checker.db_path(), "/tmp/chronodesk.db");
    }

    #[tokio::test]
    async fn file_battery_validates_a_vacuum_snapshot() {
        let (database, _temp) = test_database().await;
        let repository = MaintenanceRepository::new(database.pool().clone());
        let dest = _temp.path().join("snapshot.db");

        repository.vacuum_into(&dest).await.expect("vacuum into");
        let checks = IntegrityChecker::check_file(&dest)
            .await
            .expect("file battery");

        assert!(
            checks.quick_check.ok,
            "backup passes quick check: {:?}",
            checks.quick_check.lines
        );
        assert!(checks.foreign_key_check.is_empty());
        assert!(checks.database_size_bytes > 0);
        assert_eq!(
            checks.integrity.lines,
            Vec::<String>::new(),
            "file checks skip full scan"
        );
    }

    #[tokio::test]
    async fn file_battery_rejects_non_sqlite_content() {
        let (_database, _temp) = test_database().await;
        let junk = _temp.path().join("junk.db");
        std::fs::write(&junk, b"this is not a sqlite file at all").expect("write junk");

        let result = IntegrityChecker::check_file(&junk).await;
        assert!(result.is_err(), "non-sqlite files must fail validation");
    }

    #[test]
    fn page_stats_math_is_sane() {
        let stats = PageStats {
            page_count: 100,
            page_size: 4096,
            freelist_count: 25,
        };
        assert_eq!(stats.size_bytes(), 409_600);
        assert!((stats.freelist_ratio() - 0.25).abs() < f64::EPSILON);

        let zero = PageStats {
            page_count: 0,
            page_size: 0,
            freelist_count: 0,
        };
        assert_eq!(zero.size_bytes(), 0);
        assert_eq!(zero.freelist_ratio(), 0.0);
    }
}
