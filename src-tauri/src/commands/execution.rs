//! Execution IPC Commands - Plan execution control and monitoring

use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::copilot::execution_engine::ExecutionEngine;
use crate::copilot::proactive_models::ExecutionPlan;
use crate::copilot::ExecutionProgress;

/// Starts execution of an approved plan.
#[tauri::command]
pub async fn execution_start(
    engine: State<'_, Arc<ExecutionEngine>>,
    plan: ExecutionPlan,
    conversation_id: Option<String>,
) -> Result<String, String> {
    let cid = conversation_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;

    let execution_id = engine
        .start_execution(&plan, cid)
        .await
        .map_err(|e| e.to_string())?;

    // Start async execution
    let engine_clone = engine.inner().clone();
    tokio::spawn(async move {
        let _ = engine_clone.execute_until_complete(execution_id).await;
    });

    Ok(execution_id.to_string())
}

/// Pauses a running execution.
#[tauri::command]
pub async fn execution_pause(
    engine: State<'_, Arc<ExecutionEngine>>,
    execution_id: String,
) -> Result<(), String> {
    let eid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;
    engine.pause_execution(eid).await.map_err(|e| e.to_string())
}

/// Resumes a paused execution.
#[tauri::command]
pub async fn execution_resume(
    engine: State<'_, Arc<ExecutionEngine>>,
    execution_id: String,
) -> Result<(), String> {
    let eid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;

    // Resume async execution
    let engine_clone = engine.inner().clone();
    tokio::spawn(async move {
        if engine_clone.resume_execution(eid).await.is_ok() {
            let _ = engine_clone.execute_until_complete(eid).await;
        }
    });

    Ok(())
}

/// Cancels a running execution.
#[tauri::command]
pub async fn execution_cancel(
    engine: State<'_, Arc<ExecutionEngine>>,
    execution_id: String,
) -> Result<(), String> {
    let eid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;
    engine
        .cancel_execution(eid)
        .await
        .map_err(|e| e.to_string())
}

/// Gets execution progress.
#[tauri::command]
pub async fn execution_get_progress(
    engine: State<'_, Arc<ExecutionEngine>>,
    execution_id: String,
) -> Result<ExecutionProgress, String> {
    let eid = Uuid::parse_str(&execution_id).map_err(|e| e.to_string())?;
    engine.get_progress(eid).await.map_err(|e| e.to_string())
}

/// Lists most recently updated executions with full progress, so the
/// dashboard can re-attach to an in-flight or last-completed run after a
/// reload/restart (reconnect: fetch current state, then resubscribe).
#[tauri::command]
pub async fn execution_list_recent(
    engine: State<'_, Arc<ExecutionEngine>>,
    limit: Option<usize>,
) -> Result<Vec<ExecutionProgress>, String> {
    engine
        .list_recent(limit.unwrap_or(10))
        .await
        .map_err(|e| e.to_string())
}
