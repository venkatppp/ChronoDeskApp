//! Autonomous Agent Runtime IPC Commands - start, observe, and control
//! autonomous sessions over the shared planner + execution engine.
//!
//! Same discipline as `commands::execution`: thin wrappers around one engine.
//! All session mutations return a fresh `AutonomousSessionProgress` snapshot
//! so the UI can always render latest state, and live updates stream over
//! the `autonomous:session` / `autonomous:reasoning` events.

use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::copilot::autonomous::{AutonomousRuntime, AutonomousSessionProgress, ExecutionPolicy};

/// Starts an autonomous session for a goal. Returns the initial progress
/// snapshot; the reason–act–observe loop runs detached and streams events.
#[tauri::command]
pub async fn autonomous_start(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    goal: String,
    workspace_id: Option<String>,
    policy: Option<ExecutionPolicy>,
) -> Result<AutonomousSessionProgress, String> {
    let wid = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    runtime
        .start_session(wid, &goal, policy)
        .await
        .map_err(|e| e.to_string())
}

/// Current progress snapshot for one session.
#[tauri::command]
pub async fn autonomous_get_progress(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.get_progress(sid).await.map_err(|e| e.to_string())
}

/// Recent sessions (newest first) so the UI can list + re-attach after a
/// reload/restart.
#[tauri::command]
pub async fn autonomous_list_recent(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    limit: Option<usize>,
) -> Result<Vec<AutonomousSessionProgress>, String> {
    Ok(runtime.list_recent(limit.unwrap_or(10)).await)
}

/// Pauses a running session.
#[tauri::command]
pub async fn autonomous_pause(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.pause_session(sid).await.map_err(|e| e.to_string())
}

/// Resumes a paused session.
#[tauri::command]
pub async fn autonomous_resume(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.resume_session(sid).await.map_err(|e| e.to_string())
}

/// Cancels a session, propagating to the active engine run.
#[tauri::command]
pub async fn autonomous_cancel(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime.cancel_session(sid).await.map_err(|e| e.to_string())
}

/// Approves a pending approval checkpoint.
#[tauri::command]
pub async fn autonomous_approve(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
    note: Option<String>,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime
        .approve_session(sid, note)
        .await
        .map_err(|e| e.to_string())
}

/// Rejects a pending approval checkpoint (terminates the session).
#[tauri::command]
pub async fn autonomous_reject(
    runtime: State<'_, Arc<AutonomousRuntime>>,
    session_id: String,
    note: Option<String>,
) -> Result<AutonomousSessionProgress, String> {
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    runtime
        .reject_session(sid, note)
        .await
        .map_err(|e| e.to_string())
}
