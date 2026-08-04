//! Security audit service (RC-10 M4).
//!
//! Thin, deliberate wrapper over the `security_audit_log` table's append
//! and prune operations. Keeping it behind a named service (rather than
//! commands calling the repository directly) matches the layering rule —
//! commands reach the engine, and the engine owns the ledger lifecycle —
//! and gives one place for the retention-window policy used by the
//! monitor and the prune recommendation.

use chrono::{Duration, Utc};

use crate::errors::DatabaseError;
use crate::models::security::{SecurityAuditEntry, SecuritySeverity};
use crate::repositories::SecurityRepository;

/// Service that owns audit-ledger lifecycle (append + retention prune).
#[derive(Clone)]
pub struct AuditService {
    repository: SecurityRepository,
}

impl AuditService {
    pub fn new(repository: SecurityRepository) -> Self {
        Self { repository }
    }

    /// Appends one audit entry.
    pub async fn record(
        &self,
        action: &str,
        severity: SecuritySeverity,
        actor: &str,
        target: &str,
        detail: &str,
    ) -> Result<i64, DatabaseError> {
        self.repository
            .audit(action, severity, actor, target, detail)
            .await
    }

    /// The most recent audit entries, newest-first.
    pub async fn recent(&self, limit: u32) -> Result<Vec<SecurityAuditEntry>, DatabaseError> {
        self.repository.recent_audit(limit.clamp(1, 500)).await
    }

    /// Prunes everything older than `retention_days`. Best-effort audit
    /// housekeeping — a failure is logged by the caller, never fatal.
    pub async fn prune(&self, retention_days: i64) -> Result<u64, DatabaseError> {
        let cutoff = Utc::now() - Duration::days(retention_days.max(1));
        self.repository.prune_audit_older_than(cutoff).await
    }
}
