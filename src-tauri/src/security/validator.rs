//! Security validator (RC-10 M4).
//!
//! The stateful half of the check battery: gathers the environment facts
//! (SQLite `PRAGMA` values, file metadata, secret-store state, the backup
//! ledger + rehashed snapshot) and folds them through the pure functions
//! in [`crate::security::checks`]. Persisting the resulting findings for
//! a run is this service's job; scoring, auditing, and recommendations
//! belong to the engine.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;

use crate::errors::DatabaseError;
use crate::hashing::HashingService;
use crate::llm::SecretStore;
use crate::models::backup::{BackupRunKind, BackupRunStatus};
use crate::models::security::{
    PermissionsReport, SecretValidationReport, SecurityCheckResult, SecurityDiagnosticsReport,
};
use crate::repositories::{LLMRepository, MaintenanceRepository, SecurityRepository};
use crate::security::{checks, policy};

/// Secret-store service name for the security probe. Never collides with
/// the LLM credential service (`ChronoDesk LLM API Key`), and each probe
/// uses a fresh random account so it can never touch a real credential.
const PROBE_SERVICE: &str = "ChronoDesk Security Probe";

/// Gathers check inputs and produces batteries.
#[derive(Clone)]
pub struct SecurityValidator {
    repository: SecurityRepository,
    maintenance_repository: Arc<MaintenanceRepository>,
    llm_repository: Arc<LLMRepository>,
    secret_store: Arc<dyn SecretStore>,
    hashing: HashingService,
    db_path: PathBuf,
    backup_dir: PathBuf,
}

impl SecurityValidator {
    pub fn new(
        repository: SecurityRepository,
        maintenance_repository: Arc<MaintenanceRepository>,
        llm_repository: Arc<LLMRepository>,
        secret_store: Arc<dyn SecretStore>,
        db_path: PathBuf,
        backup_dir: PathBuf,
    ) -> Self {
        Self {
            repository,
            maintenance_repository,
            llm_repository,
            secret_store,
            hashing: HashingService::new(),
            db_path,
            backup_dir,
        }
    }

    /// The configured live database path.
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// The full battery: database configuration, file permissions,
    /// secret handling, backup verification, path rules, and policy
    /// config. Persists one `security_findings` row per check under
    /// `run_id` and returns the report (with the 0..100 score).
    pub async fn run_full(&self, run_id: &str) -> Result<SecurityDiagnosticsReport, DatabaseError> {
        let checks = self.collect_checks(false, false).await?;

        let (score, total, passed, _) = crate::security::scoring::score(&checks);
        for check in &checks {
            self.repository
                .insert_finding(
                    run_id,
                    check.category,
                    check.severity,
                    &check.check_name,
                    check.passed,
                    &check.detail,
                )
                .await?;
        }

        Ok(SecurityDiagnosticsReport {
            run_id: run_id.to_string(),
            ran_at: Utc::now(),
            db_path: self.db_path.display().to_string(),
            checks,
            score,
            total_checks: total,
            passed_checks: passed,
        })
    }

    /// The secret-handling sub-battery (API-key storage + OS secret-store
    /// round-trip). Focused live probe — never persisted, so the
    /// dashboard score stays coherent with full batteries.
    pub async fn run_secrets(&self) -> Result<SecretValidationReport, DatabaseError> {
        let mut checks = Vec::new();
        checks.push(self.check_api_key_storage().await?);
        checks.push(self.check_secret_store_probe().await?);
        let ok = checks.iter().all(|c| c.passed);
        Ok(SecretValidationReport {
            checked_at: Utc::now(),
            ok,
            checks,
        })
    }

    /// The file/database permission sub-battery. Focused live probe —
    /// never persisted.
    pub async fn run_permissions(&self) -> Result<PermissionsReport, DatabaseError> {
        let checks = self.collect_files_checks().await?;
        let ok = checks.iter().all(|c| c.passed);
        Ok(PermissionsReport {
            checked_at: Utc::now(),
            ok,
            checks,
        })
    }

