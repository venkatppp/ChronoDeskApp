//! Pure security check functions (RC-10 M4).
//!
//! Every check here is a pure function of its inputs — no SQL, no I/O —
//! so each one is trivially unit-testable in isolation. The stateful
//! [`crate::security::validator::SecurityValidator`] gathers the inputs
//! (PRAGMA values, file metadata, secret-store state, ledger checksums)
//! and folds them through these functions into a battery of
//! [`SecurityCheckResult`]s.
//!
//! The input/path validation primitives at the bottom are the reusable
//! surface for "path inputs validated at the system boundary" (mirroring
//! the M3 `vacuum_into` path rule) — surfaced in the battery as checks
//! and also usable directly by the config command.

use crate::llm::ApiKeyStorageState;
use crate::models::security::{SecurityCategory, SecurityCheckResult, SecuritySeverity};

/// Wraps a verdict into a result DTO.
fn result(
    name: &str,
    category: SecurityCategory,
    severity: SecuritySeverity,
    passed: bool,
    detail: impl Into<String>,
) -> SecurityCheckResult {
    SecurityCheckResult {
        check_name: name.to_string(),
        category,
        severity,
        passed,
        detail: detail.into(),
    }
}

// ----------------------------------------------------------------------
// Database configuration checks
// ----------------------------------------------------------------------

/// `PRAGMA journal_mode` must be `wal` (the connection factory configures
/// WAL; any other value is a misconfiguration worth surfacing).
pub fn check_journal_mode(mode: &str) -> SecurityCheckResult {
    let wal = mode.eq_ignore_ascii_case("wal");
    result(
        "journal_mode",
        SecurityCategory::Database,
        SecuritySeverity::Warning,
        wal,
        if wal {
            format!("journal_mode = {mode}")
        } else {
            format!("journal_mode = {mode} — WAL is expected")
        },
    )
}

/// `PRAGMA foreign_keys` must be ON; SQLite silently ignores FK
/// constraints without it.
pub fn check_foreign_keys(enforced: bool) -> SecurityCheckResult {
    result(
        "foreign_keys",
        SecurityCategory::Database,
        SecuritySeverity::Warning,
        enforced,
        if enforced {
            "foreign_keys = on"
        } else {
            "foreign_keys = off — referential integrity is not enforced"
        },
    )
}

/// `PRAGMA trusted_schema` — informational only; either value is
/// tolerated (off is a stricter posture used when untrusted schema is a
/// concern). The security layer never mutes or flips the connection's
/// setting, so both verdicts pass with the current state reported.
pub fn check_trusted_schema(value: i64) -> SecurityCheckResult {
    let on = value != 0;
    result(
        "trusted_schema",
        SecurityCategory::Database,
        SecuritySeverity::Info,
        true,
        if on {
            "trusted_schema = on (default)"
        } else {
            "trusted_schema = off (stricter schema evaluation)"
        },
    )
}

/// `PRAGMA secure_delete` — advisory: deleted row content remains in
/// freed pages unless `secure_delete` is on/fast. Off is not a score
/// impact but earns a recommendation.
pub fn check_secure_delete(value: i64) -> SecurityCheckResult {
    let on = value != 0;
    result(
        "secure_delete",
        SecurityCategory::Database,
        SecuritySeverity::Info,
        on,
        match value {
            0 => "secure_delete = off — deleted content is not overwritten",
            1 => "secure_delete = on",
            _ => "secure_delete = fast",
        },
    )
}

// ----------------------------------------------------------------------
// File permission checks
// ----------------------------------------------------------------------

/// The unix permission bits that are dangerous on an app's private
/// database: group-writable (0o020) and world-writable (0o002).
const DANGEROUS_MODEBITS: u32 = 0o022;

/// Hmm-unix platforms report `None` for the permission mode; nothing to
/// enforce there.
fn mode_safe(mode: Option<u32>) -> bool {
    match mode {
        Some(mode) => mode & DANGEROUS_MODEBITS == 0,
        None => true,
    }
}

/// The database file must not be group/world-writable.
pub fn check_db_file_permissions(mode: Option<u32>) -> SecurityCheckResult {
    let safe = mode_safe(mode);
    result(
        "db_file_permissions",
        SecurityCategory::Files,
        SecuritySeverity::Warning,
        safe,
        mode.map_or_else(
            || "no unix permission mode available".to_string(),
            |mode| {
                if safe {
                    format!("database file permissions ok (mode {mode:o})")
                } else {
                    format!(
                        "database file is group/world-writable (mode {mode:o}) — restrict to the current user"
                    )
                }
            },
        ),
    )
}

