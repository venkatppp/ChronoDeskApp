//! Security Hardening engine (RC-10 M4 — Production Hardening).
//!
//! Facade over the security subsystems: the pure check functions
//! ([`checks`]), the stateful validator that gathers environment facts
//! ([`validator`]), the 0..100 scorer ([`scoring`]), the pure
//! recommendation rules ([`recommendations`]), the policy table
//! ([`policy`]), and the audit ledger ([`audit`]). `lib.rs` wires one
//! [`SecurityEngine`] as managed Tauri state; the
//! [`crate::commands::security`] commands are thin forwards to it.
//!
//! Layout mirrors `maintenance` (RC-10 M3): the engine composes
//! repositories (SQL) and models (DTOs); commands stay thin. The SQL for
//! the four ledgers lives in
//! [`crate::repositories::SecurityRepository`]. The engine runs the
//! non-fatal startup validation, the background monitor loop, and the
//! user-triggered diagnostics/recommendation/config surfaces.

pub mod audit;
pub mod checks;
pub mod policy;
pub mod recommendations;
pub mod scoring;
pub mod validator;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use uuid::Uuid;

use crate::app_events::{AppEventEmitter, EVENT_SECURITY_STATUS};
use crate::errors::DatabaseError;
use crate::llm::SecretStore;
use crate::models::security::{
    PermissionsReport, SecretValidationReport, SecurityAuditEntry, SecurityConfigEntry,
    SecurityDiagnosticsReport, SecurityFinding, SecurityRecommendation,
    SecurityRecommendationStatus, SecurityScoreReport, SecuritySeverity, StartupValidationReport,
};
use crate::repositories::{LLMRepository, MaintenanceRepository, SecurityRepository};

pub use audit::AuditService;
pub use recommendations::{RecommendationCandidate, SecurityAction};

/// Facade for all security hardening operations.
#[derive(Clone)]
pub struct SecurityEngine {
    repository: SecurityRepository,
    validator: validator::SecurityValidator,
    audit_service: AuditService,
    emitter: Option<Arc<dyn AppEventEmitter>>,
}

impl SecurityEngine {
    /// Constructs the engine. `maintenance_repository` is shared with the
    /// M3 engine so the backup ledger remains single-owned (one repository,
    /// two readers). `secret_store` is the same keyring-backed store the
    /// LLM settings use.
    pub fn new(
        repository: SecurityRepository,
        maintenance_repository: Arc<MaintenanceRepository>,
        llm_repository: Arc<LLMRepository>,
        secret_store: Arc<dyn SecretStore>,
        db_path: PathBuf,
        backup_dir: PathBuf,
    ) -> Self {
        let validator = validator::SecurityValidator::new(
            repository.clone(),
            maintenance_repository,
            llm_repository,
            secret_store,
            db_path,
            backup_dir,
        );
        let audit_service = AuditService::new(repository.clone());
        Self {
            repository,
            validator,
            audit_service,
            emitter: None,
        }
    }

    /// Wires a real event emitter (the `tauri::AppHandle` in `lib.rs`);
    /// without it the engine simply skips emitting.
    pub fn with_event_emitter(mut self, emitter: Arc<dyn AppEventEmitter>) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// The configured database path (read-only; used by diagnostics).
    pub fn db_path(&self) -> &std::path::Path {
        self.validator.db_path()
    }

    /// The background monitor interval from the policy table.
    pub async fn monitor_interval_seconds(&self) -> u64 {
        match self
            .repository
            .config_get(policy::KEY_MONITOR_INTERVAL_SECONDS)
            .await
        {
            Ok(value) => policy::resolve_monitor_interval(value)
                .unwrap_or(policy::DEFAULT_MONITOR_INTERVAL_SECONDS),
            Err(_) => policy::DEFAULT_MONITOR_INTERVAL_SECONDS,
        }
    }

    // ------------------------------------------------------------------
    // Batteries & status
    // ------------------------------------------------------------------

    /// The non-fatal startup validation pass. Runs the full battery,
    /// persists findings, refreshes recommendations, and audits the run —
    /// then returns the summary. Failures inside never abort startup
    /// (callers log and continue).
    pub async fn startup_validation(&self) -> Result<StartupValidationReport, DatabaseError> {
        let run_id = Uuid::new_v4().to_string();
        let report = self.validator.run_full(&run_id).await?;
        self.persist_recommendations(&report.checks).await;

        let failed = report.total_checks.saturating_sub(report.passed_checks);
        let severity = if failed > 0 {
            SecuritySeverity::Warning
        } else {
            SecuritySeverity::Info
        };
        self.audit_service
            .record(
                "startup_validation",
                severity,
                "system",
                "startup",
                &format!(
                    "{} checks, {} passed, score {:.0}",
                    report.total_checks, report.passed_checks, report.score
                ),
            )
            .await?;

        Ok(StartupValidationReport {
            validated_at: Utc::now(),
            run_id,
            ok: failed == 0,
            score: report.score,
            total_checks: report.total_checks,
            passed_checks: report.passed_checks,
            failed_checks: failed,
        })
    }

