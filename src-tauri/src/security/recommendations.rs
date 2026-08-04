//! Security recommendation rules (RC-10 M4).
//!
//! Pure: given a battery of [`SecurityCheckResult`]s plus a couple of
//! ledger-size facts, produces recommendation *candidates*. The engine
//! persists them (upsert by rule, preserving applied/dismissed status)
//! and executes the tiny, safe apply actions (`PruneAudit`,
//! `PruneFindings`) that a rule maps to.

use crate::models::security::{SecurityCategory, SecurityCheckResult, SecuritySeverity};

/// A safe, engine-executable action a recommendation can carry. Rules
/// without an action are acknowledged by marking the recommendation
/// applied (their remediation is either manual or lives in another
/// subsystem, e.g. the Maintenance tab's backup action).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityAction {
    /// Prune the audit log to the policy retention window.
    PruneAudit,
    /// Prune the findings history to the policy retention window.
    PruneFindings,
}

/// A recommendation produced by the rules.
#[derive(Debug, Clone, PartialEq)]
pub struct RecommendationCandidate {
    pub rule: &'static str,
    pub severity: SecuritySeverity,
    pub title: String,
    pub detail: String,
    pub action: Option<SecurityAction>,
}

/// Ledger sizes used by the pruning rules.
#[derive(Debug, Clone, Copy, Default)]
pub struct SecurityLedgerStats {
    pub audit_entries: usize,
    pub findings_entries: usize,
}

/// Beyond this many audit entries, pruning is suggested.
const AUDIT_LEDGER_WARNING: usize = 5_000;
/// Beyond this many findings rows, history pruning is suggested.
const FINDINGS_LEDGER_WARNING: usize = 5_000;

fn candidate(
    rule: &'static str,
    severity: SecuritySeverity,
    title: impl Into<String>,
    detail: impl Into<String>,
) -> RecommendationCandidate {
    RecommendationCandidate {
        rule,
        severity,
        title: title.into(),
        detail: detail.into(),
        action: None,
    }
}

/// Runs the rules over a battery plus ledger sizes, ordered by severity
/// (critical first). Idempotent — no state is touched here.
pub fn recommend(
    checks: &[SecurityCheckResult],
    ledger: SecurityLedgerStats,
) -> Vec<RecommendationCandidate> {
    let mut out = Vec::new();

    for check in checks {
        if check.passed {
            continue;
        }
        let rec = rule_for(check);
        if let Some(rec) = rec {
            out.push(rec);
        }
    }

    if ledger.audit_entries >= AUDIT_LEDGER_WARNING {
        out.push(candidate(
            "prune_audit_log",
            SecuritySeverity::Info,
            "Prune the security audit log",
            format!(
                "The audit ledger holds {} entries — prune it to the configured retention window to keep history bounded.",
                ledger.audit_entries
            ),
        )
        .with_action(SecurityAction::PruneAudit));
    }
    if ledger.findings_entries >= FINDINGS_LEDGER_WARNING {
        out.push(candidate(
            "prune_findings_history",
            SecuritySeverity::Info,
            "Prune the findings history",
            format!(
                "The findings ledger holds {} rows — prune it to the configured retention window to keep history bounded.",
                ledger.findings_entries
            ),
        )
        .with_action(SecurityAction::PruneFindings));
    }

    out.sort_by_key(|rec| std::cmp::Reverse(rec.severity.weight()));
    out
}

