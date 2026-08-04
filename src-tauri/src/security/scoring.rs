//! Security score (RC-10 M4).
//!
//! Pure: the 0..100 posture is a function of a battery's check results.
//! Only failed *warning* and *critical* checks reduce the score; info
//! findings are advisory (they still surface as recommendations) but do
//! not move the score, so a fresh install with advisory notes still reads
//! as secure rather than penalized.

use crate::models::security::{
    SecurityCheckResult, SecurityFinding, SecurityScoreReport, SecuritySeverity,
};
use chrono::Utc;

/// Penalty applied per failed check by severity.
const CRITICAL_PENALTY: f64 = 40.0;
const WARNING_PENALTY: f64 = 15.0;

/// Builds a [`SecurityScoreReport`] over a run's persisted findings.
/// `scored_at` is "now" — the findings carry their own timestamps.
pub fn report_from_findings(findings: Vec<SecurityFinding>) -> SecurityScoreReport {
    let total = findings.len();
    let passed = findings.iter().filter(|f| f.passed).count();
    let score = score_findings(findings.iter().map(|f| (f.severity, f.passed))).round();
    SecurityScoreReport {
        scored_at: Utc::now(),
        score,
        status: status_for(score).to_string(),
        total_checks: total,
        passed_checks: passed,
        failed_checks: total.saturating_sub(passed),
        findings,
    }
}

/// A default, full-score report for a machine with no run recorded yet.
pub fn empty_report() -> SecurityScoreReport {
    SecurityScoreReport {
        scored_at: Utc::now(),
        score: 100.0,
        status: "excellent".to_string(),
        total_checks: 0,
        passed_checks: 0,
        failed_checks: 0,
        findings: Vec::new(),
    }
}

/// Status labels by score band.
pub fn status_for(score: f64) -> &'static str {
    if score >= 90.0 {
        "excellent"
    } else if score >= 75.0 {
        "good"
    } else if score >= 50.0 {
        "fair"
    } else {
        "weak"
    }
}

/// Computes the `0..=100` score over a battery. Also returns the counts
/// (total / passed / failed) so callers can render without re-walking.
pub fn score(checks: &[SecurityCheckResult]) -> (f64, usize, usize, usize) {
    let total = checks.len();
    let mut penalty = 0.0;
    let mut passed = 0usize;
    for check in checks {
        if check.passed {
            passed += 1;
            continue;
        }
        penalty += match check.severity {
            SecuritySeverity::Critical => CRITICAL_PENALTY,
            SecuritySeverity::Warning => WARNING_PENALTY,
            SecuritySeverity::Info => 0.0,
        };
    }
    let failed = total - passed;
    let value = (100.0 - penalty).clamp(0.0, 100.0);
    (value, total, passed, failed)
}

/// Computes the score over a battery of findings (for status reads that
/// replay the latest persisted run).
pub fn score_findings(findings: impl Iterator<Item = (SecuritySeverity, bool)>) -> f64 {
    let mut penalty = 0.0;
    for (severity, passed) in findings {
        if passed {
            continue;
        }
        penalty += match severity {
            SecuritySeverity::Critical => CRITICAL_PENALTY,
            SecuritySeverity::Warning => WARNING_PENALTY,
            SecuritySeverity::Info => 0.0,
        };
    }
    (100.0 - penalty).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::security::{SecurityCategory, SecurityCheckResult, SecurityFinding};
    use chrono::Utc;

    fn check(severity: SecuritySeverity, passed: bool) -> SecurityCheckResult {
        SecurityCheckResult {
            check_name: "c".to_string(),
            category: SecurityCategory::Database,
            severity,
            passed,
            detail: String::new(),
        }
    }

    #[test]
    fn all_passing_is_a_full_score() {
        let checks = vec![
            check(SecuritySeverity::Info, true),
            check(SecuritySeverity::Warning, true),
            check(SecuritySeverity::Critical, true),
        ];
        let (value, total, passed, failed) = score(&checks);
        assert_eq!(value, 100.0);
        assert_eq!((total, passed, failed), (3, 3, 0));
        assert_eq!(status_for(value), "excellent");
    }

    #[test]
    fn failed_warnings_and_criticals_reduce_the_score() {
        let checks = vec![
            check(SecuritySeverity::Info, false), // advisory — no penalty
            check(SecuritySeverity::Warning, false),
            check(SecuritySeverity::Critical, false),
            check(SecuritySeverity::Info, true),
        ];
        let (value, _, _, _) = score(&checks);
        assert_eq!(value, 100.0 - 15.0 - 40.0);
    }

    #[test]
    fn score_is_clamped_to_zero() {
        let checks = vec![
            check(SecuritySeverity::Critical, false),
            check(SecuritySeverity::Critical, false),
            check(SecuritySeverity::Critical, false),
        ];
        let (value, _, _, _) = score(&checks);
        assert_eq!(value, 0.0);
    }

    #[test]
    fn status_bands_are_correct() {
        assert_eq!(status_for(100.0), "excellent");
        assert_eq!(status_for(89.0), "good");
        assert_eq!(status_for(74.0), "fair");
        assert_eq!(status_for(12.0), "weak");
    }

    #[test]
    fn score_findings_matches_score() {
        let pairs = [
            (SecuritySeverity::Warning, false),
            (SecuritySeverity::Critical, false),
            (SecuritySeverity::Info, true),
        ];
        let value = score_findings(pairs.iter().map(|(s, p)| (*s, *p)));
        assert_eq!(value, 100.0 - 15.0 - 40.0);
    }

    #[test]
    fn report_from_findings_tallies_and_scores() {
        let findings = vec![
            finding("a", SecuritySeverity::Warning, false),
            finding("b", SecuritySeverity::Critical, false),
            finding("c", SecuritySeverity::Info, true),
        ];
        let report = report_from_findings(findings);
        assert_eq!(report.total_checks, 3);
        assert_eq!(report.passed_checks, 1);
        assert_eq!(report.failed_checks, 2);
        assert_eq!(report.score, 45.0);
        assert_eq!(report.status, "weak");

        let empty = empty_report();
        assert_eq!(empty.score, 100.0);
        assert_eq!(empty.status, "excellent");
        assert_eq!(empty.findings.len(), 0);
    }

    fn finding(name: &str, severity: SecuritySeverity, passed: bool) -> SecurityFinding {
        SecurityFinding {
            id: 0,
            run_id: "run".to_string(),
            category: SecurityCategory::Database,
            severity,
            check_name: name.to_string(),
            passed,
            detail: String::new(),
            checked_at: Utc::now(),
        }
    }
}
