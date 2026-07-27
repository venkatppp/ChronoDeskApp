//! System-level commands.
//!
//! These exist in Phase 1 purely to prove the IPC path between the React
//! frontend and the Rust backend works end-to-end, before any real engine
//! (workspace detection, timeline, etc.) exists to expose. Later phases
//! add commands here that simply delegate to the relevant engine module —
//! command handlers should stay thin and never contain business logic.

use serde::Serialize;

/// Returns the backend crate's semantic version, as declared in `Cargo.toml`.
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub ok: bool,
    pub backend_version: String,
}

/// Lightweight readiness probe the frontend can call on startup to confirm
/// the Tauri backend is alive before it starts issuing real IPC calls.
#[tauri::command]
pub fn health_check() -> HealthStatus {
    HealthStatus {
        ok: true,
        backend_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_app_version_matches_cargo_manifest() {
        assert_eq!(get_app_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn health_check_reports_ok() {
        let status = health_check();
        assert!(status.ok);
        assert_eq!(status.backend_version, env!("CARGO_PKG_VERSION"));
    }
}