fn rule_for(check: &SecurityCheckResult) -> Option<RecommendationCandidate> {
    match (check.check_name.as_str(), check.category) {
        ("journal_mode", SecurityCategory::Database) => Some(candidate(
            "enable_wal_journal",
            SecuritySeverity::Warning,
            "Enable the WAL journal mode",
            "The database is not running in WAL mode, which the application expects and which improves crash resilience.",
        )),
        ("foreign_keys", SecurityCategory::Database) => Some(candidate(
            "enable_foreign_keys",
            SecuritySeverity::Warning,
            "Enable foreign key enforcement",
            "Foreign keys are not being enforced — referential integrity violations can silently accumulate.",
        )),
        ("secure_delete", SecurityCategory::Database) => Some(candidate(
            "enable_secure_delete",
            SecuritySeverity::Info,
            "Enable SQLite secure_delete",
            "Deleted row content remains in freed pages; enabling secure_delete overwrites it before reuse.",
        )),
        ("db_file_permissions", SecurityCategory::Files) => Some(candidate(
            "restrict_db_file_permissions",
            SecuritySeverity::Warning,
            "Restrict database file permissions",
            "The database file is group/world-writable — restrict it to the current user with a mode such as 600 or 644.",
        )),
        ("backup_dir_permissions", SecurityCategory::Files) => Some(candidate(
            "restrict_backup_dir_permissions",
            SecuritySeverity::Info,
            "Restrict the backups directory permissions",
            "The backups directory is group/world-writable — snapshots there could be read or replaced by other users.",
        )),
        ("backup_file_permissions", SecurityCategory::Files) => Some(candidate(
            "restrict_backup_file_permissions",
            SecuritySeverity::Info,
            "Restrict backup file permissions",
            "The latest backup snapshot is group/world-writable.",
        )),
        ("api_key_storage", SecurityCategory::Secrets) => Some(candidate(
            "move_api_key_to_keychain",
            SecuritySeverity::Critical,
            "Move the API key to the OS keychain",
            "The API key is stored as plaintext in the database. Save it through the settings once so the keyring stores it: the database then holds only a marker.",
        )),
        ("secret_store_probe", SecurityCategory::Secrets) => Some(candidate(
            "verify_os_keychain",
            SecuritySeverity::Warning,
            "Verify the OS secret store",
            &check.detail,
        )),
        ("backup_presence", SecurityCategory::Backup) if check.severity == SecuritySeverity::Warning => {
            Some(candidate(
                "rebuild_backup",
                SecuritySeverity::Warning,
                "Recreate the missing backup snapshot",
                "The latest recorded backup file is missing from disk — create a fresh snapshot from the Maintenance tab.",
            ))
        }
        ("backup_checksum", SecurityCategory::Backup) if check.severity == SecuritySeverity::Critical => {
            Some(candidate(
                "rebuild_backup_checksum",
                SecuritySeverity::Critical,
                "Rebuild the tampered backup snapshot",
                "The latest backup does not match its recorded checksum — treat it as untrusted and create a fresh snapshot.",
            ))
        }
        ("backup_checksum", SecurityCategory::Backup) => Some(candidate(
            "rebuild_backup_checksum",
            SecuritySeverity::Warning,
            "Verify the backup checksum",
            "The latest backup could not be checksum-verified.",
        )),
        ("monitor_interval_config", SecurityCategory::Config) => Some(candidate(
            "fix_monitor_interval_config",
            SecuritySeverity::Warning,
            "Fix the monitor interval setting",
            &check.detail,
        )),
        ("audit_retention_config", SecurityCategory::Config) => Some(candidate(
            "fix_audit_retention_config",
            SecuritySeverity::Warning,
            "Fix the audit retention setting",
            &check.detail,
        )),
        ("path_absolute", SecurityCategory::Input) => Some(candidate(
            "use_absolute_db_path",
            SecuritySeverity::Warning,
            "Use an absolute database path",
            "The configured database path is not absolute.",
        )),
        ("path_nul", SecurityCategory::Input) => Some(candidate(
            "fix_db_path_nul",
            SecuritySeverity::Warning,
            "Fix the database path",
            "The configured database path contains a NUL byte and cannot be a real path.",
        )),
        _ => None,
    }
}

