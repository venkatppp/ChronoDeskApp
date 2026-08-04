//! Security Hardening models (RC-10 M4).
//!
//! DTOs for the security surfaces: the append-only audit ledger, the
//! key/value policy config, per-run check findings, check results, the
//! 0..100 security score, secret/config validation, file/database/backup
//! permission verification, and the persisted recommendations produced by
//! the pure rule engine. Everything here is a plain serializable DTO —
//! the SQL lives in [`crate::repositories::SecurityRepository`] and the
//! policy logic in [`crate::security`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ----------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------

/// Severity of a security finding, audit entry or recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecuritySeverity {
    /// Informational — no action required.
    Info,
    /// Worth attention — could become a problem.
    Warning,
    /// Must be addressed — an active weakness.
    Critical,
}

impl SecuritySeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            SecuritySeverity::Info => "info",
            SecuritySeverity::Warning => "warning",
            SecuritySeverity::Critical => "critical",
        }
    }

    /// Ordered weight used by the score and recommendation engine.
    pub fn weight(self) -> u32 {
        match self {
            SecuritySeverity::Info => 0,
            SecuritySeverity::Warning => 1,
            SecuritySeverity::Critical => 2,
        }
    }
}

impl From<&str> for SecuritySeverity {
    fn from(value: &str) -> Self {
        match value {
            "warning" => SecuritySeverity::Warning,
            "critical" => SecuritySeverity::Critical,
            _ => SecuritySeverity::Info,
        }
    }
}

impl From<SecuritySeverity> for String {
    fn from(value: SecuritySeverity) -> Self {
        value.as_str().to_string()
    }
}

/// Which surface a security check inspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityCategory {
    /// SQLite database configuration (journal mode, FK, trusted schema).
    Database,
    /// File system permissions (db file, backups, directories).
    Files,
    /// Secret & config handling (key storage, keyring availability).
    Secrets,
    /// Backup integrity (file presence, checksum vs the M3 ledger).
    Backup,
    /// Path / input validation rules.
    Input,
    /// Security policy configuration values.
    Config,
}

impl SecurityCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityCategory::Database => "database",
            SecurityCategory::Files => "files",
            SecurityCategory::Secrets => "secrets",
            SecurityCategory::Backup => "backup",
            SecurityCategory::Input => "input",
            SecurityCategory::Config => "config",
        }
    }
}

impl From<&str> for SecurityCategory {
    fn from(value: &str) -> Self {
        match value {
            "files" => SecurityCategory::Files,
            "secrets" => SecurityCategory::Secrets,
            "backup" => SecurityCategory::Backup,
            "input" => SecurityCategory::Input,
            "config" => SecurityCategory::Config,
            _ => SecurityCategory::Database,
        }
    }
}

/// Lifecycle state of a persisted recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityRecommendationStatus {
    /// Freshly produced; awaiting user attention.
    Open,
    /// Acknowledged (either applied or accepted as-is).
    Applied,
    /// Explicitly rejected.
    Dismissed,
}

impl SecurityRecommendationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityRecommendationStatus::Open => "open",
            SecurityRecommendationStatus::Applied => "applied",
            SecurityRecommendationStatus::Dismissed => "dismissed",
        }
    }
}

impl From<&str> for SecurityRecommendationStatus {
    fn from(value: &str) -> Self {
        match value {
            "applied" => SecurityRecommendationStatus::Applied,
            "dismissed" => SecurityRecommendationStatus::Dismissed,
            _ => SecurityRecommendationStatus::Open,
        }
    }
}

// ----------------------------------------------------------------------
// Checks & findings
// ----------------------------------------------------------------------

/// The pure output of one security check. Produced by the check functions
/// in [`crate::security::checks`], collected by the validator into a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityCheckResult {
    pub check_name: String,
    pub category: SecurityCategory,
    pub severity: SecuritySeverity,
    pub passed: bool,
    pub detail: String,
}

impl SecurityCheckResult {
    /// Convenience over the DTO; matches name + category.
    pub fn eq_name_and_category(&self, name: &str, category: SecurityCategory) -> bool {
        self.check_name == name && self.category == category
    }

    /// Convenience: the verdict boolean.
    pub fn is_passed(&self) -> bool {
        self.passed
    }
}

/// One persisted `security_findings` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFinding {
    pub id: i64,
    /// UUID grouping the battery that produced this row.
    pub run_id: String,
    pub category: SecurityCategory,
    pub severity: SecuritySeverity,
    pub check_name: String,
    pub passed: bool,
    pub detail: String,
    pub checked_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Audit ledger
// ----------------------------------------------------------------------

/// One `security_audit_log` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAuditEntry {
    pub id: i64,
    pub action: String,
    pub severity: SecuritySeverity,
    /// `system` | `monitor` | `user`.
    pub actor: String,
    pub target: String,
    pub detail: String,
    pub created_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Config
// ----------------------------------------------------------------------

/// One `security_config` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfigEntry {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Reports
// ----------------------------------------------------------------------

/// The current 0..100 security posture, recomputed over a battery's
/// findings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScoreReport {
    pub scored_at: DateTime<Utc>,
    /// `0..=100`; higher is safer.
    pub score: f64,
    /// `excellent` | `good` | `fair` | `weak`.
    pub status: String,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    /// The findings this run produced, newest first.
    pub findings: Vec<SecurityFinding>,
}

/// A full battery of checks run against the live environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityDiagnosticsReport {
    pub run_id: String,
    pub ran_at: DateTime<Utc>,
    pub db_path: String,
    pub checks: Vec<SecurityCheckResult>,
    pub score: f64,
    pub total_checks: usize,
    pub passed_checks: usize,
}

/// Secret & config handling verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretValidationReport {
    pub checked_at: DateTime<Utc>,
    pub ok: bool,
    pub checks: Vec<SecurityCheckResult>,
}

/// File / database / backup permission verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsReport {
    pub checked_at: DateTime<Utc>,
    pub ok: bool,
    pub checks: Vec<SecurityCheckResult>,
}

/// One `security_recommendations` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityRecommendation {
    pub id: i64,
    /// Identifies the producing rule; stable across runs.
    pub rule: String,
    pub severity: SecuritySeverity,
    pub title: String,
    pub detail: String,
    pub status: SecurityRecommendationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The non-fatal startup validation pass result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupValidationReport {
    pub validated_at: DateTime<Utc>,
    pub run_id: String,
    pub ok: bool,
    pub score: f64,
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
}
