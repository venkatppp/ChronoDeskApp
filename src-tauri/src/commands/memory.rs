//! Execution Memory IPC Commands - search, recommend, and inspect what
//! ChronoDesk has learned from previous executions (RC-6 M1).
//!
//! Thin wrappers around the shared `MemoryEngine`; no business logic lives
//! here.

use std::sync::Arc;

use tauri::State;
use uuid::Uuid;

use crate::copilot::memory::{
    AvoidedStrategy, LearnedWorkflow, MemoryEngine, MemoryHit, MemoryKind, MemoryRecommendation,
    MemorySearchRequest, MemoryStats, MemoryStatus,
};

/// Searches remembered runs by goal similarity, with optional filters.
#[tauri::command]
pub async fn memory_search(
    engine: State<'_, Arc<MemoryEngine>>,
    query: String,
    kind: Option<MemoryKind>,
    workspace_id: Option<String>,
    status: Option<MemoryStatus>,
    limit: Option<usize>,
) -> Result<Vec<MemoryHit>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let request = MemorySearchRequest {
        query,
        kind,
        workspace_id,
        status,
        limit: limit.unwrap_or(10),
    };
    engine.search(&request).await.map_err(|e| e.to_string())
}

/// Recommends previously successful workflows for a goal, ranked by the
/// learning blend.
#[tauri::command]
pub async fn memory_recommend(
    engine: State<'_, Arc<MemoryEngine>>,
    goal: String,
    workspace_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<MemoryRecommendation>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    engine
        .recommend(&goal, workspace_id, limit.unwrap_or(5))
        .await
        .map_err(|e| e.to_string())
}

/// Retrieves failed/cancelled strategies relevant to a goal — what the
/// runtime should avoid repeating.
#[tauri::command]
pub async fn memory_avoid(
    engine: State<'_, Arc<MemoryEngine>>,
    goal: String,
    workspace_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<AvoidedStrategy>, String> {
    let workspace_id = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    engine
        .avoid(&goal, workspace_id, limit.unwrap_or(5))
        .await
        .map_err(|e| e.to_string())
}

/// Aggregated workflows learned from repeated executions.
#[tauri::command]
pub async fn memory_learned_workflows(
    engine: State<'_, Arc<MemoryEngine>>,
) -> Result<Vec<LearnedWorkflow>, String> {
    engine.learned_workflows().await.map_err(|e| e.to_string())
}

/// Dashboard statistics over the memory store.
#[tauri::command]
pub async fn memory_stats(engine: State<'_, Arc<MemoryEngine>>) -> Result<MemoryStats, String> {
    engine.stats().await.map_err(|e| e.to_string())
}