    /// Collects every check for the full battery. `secondary` toggles
    /// nothing today but keeps a seam for lighter monitor passes.
    async fn collect_checks(
        &self,
        _include_secrets: bool,
        _include_backup: bool,
    ) -> Result<Vec<SecurityCheckResult>, DatabaseError> {
        let mut out = Vec::new();

        // -- Database configuration --
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(self.repository.pool())
            .await
            .map_err(DatabaseError::from)?;
        out.push(checks::check_journal_mode(&journal_mode));

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(self.repository.pool())
            .await
            .map_err(DatabaseError::from)?;
        out.push(checks::check_foreign_keys(foreign_keys != 0));

        let trusted_schema: i64 = sqlx::query_scalar("PRAGMA trusted_schema")
            .fetch_one(self.repository.pool())
            .await
            .map_err(DatabaseError::from)?;
        out.push(checks::check_trusted_schema(trusted_schema));

        let secure_delete: i64 = sqlx::query_scalar("PRAGMA secure_delete")
            .fetch_one(self.repository.pool())
            .await
            .map_err(DatabaseError::from)?;
        out.push(checks::check_secure_delete(secure_delete));

        // -- File permissions --
        out.extend(self.collect_files_checks().await?);

        // -- Secret handling --
        out.push(self.check_api_key_storage().await?);
        out.push(self.check_secret_store_probe().await?);

        // -- Backup verification --
        out.extend(self.collect_backup_checks().await?);

        // -- Path / input rules --
        let db_path = self.db_path.display().to_string();
        out.push(checks::check_path_absolute(&db_path));
        out.push(checks::check_path_has_nul(&db_path));

        // -- Policy config --
        let monitor = self
            .repository
            .config_get(policy::KEY_MONITOR_INTERVAL_SECONDS)
            .await?;
        let monitor_valid = monitor
            .as_deref()
            .map(|v| policy::validate_monitor_interval(v).is_ok())
            .unwrap_or(true);
        out.push(checks::check_monitor_interval_config(
            monitor.as_deref(),
            monitor_valid,
        ));

        let retention = self
            .repository
            .config_get(policy::KEY_AUDIT_RETENTION_DAYS)
            .await?;
        let retention_valid = retention
            .as_deref()
            .map(|v| policy::validate_retention_days(v).is_ok())
            .unwrap_or(true);
        out.push(checks::check_audit_retention_config(
            retention.as_deref(),
            retention_valid,
        ));

        Ok(out)
    }

    /// The file-permission checks (db file + backups dir + latest backup).
    async fn collect_files_checks(&self) -> Result<Vec<SecurityCheckResult>, DatabaseError> {
        let mut out = Vec::new();

        let db_mode = file_mode(&self.db_path).await;
        out.push(checks::check_db_file_permissions(db_mode));

        let backup_dir_exists = self.backup_dir.is_dir();
        let backup_dir_mode = if backup_dir_exists {
            file_mode(&self.backup_dir).await
        } else {
            None
        };
        out.push(checks::check_backup_dir_permissions(
            backup_dir_exists,
            backup_dir_mode,
        ));

        let latest = self.latest_backup_path().await?;
        let (exists, m) = match &latest {
            Some(path) => (true, file_mode(path).await),
            None => (false, None),
        };
        out.push(checks::check_latest_backup_permissions(exists, m));

        Ok(out)
    }

    /// Backup-presence and checksum checks against the M3 audit ledger.
    async fn collect_backup_checks(&self) -> Result<Vec<SecurityCheckResult>, DatabaseError> {
        let mut out = Vec::new();

        let run = self
            .maintenance_repository
            .latest_run_of_kind(BackupRunKind::Backup)
            .await?;
        let run = run.filter(|r| r.status == BackupRunStatus::Success);
        let Some(run) = run else {
            out.push(checks::check_backup_presence(false, false));
            out.push(checks::check_backup_checksum(false, None, None));
            return Ok(out);
        };

        let backup_path = self.backup_dir.join(&run.path);
        let exists = backup_path.is_file();
        let computed = if exists {
            self.hashing.hash_file(&backup_path).ok()
        } else {
            None
        };

        out.push(checks::check_backup_presence(true, exists));
        out.push(checks::check_backup_checksum(
            true,
            Some(&run.checksum),
            computed.as_deref(),
        ));

        Ok(out)
    }

    /// The latest successful backup's resolved path, if the ledger has one.
    async fn latest_backup_path(&self) -> Result<Option<PathBuf>, DatabaseError> {
        let run = self
            .maintenance_repository
            .latest_run_of_kind(BackupRunKind::Backup)
            .await?;
        let run = run.filter(|r| r.status == BackupRunStatus::Success);
        Ok(run.map(|r| self.backup_dir.join(r.path)))
    }

    async fn check_api_key_storage(&self) -> Result<SecurityCheckResult, DatabaseError> {
        let state = self.llm_repository.api_key_storage_state().await?;
        Ok(checks::check_api_key_storage(state))
    }

    async fn check_secret_store_probe(&self) -> Result<SecurityCheckResult, DatabaseError> {
        let account = uuid::Uuid::new_v4().to_string();
        let value = uuid::Uuid::new_v4().to_string();
        let probe = (|| {
            self.secret_store.store(PROBE_SERVICE, &account, &value)?;
            let read_back = self.secret_store.get(PROBE_SERVICE, &account)?;
            if read_back != value {
                return Err("secret store returned a different value".to_string());
            }
            self.secret_store.delete(PROBE_SERVICE, &account)?;
            Ok(())
        })();
        match probe {
            Ok(()) => Ok(checks::check_secret_store_probe(true, None)),
            Err(error) => Ok(checks::check_secret_store_probe(false, Some(error))),
        }
    }
}

