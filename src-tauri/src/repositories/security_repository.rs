//! Security Hardening repository (RC-10 M4).
//!
//! Owns every SQL statement behind the security surfaces: the append-only
//! `security_audit_log`, the `security_config` key/value policy table, the
//! per-run `security_findings` ledger, and the `security_recommendations`
//! table. All SQL stays here; check *policy* (which verdicts are
//! tolerable, how to score) lives in [`crate::security`].

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::errors::DatabaseError;
use crate::models::security::{
    SecurityAuditEntry, SecurityCategory, SecurityConfigEntry, SecurityFinding,
    SecurityRecommendation, SecurityRecommendationStatus, SecuritySeverity,
};

/// Raw `security_audit_log` row.
type AuditRow = (i64, String, String, String, String, String, DateTime<Utc>);
/// Raw `security_config` row.
type ConfigRow = (String, String, DateTime<Utc>);
/// Raw `security_findings` row.
type FindingRow = (
    i64,
    String,
    String,
    String,
    String,
    bool,
    String,
    DateTime<Utc>,
);
/// Raw `security_recommendations` row.
type RecommendationRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// Repository for the RC-10 M4 security ledgers.
#[derive(Debug, Clone)]
pub struct SecurityRepository {
    pool: SqlitePool,
}

impl SecurityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// The pool, for the validator's read-only `PRAGMA` probes.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ------------------------------------------------------------------
    // Audit log
    // ------------------------------------------------------------------

    /// Appends one audit entry, returning its id.
    pub async fn audit(
        &self,
        action: &str,
        severity: SecuritySeverity,
        actor: &str,
        target: &str,
        detail: &str,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO security_audit_log (action, severity, actor, target, detail)
             VALUES (?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(action)
        .bind(severity.as_str())
        .bind(actor)
        .bind(target)
        .bind(detail)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// The most recent audit entries, newest-first.
    pub async fn recent_audit(&self, limit: u32) -> Result<Vec<SecurityAuditEntry>, DatabaseError> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT id, action, severity, actor, target, detail, created_at
             FROM security_audit_log ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::audit_from_row).collect())
    }

    /// Removes audit entries older than `cutoff`; returns rows removed.
    pub async fn prune_audit_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query("DELETE FROM security_audit_log WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    fn audit_from_row(row: AuditRow) -> SecurityAuditEntry {
        SecurityAuditEntry {
            id: row.0,
            action: row.1,
            severity: SecuritySeverity::from(row.2.as_str()),
            actor: row.3,
            target: row.4,
            detail: row.5,
            created_at: row.6,
        }
    }

    /// Total audit entries — feeds the prune recommendation rule.
    pub async fn count_audit(&self) -> Result<i64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM security_audit_log")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    /// Total findings rows — feeds the prune recommendation rule.
    pub async fn count_findings(&self) -> Result<i64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM security_findings")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    // ------------------------------------------------------------------
    // Policy config
    // ------------------------------------------------------------------

    /// Every policy entry.
    pub async fn config_all(&self) -> Result<Vec<SecurityConfigEntry>, DatabaseError> {
        let rows: Vec<ConfigRow> =
            sqlx::query_as("SELECT key, value, updated_at FROM security_config ORDER BY key")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows
            .into_iter()
            .map(|r| SecurityConfigEntry {
                key: r.0,
                value: r.1,
                updated_at: r.2,
            })
            .collect())
    }

    /// One policy value, if set.
    pub async fn config_get(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let value: Option<(String,)> =
            sqlx::query_as("SELECT value FROM security_config WHERE key = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(value.map(|r| r.0))
    }

    /// Inserts or updates a policy value.
    pub async fn config_set(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO security_config (key, value, updated_at)
             VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                                            updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Findings
    // ------------------------------------------------------------------

    /// Inserts one finding row, returning its id.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_finding(
        &self,
        run_id: &str,
        category: SecurityCategory,
        severity: SecuritySeverity,
        check_name: &str,
        passed: bool,
        detail: &str,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO security_findings
               (run_id, category, severity, check_name, passed, detail)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(run_id)
        .bind(category.as_str())
        .bind(severity.as_str())
        .bind(check_name)
        .bind(passed)
        .bind(detail)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// The most recent findings, newest-first.
    pub async fn recent_findings(&self, limit: u32) -> Result<Vec<SecurityFinding>, DatabaseError> {
        let rows: Vec<FindingRow> = sqlx::query_as(
            "SELECT id, run_id, category, severity, check_name, passed, detail, checked_at
             FROM security_findings ORDER BY checked_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::finding_from_row).collect())
    }

    /// All findings from one battery run, newest-first.
    pub async fn findings_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<SecurityFinding>, DatabaseError> {
        let rows: Vec<FindingRow> = sqlx::query_as(
            "SELECT id, run_id, category, severity, check_name, passed, detail, checked_at
             FROM security_findings WHERE run_id = ?
             ORDER BY checked_at DESC, id DESC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::finding_from_row).collect())
    }

    /// Removes findings older than `cutoff`; returns rows removed.
    pub async fn prune_findings_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query("DELETE FROM security_findings WHERE checked_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    fn finding_from_row(row: FindingRow) -> SecurityFinding {
        SecurityFinding {
            id: row.0,
            run_id: row.1,
            category: SecurityCategory::from(row.2.as_str()),
            severity: SecuritySeverity::from(row.3.as_str()),
            check_name: row.4,
            passed: row.5,
            detail: row.6,
            checked_at: row.7,
        }
    }

    // ------------------------------------------------------------------
    // Recommendations
    // ------------------------------------------------------------------

    /// Upserts a recommendation by rule: existing rows keep their status
    /// (an applied or dismissed decision is never reopened) while the
    /// produced content is refreshed.
    pub async fn upsert_recommendation(
        &self,
        rule: &str,
        severity: SecuritySeverity,
        title: &str,
        detail: &str,
    ) -> Result<i64, DatabaseError> {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO security_recommendations (rule, severity, title, detail)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(rule) DO UPDATE SET severity = excluded.severity,
                                             title = excluded.title,
                                             detail = excluded.detail,
                                             status = security_recommendations.status,
                                             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             RETURNING id",
        )
        .bind(rule)
        .bind(severity.as_str())
        .bind(title)
        .bind(detail)
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    /// All recommendations, newest-first.
    pub async fn recommendations(&self) -> Result<Vec<SecurityRecommendation>, DatabaseError> {
        let rows: Vec<RecommendationRow> = sqlx::query_as(
            "SELECT id, rule, severity, title, detail, status, created_at, updated_at
             FROM security_recommendations ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(Self::recommendation_from_row)
            .collect())
    }

    /// Updates a recommendation's status; returns whether a row changed.
    pub async fn update_recommendation_status(
        &self,
        id: i64,
        status: SecurityRecommendationStatus,
    ) -> Result<bool, DatabaseError> {
        let result = sqlx::query(
            "UPDATE security_recommendations
             SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// One recommendation by id, if it exists.
    pub async fn recommendation_by_id(
        &self,
        id: i64,
    ) -> Result<Option<SecurityRecommendation>, DatabaseError> {
        let row: Option<RecommendationRow> = sqlx::query_as(
            "SELECT id, rule, severity, title, detail, status, created_at, updated_at
             FROM security_recommendations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Self::recommendation_from_row))
    }

    fn recommendation_from_row(row: RecommendationRow) -> SecurityRecommendation {
        SecurityRecommendation {
            id: row.0,
            rule: row.1,
            severity: SecuritySeverity::from(row.2.as_str()),
            title: row.3,
            detail: row.4,
            status: SecurityRecommendationStatus::from(row.5.as_str()),
            created_at: row.6,
            updated_at: row.7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn audit_round_trip_orders_newest_first() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        let first = repository
            .audit(
                "startup_validation",
                SecuritySeverity::Info,
                "system",
                "db",
                "first",
            )
            .await
            .expect("audit first");
        let second = repository
            .audit(
                "config_set",
                SecuritySeverity::Warning,
                "user",
                "config",
                "second",
            )
            .await
            .expect("audit second");

        let entries = repository.recent_audit(10).await.expect("recent audit");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, second, "newest first");
        assert_eq!(entries[0].action, "config_set");
        assert_eq!(entries[0].severity, SecuritySeverity::Warning);
        assert_eq!(entries[0].actor, "user");
        assert_eq!(entries[1].id, first);

        let limited = repository.recent_audit(1).await.expect("limited");
        assert_eq!(limited.len(), 1);
    }

    #[tokio::test]
    async fn audit_prune_removes_only_old_rows() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        repository
            .audit("monitor_tick", SecuritySeverity::Info, "monitor", "", "old")
            .await
            .expect("old");
        let cutoff = Utc::now() - chrono::Duration::days(1);
        // Backdate the single row so a normal prune targets it.
        sqlx::query(
            "UPDATE security_audit_log SET created_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-10 days') WHERE action = 'monitor_tick'",
        )
        .execute(database.pool())
        .await
        .expect("backdate");
        repository
            .audit(
                "config_set",
                SecuritySeverity::Info,
                "user",
                "config",
                "fresh",
            )
            .await
            .expect("fresh");

        let removed = repository
            .prune_audit_older_than(cutoff)
            .await
            .expect("prune");
        assert_eq!(removed, 1, "only the backdated row is pruned");

        let entries = repository.recent_audit(10).await.expect("recent");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, "config_set");
    }

    #[tokio::test]
    async fn config_upsert_lists_and_round_trips() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        repository
            .config_set("security.monitor_interval_seconds", "300")
            .await
            .expect("set interval");
        repository
            .config_set("security.audit_retention_days", "90")
            .await
            .expect("set retention");

        let all = repository.config_all().await.expect("all config");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].key, "security.audit_retention_days");

        repository
            .config_set("security.monitor_interval_seconds", "600")
            .await
            .expect("update interval");
        let value = repository
            .config_get("security.monitor_interval_seconds")
            .await
            .expect("get interval")
            .expect("some value");
        assert_eq!(value, "600");

        assert!(
            repository
                .config_get("security.missing")
                .await
                .expect("missing")
                .is_none(),
            "unset keys return None"
        );
    }

    #[tokio::test]
    async fn findings_round_trip_and_group_by_run() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        repository
            .insert_finding(
                "run-a",
                SecurityCategory::Database,
                SecuritySeverity::Info,
                "journal_mode",
                true,
                "wal",
            )
            .await
            .expect("finding a1");
        repository
            .insert_finding(
                "run-a",
                SecurityCategory::Secrets,
                SecuritySeverity::Warning,
                "api_key_storage",
                false,
                "plaintext key detected",
            )
            .await
            .expect("finding a2");
        repository
            .insert_finding(
                "run-b",
                SecurityCategory::Files,
                SecuritySeverity::Critical,
                "db_file_permissions",
                false,
                "world writable",
            )
            .await
            .expect("finding b1");

        let run_a = repository
            .findings_by_run("run-a")
            .await
            .expect("findings a");
        assert_eq!(run_a.len(), 2);
        assert!(
            run_a
                .iter()
                .any(|f| f.check_name == "api_key_storage" && !f.passed),
            "failed finding present in group"
        );

        let recent = repository.recent_findings(10).await.expect("recent");
        assert_eq!(recent.len(), 3, "recent spans all runs");
        assert!(recent.iter().any(|f| f.run_id == "run-b"));
    }

    #[tokio::test]
    async fn findings_prune_respects_cutoff() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        repository
            .insert_finding(
                "old-run",
                SecurityCategory::Input,
                SecuritySeverity::Info,
                "path_absolute",
                true,
                "",
            )
            .await
            .expect("old");
        sqlx::query(
            "UPDATE security_findings SET checked_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-30 days') WHERE run_id = 'old-run'",
        )
        .execute(database.pool())
        .await
        .expect("backdate");
        repository
            .insert_finding(
                "new-run",
                SecurityCategory::Config,
                SecuritySeverity::Info,
                "monitor_interval",
                true,
                "",
            )
            .await
            .expect("new");

        let removed = repository
            .prune_findings_older_than(Utc::now() - chrono::Duration::days(7))
            .await
            .expect("prune");
        assert_eq!(removed, 1);

        let recent = repository.recent_findings(10).await.expect("recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].run_id, "new-run");
    }

    #[tokio::test]
    async fn recommendation_upsert_keeps_applied_status() {
        let (database, _temp) = test_database().await;
        let repository = SecurityRepository::new(database.pool().clone());

        repository
            .upsert_recommendation(
                "enable_secure_delete",
                SecuritySeverity::Info,
                "Enable secure delete",
                "Erasure of deleted data is not guaranteed.",
            )
            .await
            .expect("first upsert");

        let recommendations = repository.recommendations().await.expect("list");
        assert_eq!(recommendations.len(), 1);
        assert_eq!(
            recommendations[0].status,
            SecurityRecommendationStatus::Open
        );

        repository
            .update_recommendation_status(
                recommendations[0].id,
                SecurityRecommendationStatus::Applied,
            )
            .await
            .expect("apply");

        // Re-running the rule upserts details/severity but must NOT reopen
        // an already-applied recommendation.
        repository
            .upsert_recommendation(
                "enable_secure_delete",
                SecuritySeverity::Warning,
                "Enable secure delete",
                "Updated detail.",
            )
            .await
            .expect("second upsert");

        let recommendations = repository.recommendations().await.expect("list");
        assert_eq!(recommendations.len(), 1);
        assert_eq!(
            recommendations[0].status,
            SecurityRecommendationStatus::Applied,
            "applied recs keep their status"
        );
        assert_eq!(recommendations[0].detail, "Updated detail.");

        assert!(
            !repository
                .update_recommendation_status(999_999, SecurityRecommendationStatus::Dismissed)
                .await
                .expect("missing update"),
            "unknown ids report no change"
        );
    }
}
