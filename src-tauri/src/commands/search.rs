use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::search::{SavedSearch, SearchEntityType, SearchResult, SearchStats};
use crate::services::SearchService;

/// Performs a search across indexed entities.
#[tauri::command]
pub async fn search(
    _app: AppHandle,
    service: State<'_, SearchService>,
    query: String,
    entity_types: Option<Vec<SearchEntityType>>,
    workspace_id: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<SearchResult>, DatabaseError> {
    let limit = limit.unwrap_or(20);
    let entity_types = entity_types.unwrap_or_default();
    service
        .search(&query, &entity_types, workspace_id, limit)
        .await
}

/// Fetches the most recent search queries for auto-complete.
#[tauri::command]
pub async fn get_search_history(
    service: State<'_, SearchService>,
    limit: Option<i64>,
) -> Result<Vec<String>, DatabaseError> {
    let limit = limit.unwrap_or(10);
    service.get_search_history(limit).await
}

/// Records a search query in history.
#[tauri::command]
pub async fn save_search_query(
    service: State<'_, SearchService>,
    query: String,
) -> Result<(), DatabaseError> {
    service.save_search_query(&query).await
}

/// Clears the entire search history.
#[tauri::command]
pub async fn clear_search_history(service: State<'_, SearchService>) -> Result<(), DatabaseError> {
    service.clear_search_history().await
}

/// Persists a search query.
#[tauri::command]
pub async fn save_search(
    service: State<'_, SearchService>,
    query: String,
) -> Result<SavedSearch, DatabaseError> {
    service.save_search(&query).await
}

/// Lists all saved searches.
#[tauri::command]
pub async fn list_saved_searches(
    service: State<'_, SearchService>,
) -> Result<Vec<SavedSearch>, DatabaseError> {
    service.list_saved_searches().await
}

/// Deletes a saved search.
#[tauri::command]
pub async fn delete_saved_search(
    service: State<'_, SearchService>,
    id: Uuid,
) -> Result<(), DatabaseError> {
    service.delete_saved_search(id).await
}

/// Returns the most recently updated files in a workspace.
#[tauri::command]
pub async fn get_recent_files(
    service: State<'_, SearchService>,
    workspace_id: Uuid,
    limit: Option<i64>,
) -> Result<Vec<SearchResult>, DatabaseError> {
    let limit = limit.unwrap_or(10);
    service.get_recent_files(workspace_id, limit).await
}

/// Returns search-related statistics for a workspace.
#[tauri::command]
pub async fn get_workspace_stats(
    service: State<'_, SearchService>,
    workspace_id: Uuid,
) -> Result<SearchStats, DatabaseError> {
    service.get_workspace_stats(workspace_id).await
}