/// The backups directory must not be group/world-writable. Absent (no
/// backups dir yet) passes — there is nothing to protect.
pub fn check_backup_dir_permissions(exists: bool, mode: Option<u32>) -> SecurityCheckResult {
    if !exists {
        return result(
            "backup_dir_permissions",
            SecurityCategory::Files,
            SecuritySeverity::Info,
            true,
            "no backups directory yet — nothing to protect",
        );
    }
    let safe = mode_safe(mode);
    result(
        "backup_dir_permissions",
        SecurityCategory::Files,
        SecuritySeverity::Info,
        safe,
        mode.map_or_else(
            || "backups directory exists".to_string(),
            |mode| {
                if safe {
                    format!("backups directory permissions ok (mode {mode:o})")
                } else {
                    format!(
                        "backups directory is group/world-writable (mode {mode:o}) — restrict to the current user"
                    )
                }
            },
        ),
    )
}

/// The latest backup file must not be group/world-writable. Absent (no
/// backup yet) passes.
pub fn check_latest_backup_permissions(exists: bool, mode: Option<u32>) -> SecurityCheckResult {
    if !exists {
        return result(
            "backup_file_permissions",
            SecurityCategory::Files,
            SecuritySeverity::Info,
            true,
            "no backup file to check",
        );
    }
    let safe = mode_safe(mode);
    result(
        "backup_file_permissions",
        SecurityCategory::Files,
        SecuritySeverity::Info,
        safe,
        mode.map_or_else(
            || "latest backup exists".to_string(),
            |mode| {
                if safe {
                    format!("latest backup permissions ok (mode {mode:o})")
                } else {
                    format!(
                        "latest backup is group/world-writable (mode {mode:o}) — restrict to the current user"
                    )
                }
            },
        ),
    )
}

// ----------------------------------------------------------------------
// Secret & config checks
// ----------------------------------------------------------------------

/// Verifies how the LLM API key is stored. The actual key value is never
/// read here — only its storage state (see
/// [`crate::repositories::LLMRepository::api_key_storage_state`]).
pub fn check_api_key_storage(state: ApiKeyStorageState) -> SecurityCheckResult {
    match state {
        ApiKeyStorageState::Secure => result(
            "api_key_storage",
            SecurityCategory::Secrets,
            SecuritySeverity::Info,
            true,
            "API key stored in the OS secret store",
        ),
        ApiKeyStorageState::None => result(
            "api_key_storage",
            SecurityCategory::Secrets,
            SecuritySeverity::Info,
            true,
            "no API key configured",
        ),
        ApiKeyStorageState::Plaintext => result(
            "api_key_storage",
            SecurityCategory::Secrets,
            SecuritySeverity::Critical,
            false,
            "API key stored as plaintext in the database — move it to the OS keychain",
        ),
        ApiKeyStorageState::SecretStoreUnavailable => result(
            "api_key_storage",
            SecurityCategory::Secrets,
            SecuritySeverity::Warning,
            false,
            "API key is expected in the OS secret store but it cannot be read",
        ),
    }
}

/// Validates the OS secret backend itself with a unique ephemeral write
/// → read → delete round-trip. Reports the backend error verbatim (never
/// a secret value).
pub fn check_secret_store_probe(ok: bool, error: Option<String>) -> SecurityCheckResult {
    result(
        "secret_store_probe",
        SecurityCategory::Secrets,
        SecuritySeverity::Warning,
        ok,
        if ok {
            "OS secret store round-trip verified".to_string()
        } else {
            format!(
                "OS secret store unavailable: {}",
                error.unwrap_or_else(|| "unknown error".to_string())
            )
        },
    )
}

// ----------------------------------------------------------------------
// Backup verification checks
// ----------------------------------------------------------------------

/// Whether a successful backup snapshot exists on disk today.
pub fn check_backup_presence(recorded: bool, exists: bool) -> SecurityCheckResult {
    if !recorded {
        return result(
            "backup_presence",
            SecurityCategory::Backup,
            SecuritySeverity::Info,
            true,
            "no backup has been created yet",
        );
    }
    if exists {
        return result(
            "backup_presence",
            SecurityCategory::Backup,
            SecuritySeverity::Info,
            true,
            "latest backup snapshot present on disk",
        );
    }
    result(
        "backup_presence",
        SecurityCategory::Backup,
        SecuritySeverity::Warning,
        false,
        "latest backup is recorded but its file is missing from disk",
    )
}