impl RecommendationCandidate {
    fn with_action(mut self, action: SecurityAction) -> Self {
        self.action = Some(action);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::security::SecurityCheckResult;

    fn check(
        name: &str,
        category: SecurityCategory,
        severity: SecuritySeverity,
        passed: bool,
        detail: &str,
    ) -> SecurityCheckResult {
        SecurityCheckResult {
            check_name: name.to_string(),
            category,
            severity,
            passed,
            detail: detail.to_string(),
        }
    }

    #[test]
    fn all_passing_checks_produce_no_recommendations() {
        let checks = vec![
            check(
                "journal_mode",
                SecurityCategory::Database,
                SecuritySeverity::Info,
                true,
                "",
            ),
            check(
                "api_key_storage",
                SecurityCategory::Secrets,
                SecuritySeverity::Info,
                true,
                "",
            ),
            check(
                "secure_delete",
                SecurityCategory::Database,
                SecuritySeverity::Info,
                true,
                "",
            ),
        ];
        assert!(recommend(&checks, SecurityLedgerStats::default()).is_empty());
    }

    #[test]
    fn failed_checks_map_to_rules() {
        let checks = vec![
            check(
                "db_file_permissions",
                SecurityCategory::Files,
                SecuritySeverity::Warning,
                false,
                "writable",
            ),
            check(
                "monitor_interval_config",
                SecurityCategory::Config,
                SecuritySeverity::Warning,
                false,
                "bad",
            ),
        ];
        let recs = recommend(&checks, SecurityLedgerStats::default());
        let rules: Vec<_> = recs.iter().map(|r| r.rule).collect();
        assert!(rules.contains(&"restrict_db_file_permissions"));
        assert!(rules.contains(&"fix_monitor_interval_config"));
    }

    #[test]
    fn api_key_plaintext_is_critical_and_backup_mismatch_carries_no_action() {
        let checks = vec![check(
            "api_key_storage",
            SecurityCategory::Secrets,
            SecuritySeverity::Critical,
            false,
            "plaintext",
        )];
        let recs = recommend(&checks, SecurityLedgerStats::default());
        assert_eq!(recs[0].rule, "move_api_key_to_keychain");
        assert_eq!(recs[0].severity, SecuritySeverity::Critical);
        assert!(
            recs[0].action.is_none(),
            "keychain migration is a manual/settings action"
        );

        let tampered = vec![check(
            "backup_checksum",
            SecurityCategory::Backup,
            SecuritySeverity::Critical,
            false,
            "mismatch",
        )];
        let recs = recommend(&tampered, SecurityLedgerStats::default());
        assert_eq!(recs[0].rule, "rebuild_backup_checksum");
    }

    #[test]
    fn oversized_ledgers_suggest_bounded_pruning_actions() {
        let ledger = SecurityLedgerStats {
            audit_entries: 5_001,
            findings_entries: 6_000,
        };
        let recs = recommend(&[], ledger);
        let prune_audit = recs.iter().find(|r| r.rule == "prune_audit_log");
        let prune_findings = recs.iter().find(|r| r.rule == "prune_findings_history");
        assert_eq!(
            prune_audit.map(|r| r.action),
            Some(Some(SecurityAction::PruneAudit))
        );
        assert_eq!(
            prune_findings.map(|r| r.action),
            Some(Some(SecurityAction::PruneFindings))
        );

        assert!(recommend(&[], SecurityLedgerStats::default()).is_empty());
    }

    #[test]
    fn recommendations_sort_critical_first() {
        let checks = vec![
            check(
                "db_file_permissions",
                SecurityCategory::Files,
                SecuritySeverity::Warning,
                false,
                "",
            ),
            check(
                "api_key_storage",
                SecurityCategory::Secrets,
                SecuritySeverity::Critical,
                false,
                "",
            ),
        ];
        let recs = recommend(&checks, SecurityLedgerStats::default());
        assert_eq!(recs[0].rule, "move_api_key_to_keychain");
        assert_eq!(recs[1].rule, "restrict_db_file_permissions");
    }
}