    /// A fresh user-triggered full battery.
    pub async fn diagnostics(&self) -> Result<SecurityDiagnosticsReport, DatabaseError> {
        let run_id = Uuid::new_v4().to_string();
        let report = self.validator.run_full(&run_id).await?;
        self.persist_recommendations(&report.checks).await;
        self.audit_service
            .record(
                "diagnostics_run",
                SecuritySeverity::Info,
                "user",
                "diagnostics",
                &format!("{} checks, score {:.0}", report.total_checks, report.score),
            )
            .await?;
        Ok(report)
    }

    /// The current posture: the latest full battery's findings replayed
    /// into a [`SecurityScoreReport`]. No fresh scan — cheap to call.
    pub async fn status(&self) -> Result<SecurityScoreReport, DatabaseError> {
        let Some(latest) = self.repository.recent_findings(1).await?.into_iter().next() else {
            return Ok(scoring::empty_report());
        };
        let findings = self.repository.findings_by_run(&latest.run_id).await?;
        Ok(scoring::report_from_findings(findings))
    }

    /// Focused secret/config-validation battery (live probe, not persisted).
    pub async fn secrets(&self) -> Result<SecretValidationReport, DatabaseError> {
        self.validator.run_secrets().await
    }

    /// Focused file/database/backup permission battery (live probe).
    pub async fn permissions(&self) -> Result<PermissionsReport, DatabaseError> {
        self.validator.run_permissions().await
    }

    /// The findings history ledger, newest-first.
    pub async fn history(&self, limit: u32) -> Result<Vec<SecurityFinding>, DatabaseError> {
        self.repository.recent_findings(limit.clamp(1, 500)).await
    }

    // ------------------------------------------------------------------
    // Audit log
    // ------------------------------------------------------------------

    /// The most recent audit entries.
    pub async fn audit_log(&self, limit: u32) -> Result<Vec<SecurityAuditEntry>, DatabaseError> {
        self.audit_service.recent(limit).await
    }

    // ------------------------------------------------------------------
    // Policy config
    // ------------------------------------------------------------------

    /// Every policy entry.
    pub async fn config(&self) -> Result<Vec<SecurityConfigEntry>, DatabaseError> {
        self.repository.config_all().await
    }

    /// Validates and persists a policy value, then audits the change.
    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        policy::validate_config(key, value).map_err(DatabaseError::InvalidInput)?;
        self.repository.config_set(key, value).await?;
        self.audit_service
            .record(
                "config_set",
                SecuritySeverity::Info,
                "user",
                key,
                &format!("{key} → {value}"),
            )
            .await?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Recommendations
    // ------------------------------------------------------------------

    /// The persisted recommendations, newest-first.
    pub async fn recommendations(&self) -> Result<Vec<SecurityRecommendation>, DatabaseError> {
        self.repository.recommendations().await
    }