/// Re-hashes the latest backup file and compares it to the M3 audit
/// ledger's stored checksum, so a tampered or partially-written snapshot
/// is caught before it is ever relied on.
pub fn check_backup_checksum(
    recorded: bool,
    expected: Option<&str>,
    computed: Option<&str>,
) -> SecurityCheckResult {
    if !recorded {
        return result(
            "backup_checksum",
            SecurityCategory::Backup,
            SecuritySeverity::Info,
            true,
            "no backup has been created yet",
        );
    }
    let Some(expected) = expected else {
        return result(
            "backup_checksum",
            SecurityCategory::Backup,
            SecuritySeverity::Warning,
            false,
            "latest backup ledger row carries no checksum",
        );
    };
    let Some(computed) = computed else {
        return result(
            "backup_checksum",
            SecurityCategory::Backup,
            SecuritySeverity::Critical,
            false,
            "latest backup file could not be hashed — is it readable?",
        );
    };
    let ok = expected == computed;
    result(
        "backup_checksum",
        SecurityCategory::Backup,
        SecuritySeverity::Critical,
        ok,
        if ok {
            format!(
                "backup checksum verified ({})",
                &computed[..computed.len().min(12)]
            )
        } else {
            format!("backup checksum mismatch: expected {expected}, computed {computed}")
        },
    )
}

// ----------------------------------------------------------------------
// Path / input checks
// ----------------------------------------------------------------------

/// The database path must be absolute.
pub fn check_path_absolute(path: &str) -> SecurityCheckResult {
    let ok = is_absolute(path);
    result(
        "path_absolute",
        SecurityCategory::Input,
        SecuritySeverity::Warning,
        ok,
        if ok {
            "database path is absolute"
        } else {
            "database path is relative — an absolute path is required"
        },
    )
}

/// The database path must not contain a NUL byte.
pub fn check_path_has_nul(path: &str) -> SecurityCheckResult {
    let ok = !has_nul(path);
    result(
        "path_nul",
        SecurityCategory::Input,
        SecuritySeverity::Warning,
        ok,
        if ok {
            "database path contains no NUL bytes"
        } else {
            "database path contains a NUL byte"
        },
    )
}

// ----------------------------------------------------------------------
// Config policy checks
// ----------------------------------------------------------------------

/// The monitor-interval policy value, when set, must parse and fall in
/// the allowed band.
pub fn check_monitor_interval_config(value: Option<&str>, valid: bool) -> SecurityCheckResult {
    result(
        "monitor_interval_config",
        SecurityCategory::Config,
        SecuritySeverity::Warning,
        valid,
        match value {
            None => format!(
                "no custom monitor interval set — using {}s",
                crate::security::policy::DEFAULT_MONITOR_INTERVAL_SECONDS
            ),
            Some(v) if valid => format!("monitor interval configured ({v}s)"),
            Some(v) => format!("monitor interval {v:?} is invalid — expected 10..=3600"),
        },
    )
}

/// The audit-retention policy value, when set, must parse and fall in the
/// allowed band.
pub fn check_audit_retention_config(value: Option<&str>, valid: bool) -> SecurityCheckResult {
    result(
        "audit_retention_config",
        SecurityCategory::Config,
        SecuritySeverity::Warning,
        valid,
        match value {
            None => format!(
                "no custom audit retention set — using {} days",
                crate::security::policy::DEFAULT_AUDIT_RETENTION_DAYS
            ),
            Some(v) if valid => format!("audit retention configured ({v} days)"),
            Some(v) => format!("audit retention {v:?} is invalid — expected 1..=3650"),
        },
    )
}

// ----------------------------------------------------------------------
// Path validation primitives
// ----------------------------------------------------------------------

/// Whether the path is absolute (POSIX root or Windows drive/UNC prefix).
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with("\\\\")
        || path.as_bytes().get(1).is_some_and(|second| *second == b':')
}

/// Whether the path contains a NUL byte (rejected outright — it cannot
/// appear in a real path and truncates at the OS-call boundary).
pub fn has_nul(path: &str) -> bool {
    path.as_bytes().contains(&0)
}

/// Whether the path contains a `..` component that escapes its parent
/// (path-traversal guard for boundary inputs).
pub fn has_traversal(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.split('/').any(|component| component == "..")
}

/// Full boundary rule for a path-ish input: absolute, NUL-free and with
/// no `..` traversal component.
pub fn validate_path(path: &str) -> Result<(), String> {
    if has_nul(path) {
        return Err("path must not contain a NUL byte".to_string());
    }
    if !is_absolute(path) {
        return Err("path must be absolute".to_string());
    }
    if has_traversal(path) {
        return Err("path must not contain '..' components".to_string());
    }
    Ok(())
}