/// Unix permission mode of `path`, or `None` on non-unix or when the file
/// cannot be stat-ed (a disappearing file is treated as nothing to
/// enforce rather than an error).
async fn file_mode(path: &std::path::Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        tokio::fs::metadata(path).await.ok().map(|meta| meta.mode())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

#[cfg(test)]
use crate::database::test_database;
#[cfg(test)]
use crate::llm::InMemorySecretStore;

/// Builds a validator wired to a disposable temp database with an
/// in-memory secret store (test-only). Returns the temp guard, the
/// validator, its security repository, and its maintenance repository
/// (so tests can seed the backup ledger the validator reads).
#[cfg(test)]
pub(crate) async fn build_validator() -> (
    tempfile::TempDir,
    SecurityValidator,
    SecurityRepository,
    Arc<MaintenanceRepository>,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();

    let security_repository = SecurityRepository::new(pool.clone());
    let maintenance_repository = Arc::new(MaintenanceRepository::new(pool.clone()));
    let store = Arc::new(InMemorySecretStore::new());
    let llm_repository = Arc::new(LLMRepository::new(pool, store.clone()));

    let db_path = temp_dir.path().join("chronodesk.db");
    let backup_dir = temp_dir.path().join("backups");

    let validator = SecurityValidator::new(
        security_repository.clone(),
        maintenance_repository.clone(),
        llm_repository,
        store,
        db_path,
        backup_dir,
    );
    (
        temp_dir,
        validator,
        security_repository,
        maintenance_repository,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::backup::BackupRunStatus;
    use crate::models::security::{SecurityCategory, SecuritySeverity};

    #[tokio::test]
    async fn fresh_full_battery_passes_with_all_defaults() {
        let (_temp, validator, _repo, _maintenance) = build_validator().await;
        let run_id = uuid::Uuid::new_v4().to_string();

        let report = validator.run_full(&run_id).await.expect("run full");

        assert!(report.db_path.contains("chronodesk.db"));
        // WAL + FK + trusted_schema + secure_delete(off→info) + 2 file
        // checks + 2 secret checks + 2 backup checks + 2 path + 2 config.
        assert!(report.total_checks >= 14, "expected a broad battery");
        assert_eq!(
            report.passed_checks,
            report.total_checks - 1,
            "the only soft finding on a fresh DB is the secure_delete advisory"
        );
        let soft = report
            .checks
            .iter()
            .find(|c| !c.passed)
            .expect("the one advisory");
        assert_eq!(soft.check_name, "secure_delete");
        assert_eq!(soft.severity, SecuritySeverity::Info, "no score impact");
        assert_eq!(report.score, 100.0, "info advisory is score-neutral");
    }

    #[tokio::test]
    async fn run_full_persists_findings_under_the_run_id() {
        let (_temp, validator, repository, _maintenance) = build_validator().await;
        let run_id = uuid::Uuid::new_v4().to_string();

        validator.run_full(&run_id).await.expect("run full");

        let findings = repository
            .recent_findings(100)
            .await
            .expect("findings")
            .into_iter()
            .filter(|f| f.run_id == run_id)
            .collect::<Vec<_>>();
        assert!(!findings.is_empty(), "findings must be persisted");
        assert!(
            findings.iter().any(|f| f.check_name == "journal_mode"),
            "battery includes the journal check"
        );
    }

    #[tokio::test]
    async fn secrets_and_permissions_sub_batteries_are_coherent() {
        let (_temp, validator, _repo, _maintenance) = build_validator().await;

        let secrets = validator.run_secrets().await.expect("secrets");
        assert_eq!(secrets.checks.len(), 2);
        assert!(secrets.ok, "in-memory keyring + no key must pass");

        let permissions = validator.run_permissions().await.expect("permissions");
        assert!(
            permissions
                .checks
                .iter()
                .any(|c| c.check_name == "db_file_permissions"),
            "permissions battery includes the db file check"
        );
    }

    #[tokio::test]
    async fn backup_checksum_detects_tampering() {
        // Seed the validator's own maintenance ledger with a success
        // backup run, then write a file whose content does not match the
        // recorded checksum.
        let (temp_dir, validator, _repo, maintenance_repository) = build_validator().await;
        let run_id = uuid::Uuid::new_v4().to_string();

        let backups_dir = temp_dir.path().join("backups");
        std::fs::create_dir_all(&backups_dir).expect("backups dir");
        let backup_file = backups_dir.join("chronodesk-0001.db");
        std::fs::write(&backup_file, b"tampered-content").expect("write backup");

        maintenance_repository
            .record_run(
                BackupRunKind::Backup,
                BackupRunStatus::Success,
                "chronodesk-0001.db",
                backup_file.metadata().expect("meta").len() as i64,
                "deadbeef",
                "created",
                1,
            )
            .await
            .expect("record run");

        // Run the full battery: the validator rehashes the recorded
        // backup via its own maintenance ledger.
        let report = validator.run_full(&run_id).await.expect("run full");
        let checksum = report
            .checks
            .iter()
            .find(|c| c.check_name == "backup_checksum")
            .expect("checksum check");
        assert!(!checksum.passed, "tampered backup must fail checksum");
        assert_eq!(checksum.category, SecurityCategory::Backup);
        assert_eq!(checksum.severity, SecuritySeverity::Critical);
    }
}
