//! RC-10 M4 security hardening IPC commands.
//!
//! Thin wrappers only: every command pulls the [`SecurityEngine`] state
//! and forwards to its facade method. Zero business logic lives here —
//! check policy, scoring, and recommendation rules live in
//! [`crate::security`] and the SQL in
//! [`crate::repositories::SecurityRepository`].

use tauri::State;

use crate::errors::DatabaseError;
use crate::models::security::{
    PermissionsReport, SecretValidationReport, SecurityAuditEntry, SecurityConfigEntry,
    SecurityDiagnosticsReport, SecurityFinding, SecurityRecommendation, SecurityScoreReport,
};
use crate::security::SecurityEngine;

/// The current 0..100 security posture, replayed from the latest run.
#[tauri::command]
pub async fn security_status(
    engine: State<'_, SecurityEngine>,
) -> Result<SecurityScoreReport, DatabaseError> {
    engine.status().await
}

/// A fresh full battery of checks against the live environment.
#[tauri::command]
pub async fn security_diagnostics(
    engine: State<'_, SecurityEngine>,
) -> Result<SecurityDiagnosticsReport, DatabaseError> {
    engine.diagnostics().await
}

/// The focused secret/config-handling battery (live probe).
#[tauri::command]
pub async fn security_secrets(
    engine: State<'_, SecurityEngine>,
) -> Result<SecretValidationReport, DatabaseError> {
    engine.secrets().await
}

/// The focused file/database/backup permission battery (live probe).
#[tauri::command]
pub async fn security_permissions(
    engine: State<'_, SecurityEngine>,
) -> Result<PermissionsReport, DatabaseError> {
    engine.permissions().await
}

/// The findings history ledger, newest-first.
#[tauri::command]
pub async fn security_history(
    engine: State<'_, SecurityEngine>,
    limit: Option<u32>,
) -> Result<Vec<SecurityFinding>, DatabaseError> {
    engine.history(limit.unwrap_or(50)).await
}

/// The most recent security audit entries.
#[tauri::command]
pub async fn security_audit_log(
    engine: State<'_, SecurityEngine>,
    limit: Option<u32>,
) -> Result<Vec<SecurityAuditEntry>, DatabaseError> {
    engine.audit_log(limit.unwrap_or(50)).await
}

/// Every security policy entry.
#[tauri::command]
pub async fn security_config(
    engine: State<'_, SecurityEngine>,
) -> Result<Vec<SecurityConfigEntry>, DatabaseError> {
    engine.config().await
}

/// Validates and persists a security policy value.
#[tauri::command]
pub async fn security_set_config(
    engine: State<'_, SecurityEngine>,
    key: String,
    value: String,
) -> Result<(), DatabaseError> {
    engine.set_config(&key, &value).await
}

/// The persisted security recommendations, newest-first.
#[tauri::command]
pub async fn security_recommendations(
    engine: State<'_, SecurityEngine>,
) -> Result<Vec<SecurityRecommendation>, DatabaseError> {
    engine.recommendations().await
}

/// Applies a recommendation (executes its safe action, if any).
#[tauri::command]
pub async fn security_apply_recommendation(
    engine: State<'_, SecurityEngine>,
    id: i64,
) -> Result<SecurityRecommendation, DatabaseError> {
    engine.apply_recommendation(id).await
}

/// Dismisses a recommendation.
#[tauri::command]
pub async fn security_dismiss_recommendation(
    engine: State<'_, SecurityEngine>,
    id: i64,
) -> Result<SecurityRecommendation, DatabaseError> {
    engine.dismiss_recommendation(id).await
}