    /// Applies a recommendation: executes its safe action (if any), marks
    /// it applied, and audits the decision.
    pub async fn apply_recommendation(
        &self,
        id: i64,
    ) -> Result<SecurityRecommendation, DatabaseError> {
        let recommendation = self
            .repository
            .recommendation_by_id(id)
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "security recommendation",
                id: id.to_string(),
            })?;

        self.execute_recommendation_action(&recommendation).await;

        self.repository
            .update_recommendation_status(id, SecurityRecommendationStatus::Applied)
            .await?;
        self.audit_service
            .record(
                "recommendation_apply",
                SecuritySeverity::Info,
                "user",
                &format!("recommendation:{id}"),
                &recommendation.rule,
            )
            .await?;

        self.repository
            .recommendation_by_id(id)
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "security recommendation",
                id: id.to_string(),
            })
    }

    /// Dismisses a recommendation and audits the decision.
    pub async fn dismiss_recommendation(
        &self,
        id: i64,
    ) -> Result<SecurityRecommendation, DatabaseError> {
        let recommendation = self
            .repository
            .recommendation_by_id(id)
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "security recommendation",
                id: id.to_string(),
            })?;

        self.repository
            .update_recommendation_status(id, SecurityRecommendationStatus::Dismissed)
            .await?;
        self.audit_service
            .record(
                "recommendation_dismiss",
                SecuritySeverity::Info,
                "user",
                &format!("recommendation:{id}"),
                &recommendation.rule,
            )
            .await?;

        self.repository
            .recommendation_by_id(id)
            .await?
            .ok_or_else(|| DatabaseError::NotFound {
                entity: "security recommendation",
                id: id.to_string(),
            })
    }

    // ------------------------------------------------------------------
    // Monitor
    // ------------------------------------------------------------------

    /// One monitor pass: full battery → persisted findings → score →
    /// recommendations → bounded pruning → optional `security:status`
    /// event. Non-fatal pass over pass.
    pub async fn monitor_tick(&self) -> Result<SecurityScoreReport, DatabaseError> {
        let run_id = Uuid::new_v4().to_string();
        let report = self.validator.run_full(&run_id).await?;
        self.persist_recommendations(&report.checks).await;

        self.prune_ledgers().await;
        self.audit_service
            .record(
                "monitor_tick",
                SecuritySeverity::Info,
                "monitor",
                "monitor",
                &format!("score {:.0}", report.score),
            )
            .await?;

        let score_report = self
            .status()
            .await
            .unwrap_or_else(|_| scoring::empty_report());
        if let Some(emitter) = &self.emitter {
            crate::app_events::emit(&**emitter, EVENT_SECURITY_STATUS, &score_report);
        }
        Ok(score_report)
    }

    /// The background monitor/recurring loop. Reads the interval from the
    /// policy table on every pass so a config change takes effect without
    /// a restart. Runs forever; each pass failure is logged and the loop
    /// continues.
    pub async fn run_monitor_loop(&self) {
        loop {
            let seconds = self.monitor_interval_seconds().await.max(10);
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            match self.monitor_tick().await {
                Ok(report) => tracing::info!(
                    score = report.score,
                    checks = report.total_checks,
                    "scheduled security monitor pass completed"
                ),
                Err(error) => {
                    tracing::warn!(error = %error, "scheduled security monitor pass failed")
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Internals
    // ------------------------------------------------------------------

    /// Upserts the recommendation candidates for a battery. Keeps an
    /// applied/dismissed status intact across runs (repository upsert).
    async fn persist_recommendations(
        &self,
        checks: &[crate::models::security::SecurityCheckResult],
    ) {
        let counts = match (
            self.repository.count_audit().await,
            self.repository.count_findings().await,
        ) {
            (Ok(audit), Ok(findings)) => recommendations::SecurityLedgerStats {
                audit_entries: audit as usize,
                findings_entries: findings as usize,
            },
            _ => recommendations::SecurityLedgerStats::default(),
        };

        for candidate in recommendations::recommend(checks, counts) {
            if let Err(error) = self
                .repository
                .upsert_recommendation(
                    candidate.rule,
                    candidate.severity,
                    &candidate.title,
                    &candidate.detail,
                )
                .await
            {
                tracing::warn!(rule = candidate.rule, error = %error, "recommendation upsert failed");
            }
        }
    }

    /// Executes the safe, engine-owned action a recommendation maps to.
    async fn execute_recommendation_action(&self, recommendation: &SecurityRecommendation) {
        let action = match recommendation.rule.as_str() {
            "prune_audit_log" => Some(SecurityAction::PruneAudit),
            "prune_findings_history" => Some(SecurityAction::PruneFindings),
            _ => None,
        };
        let Ok(action) = action.ok_or(()) else {
            return;
        };

        match action {
            SecurityAction::PruneAudit => {
                let retention = self
                    .retention_days(
                        policy::KEY_AUDIT_RETENTION_DAYS,
                        policy::DEFAULT_AUDIT_RETENTION_DAYS,
                    )
                    .await;
                match self.audit_service.prune(retention).await {
                    Ok(removed) => tracing::info!(removed, "audit log pruned by recommendation"),
                    Err(error) => tracing::warn!(error = %error, "audit prune failed"),
                }
            }
            SecurityAction::PruneFindings => {
                let retention = self
                    .retention_days(
                        policy::KEY_FINDINGS_RETENTION_DAYS,
                        policy::DEFAULT_FINDINGS_RETENTION_DAYS,
                    )
                    .await;
                let cutoff = Utc::now() - chrono::Duration::days(retention.max(1));
                match self.repository.prune_findings_older_than(cutoff).await {
                    Ok(removed) => {
                        tracing::info!(removed, "findings history pruned by recommendation")
                    }
                    Err(error) => tracing::warn!(error = %error, "findings prune failed"),
                }
            }
        }
    }

    /// Resolves a retention-window policy value with a fallback.
    async fn retention_days(&self, key: &str, default: i64) -> i64 {
        match self.repository.config_get(key).await {
            Ok(value) => policy::resolve_retention_days(value, default).unwrap_or(default),
            Err(_) => default,
        }
    }

    /// Best-effort bounded-ledger housekeeping per the policy window.
    async fn prune_ledgers(&self) {
        let audit = self
            .retention_days(
                policy::KEY_AUDIT_RETENTION_DAYS,
                policy::DEFAULT_AUDIT_RETENTION_DAYS,
            )
            .await;
        if let Err(error) = self.audit_service.prune(audit).await {
            tracing::warn!(error = %error, "audit retention prune failed");
        }
        let findings = self
            .retention_days(
                policy::KEY_FINDINGS_RETENTION_DAYS,
                policy::DEFAULT_FINDINGS_RETENTION_DAYS,
            )
            .await;
        let cutoff = Utc::now() - chrono::Duration::days(findings.max(1));
        if let Err(error) = self.repository.prune_findings_older_than(cutoff).await {
            tracing::warn!(error = %error, "findings retention prune failed");
        }
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
