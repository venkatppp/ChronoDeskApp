//! Duplicate detection IPC commands (Phase 5 Stage 2).
//!
//! Exposes the duplicate detection engine to the frontend via Tauri commands.
//! All commands are thin wrappers around the DuplicateDetectionEngine, following
//! the same pattern as other command modules.

use tauri::State;
use uuid::Uuid;

use crate::duplicates::{DuplicateDetectionEngine, DuplicateGroup, ScanProgress};
use crate::errors::DatabaseError;

/// Scans a workspace for duplicate files, hashing any unhashed files.
///
/// This is an incremental, resumable scan. If cancelled or interrupted,
/// calling it again will only hash files that still need it. Progress is
/// emitted via the "duplicates:scan-progress" event.
///
/// # Frontend usage
/// ```typescript
/// const progress = await invoke('scan_workspace_for_duplicates', {
///   workspaceId: 'uuid'
/// });
/// ```
#[tauri::command]
pub async fn scan_workspace_for_duplicates(
    workspace_id: String,
    engine: State<'_, DuplicateDetectionEngine>,
) -> Result<ScanProgress, DatabaseError> {
    let id = Uuid::parse_str(&workspace_id)
        .map_err(|e| DatabaseError::InvalidInput(format!("invalid workspace_id: {}", e)))?;

    engine
        .hash_workspace_incremental(id)
        .await
        .map_err(|e| DatabaseError::InvalidInput(e.to_string()))
}

/// Hashes a single file and updates its content_hash.
///
/// # Frontend usage
/// ```typescript
/// const hash = await invoke('scan_file', {
///   fileId: 'uuid',
///   path: '/path/to/file.txt'
/// });
/// ```
#[tauri::command]
pub async fn scan_file(
    file_id: String,
    path: String,
    engine: State<'_, DuplicateDetectionEngine>,
) -> Result<String, DatabaseError> {
    let id = Uuid::parse_str(&file_id)
        .map_err(|e| DatabaseError::InvalidInput(format!("invalid file_id: {}", e)))?;

    engine
        .hash_single_file(id, path)
        .await
        .map_err(|e| DatabaseError::InvalidInput(e.to_string()))
}

/// Gets all duplicate file groups in a workspace (or all workspaces).
///
/// Only returns groups with 2+ files. Groups are ordered by file count
/// (largest duplicate sets first).
///
/// # Frontend usage
/// ```typescript
/// // All workspaces
/// const groups = await invoke('get_duplicate_groups', { workspaceId: null });
///
/// // Specific workspace
/// const groups = await invoke('get_duplicate_groups', {
///   workspaceId: 'uuid'
/// });
/// ```
#[tauri::command]
pub async fn get_duplicate_groups(
    workspace_id: Option<String>,
    engine: State<'_, DuplicateDetectionEngine>,
) -> Result<Vec<DuplicateGroup>, DatabaseError> {
    let id = workspace_id
        .map(|s| {
            Uuid::parse_str(&s)
                .map_err(|e| DatabaseError::InvalidInput(format!("invalid workspace_id: {}", e)))
        })
        .transpose()?;

    engine
        .get_duplicate_groups(id)
        .await
        .map_err(|e| DatabaseError::InvalidInput(e.to_string()))
}

/// Finds files that are duplicates of a specific file.
///
/// This is a convenience wrapper that filters get_duplicate_groups to only
/// return the group containing the specified file (if any).
///
/// # Frontend usage
/// ```typescript
/// const duplicates = await invoke('find_duplicates', { fileId: 'uuid' });
/// ```
#[tauri::command]
pub async fn find_duplicates(
    file_id: String,
    engine: State<'_, DuplicateDetectionEngine>,
) -> Result<Option<DuplicateGroup>, DatabaseError> {
    let id = Uuid::parse_str(&file_id)
        .map_err(|e| DatabaseError::InvalidInput(format!("invalid file_id: {}", e)))?;

    // Get all duplicate groups
    let groups = engine
        .get_duplicate_groups(None)
        .await
        .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

    // Find the group containing this file
    let result = groups
        .into_iter()
        .find(|group| group.files.iter().any(|f| f.id == id));

    Ok(result)
}

/// Gets the current scan progress, if a scan is in progress.
///
/// Returns None if no scan is running.
///
/// # Frontend usage
/// ```typescript
/// const progress = await invoke('get_scan_progress');
/// if (progress) {
///   console.log(`${progress.filesScanned} / ${progress.totalFiles}`);
/// }
/// ```
#[tauri::command]
pub async fn get_scan_progress(
    engine: State<'_, DuplicateDetectionEngine>,
) -> Result<Option<ScanProgress>, DatabaseError> {
    Ok(engine.get_scan_progress().await)
}

/// Cancels an ongoing scan.
///
/// The scan will stop after completing the current file. Already-hashed
/// files remain hashed, so resuming will skip them.
///
/// # Frontend usage
/// ```typescript
/// await invoke('cancel_scan');
/// ```
#[tauri::command]
pub async fn cancel_scan(engine: State<'_, DuplicateDetectionEngine>) -> Result<(), DatabaseError> {
    engine.cancel_scan().await;
    Ok(())
}
