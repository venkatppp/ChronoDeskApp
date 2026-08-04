//! Security policy configuration (RC-10 M4).
//!
//! Owns the key names, defaults, and value-validation rules behind the
//! `security_config` table. The monitor and the config command validate
//! every value through these pure functions before it is persisted, so a
//! bad threshold can never be written.

/// Key for the background monitor interval, in seconds.
pub const KEY_MONITOR_INTERVAL_SECONDS: &str = "security.monitor_interval_seconds";
/// Key for the audit-log retention window, in days.
pub const KEY_AUDIT_RETENTION_DAYS: &str = "security.audit_retention_days";
/// Key for the findings-history retention window, in days.
pub const KEY_FINDINGS_RETENTION_DAYS: &str = "security.findings_retention_days";

/// All policy keys, for the CLI surface (Config tab / config command).
pub const POLICY_KEYS: [&str; 3] = [
    KEY_MONITOR_INTERVAL_SECONDS,
    KEY_AUDIT_RETENTION_DAYS,
    KEY_FINDINGS_RETENTION_DAYS,
];

/// Default monitor interval when unset (300 s = 5 min: heavy enough to
/// re-verify backup checksums, light enough for a desktop app).
pub const DEFAULT_MONITOR_INTERVAL_SECONDS: u64 = 300;
/// Default audit-log retention when unset (90 days).
pub const DEFAULT_AUDIT_RETENTION_DAYS: i64 = 90;
/// Default findings-history retention when unset (30 days).
pub const DEFAULT_FINDINGS_RETENTION_DAYS: i64 = 30;

/// Allowed band for the monitor interval.
const MONITOR_INTERVAL_MIN: u64 = 10;
const MONITOR_INTERVAL_MAX: u64 = 3600;
/// Allowed band for the retention windows.
const RETENTION_MIN: i64 = 1;
const RETENTION_MAX: i64 = 3650;

/// Validates a monitor-interval value: a whole number of seconds within
/// the allowed band.
pub fn validate_monitor_interval(value: &str) -> Result<u64, String> {
    let seconds: u64 = value
        .trim()
        .parse()
        .map_err(|_| format!("{value:?} is not a whole number of seconds"))?;
    if !(MONITOR_INTERVAL_MIN..=MONITOR_INTERVAL_MAX).contains(&seconds) {
        return Err(format!(
            "monitor interval must be between {MONITOR_INTERVAL_MIN} and {MONITOR_INTERVAL_MAX} seconds"
        ));
    }
    Ok(seconds)
}

/// Validates a retention-window value: whole days within the allowed band.
pub fn validate_retention_days(value: &str) -> Result<i64, String> {
    let days: i64 = value
        .trim()
        .parse()
        .map_err(|_| format!("{value:?} is not a whole number of days"))?;
    if !(RETENTION_MIN..=RETENTION_MAX).contains(&days) {
        return Err(format!(
            "retention must be between {RETENTION_MIN} and {RETENTION_MAX} days"
        ));
    }
    Ok(days)
}

/// The monitor interval to use given an optional configured value.
pub fn resolve_monitor_interval(value: Option<String>) -> Result<u64, String> {
    match value {
        None => Ok(DEFAULT_MONITOR_INTERVAL_SECONDS),
        Some(v) => validate_monitor_interval(&v),
    }
}

/// The audit-retention window to use given an optional configured value.
pub fn resolve_retention_days(value: Option<String>, default: i64) -> Result<i64, String> {
    match value {
        None => Ok(default),
        Some(v) => validate_retention_days(&v),
    }
}

/// Whether `key` is a known policy key.
pub fn is_known_key(key: &str) -> bool {
    POLICY_KEYS.contains(&key)
}

/// Validates a key/value pair for the config command. Rejects unknown
/// keys and out-of-band values.
pub fn validate_config(key: &str, value: &str) -> Result<(), String> {
    match key {
        KEY_MONITOR_INTERVAL_SECONDS => validate_monitor_interval(value).map(|_| ()),
        KEY_AUDIT_RETENTION_DAYS | KEY_FINDINGS_RETENTION_DAYS => {
            validate_retention_days(value).map(|_| ())
        }
        _ => Err(format!("unknown security policy key: {key}")),
    }
}

/// Human-readable label for a policy key, for the UI.
pub fn label(key: &str) -> &'static str {
    match key {
        KEY_MONITOR_INTERVAL_SECONDS => "Monitor interval (seconds)",
        KEY_AUDIT_RETENTION_DAYS => "Audit log retention (days)",
        KEY_FINDINGS_RETENTION_DAYS => "Findings history retention (days)",
        _ => "Security policy key",
    }
}

/// The default value for a policy key, when unset.
pub fn default_value(key: &str) -> Option<&'static str> {
    match key {
        KEY_MONITOR_INTERVAL_SECONDS => Some("300"),
        KEY_AUDIT_RETENTION_DAYS => Some("90"),
        KEY_FINDINGS_RETENTION_DAYS => Some("30"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_interval_band_is_enforced() {
        assert_eq!(validate_monitor_interval("300").expect("300s"), 300);
        assert_eq!(validate_monitor_interval("  60  ").expect("trimmed"), 60);
        assert!(validate_monitor_interval("9").is_err(), "below min");
        assert!(validate_monitor_interval("3601").is_err(), "above max");
        assert!(validate_monitor_interval("fast").is_err(), "not a number");
        assert!(validate_monitor_interval("").is_err());
    }

    #[test]
    fn retention_band_is_enforced() {
        assert_eq!(validate_retention_days("90").expect("90"), 90);
        assert!(validate_retention_days("0").is_err());
        assert!(validate_retention_days("3651").is_err());
        assert!(validate_retention_days("monthly").is_err());
    }

    #[test]
    fn resolve_falls_back_to_defaults() {
        assert_eq!(
            resolve_monitor_interval(None).expect("none"),
            DEFAULT_MONITOR_INTERVAL_SECONDS
        );
        assert_eq!(
            resolve_monitor_interval(Some("600".to_string())).expect("600"),
            600
        );
        assert!(resolve_monitor_interval(Some("bad".to_string())).is_err());
    }

    #[test]
    fn validate_config_rejects_unknown_keys() {
        assert!(validate_config(KEY_MONITOR_INTERVAL_SECONDS, "60").is_ok());
        assert!(validate_config("security.NOT_A_KEY", "60").is_err());
        assert!(validate_config(KEY_AUDIT_RETENTION_DAYS, "abc").is_err());
        assert!(is_known_key(KEY_FINDINGS_RETENTION_DAYS));
        assert!(!is_known_key("nope"));
    }
}