/// The default constructor for the reporting type, so callers can build
/// a check result for an unhandled rule without reaching into internals.
pub fn unchecked(name: &str, category: SecurityCategory, detail: &str) -> SecurityCheckResult {
    result(name, category, SecuritySeverity::Info, true, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_mode_must_be_wal() {
        assert!(check_journal_mode("wal").passed);
        assert!(!check_journal_mode("delete").passed);
        assert!(!check_journal_mode("MEMORY").passed);
    }

    #[test]
    fn foreign_keys_and_trusted_schema() {
        assert!(check_foreign_keys(true).passed);
        assert!(!check_foreign_keys(false).passed);
        // trusted_schema is informational both ways.
        assert!(check_trusted_schema(1).passed);
        assert!(check_trusted_schema(0).passed);
        assert!(check_trusted_schema(0).detail.contains("off"));
    }

    #[test]
    fn secure_delete_off_is_advisory_only() {
        let off = check_secure_delete(0);
        assert!(!off.passed);
        assert_eq!(off.severity, SecuritySeverity::Info, "no score impact");
        assert!(check_secure_delete(1).passed);
        assert!(check_secure_delete(2).passed);
    }

    #[test]
    fn file_permission_checks_report_group_and_world_writable() {
        // 0o644 is safe; 0o664 (group writable) and 0o666 (world) are not.
        assert!(check_db_file_permissions(Some(0o644)).passed);
        assert!(!check_db_file_permissions(Some(0o664)).passed);
        assert!(!check_db_file_permissions(Some(0o666)).passed);
        // Non-unix: no mode, nothing to enforce.
        assert!(check_db_file_permissions(None).passed);

        assert!(check_backup_dir_permissions(false, None).passed);
        assert!(!check_backup_dir_permissions(true, Some(0o777)).passed);

        assert!(check_latest_backup_permissions(false, None).passed);
        assert!(!check_latest_backup_permissions(true, Some(0o662)).passed);
    }

    #[test]
    fn api_key_storage_maps_all_states() {
        assert!(check_api_key_storage(ApiKeyStorageState::Secure).passed);
        assert!(check_api_key_storage(ApiKeyStorageState::None).passed);
        assert!(!check_api_key_storage(ApiKeyStorageState::Plaintext).passed);
        assert_eq!(
            check_api_key_storage(ApiKeyStorageState::Plaintext).severity,
            SecuritySeverity::Critical
        );
        assert!(!check_api_key_storage(ApiKeyStorageState::SecretStoreUnavailable).passed);
        assert_eq!(
            check_api_key_storage(ApiKeyStorageState::SecretStoreUnavailable).severity,
            SecuritySeverity::Warning
        );
    }

    #[test]
    fn secret_store_probe_reports_backend_errors_not_values() {
        assert!(check_secret_store_probe(true, None).passed);
        let failed = check_secret_store_probe(false, Some("keychain locked".to_string()));
        assert!(!failed.passed);
        assert!(failed.detail.contains("keychain locked"));
    }

    #[test]
    fn backup_presence_and_checksum_policies() {
        assert!(check_backup_presence(false, false).passed, "no backup = ok");
        assert!(check_backup_presence(true, true).passed);
        assert!(!check_backup_presence(true, false).passed);

        assert!(check_backup_checksum(false, None, None).passed);
        assert!(check_backup_checksum(true, Some("abc"), Some("abc")).passed);
        assert!(!check_backup_checksum(true, Some("abc"), Some("def")).passed);
        assert!(
            !check_backup_checksum(true, None, None).passed,
            "missing ledger checksum"
        );
        assert!(
            !check_backup_checksum(true, Some("abc"), None).passed,
            "unreadable file"
        );
    }

    #[test]
    fn path_validation_primitives_reject_bad_inputs() {
        assert!(is_absolute("/abs/dir"));
        assert!(is_absolute("C:/windows"));
        assert!(!is_absolute("relative/path"));

        assert!(has_nul("bad\0path"));
        assert!(!has_nul("ok/path"));

        assert!(has_traversal("../../etc/passwd"));
        assert!(has_traversal("/a/../b"));
        assert!(!has_traversal("/a/b"));

        assert!(validate_path("/a/b").is_ok());
        assert!(validate_path("relative").is_err());
        assert!(validate_path("/a/\0b").is_err());
        assert!(validate_path("/a/../b").is_err());
    }
}
