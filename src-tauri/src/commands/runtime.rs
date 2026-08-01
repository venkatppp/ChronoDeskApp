//! Runtime status commands for frontend diagnostics.

use tauri::State;

use crate::runtime::{DiagnosticsService, RuntimeDiagnostics, RuntimeHealth, RuntimeHealthService};

/// Gets runtime health status.
#[tauri::command]
pub async fn get_runtime_health(
    health_service: State<'_, RuntimeHealthService>,
) -> Result<RuntimeHealth, String> {
    Ok(health_service.get_health().await)
}

/// Gets comprehensive runtime diagnostics.
#[tauri::command]
pub async fn get_runtime_diagnostics(
    diagnostics_service: State<'_, DiagnosticsService>,
) -> Result<RuntimeDiagnostics, String> {
    Ok(diagnostics_service.get_diagnostics().await)
}

/// Gets a human-readable runtime summary.
#[tauri::command]
pub async fn get_runtime_summary(
    diagnostics_service: State<'_, DiagnosticsService>,
) -> Result<String, String> {
    Ok(diagnostics_service.get_summary().await)
}
