//! File Watcher IPC commands: manage which directories ChronoDesk
//! watches, persisting the list (via [`SettingsRepository`]) so it's
//! restored automatically on the next launch — see
//! [`restore_watched_paths`], called once from `lib.rs`'s `setup()`.

use std::path::PathBuf;

use tauri::State;

use crate::errors::{DatabaseError, WatcherError};
use crate::repositories::SettingsRepository;
use crate::watcher::FileWatcher;

/// Settings key the watched-paths list is persisted under, as a
/// JSON-encoded `Vec<String>`.
const WATCHED_PATHS_SETTINGS_KEY: &str = "watched_paths";

/// Starts watching `path` and persists the updated watch list.
///
/// # Errors
/// [`WatcherError::InvalidPath`] if `path` doesn't exist or isn't a
/// directory; [`WatcherError::AlreadyWatching`] if it's already watched.
#[tauri::command]
pub async fn add_watch_path(
    watcher: State<'_, FileWatcher>,
    settings: State<'_, SettingsRepository>,
    path: PathBuf,
) -> Result<(), WatcherError> {
    watcher.watch(path.clone()).await?;
    persist_watched_paths(&settings, watcher.watched_paths().await).await?;

    tracing::info!(path = %path.display(), "watch path added");
    Ok(())
}

/// Stops watching `path` and persists the updated watch list.
///
/// # Errors
/// [`WatcherError::NotWatching`] if `path` isn't currently watched.
#[tauri::command]
pub async fn remove_watch_path(
    watcher: State<'_, FileWatcher>,
    settings: State<'_, SettingsRepository>,
    path: PathBuf,
) -> Result<(), WatcherError> {
    watcher.unwatch(&path).await?;
    persist_watched_paths(&settings, watcher.watched_paths().await).await?;

    tracing::info!(path = %path.display(), "watch path removed");
    Ok(())
}

/// Lists every directory currently being watched.
#[tauri::command]
pub async fn list_watch_paths(
    watcher: State<'_, FileWatcher>,
) -> Result<Vec<PathBuf>, WatcherError> {
    Ok(watcher.watched_paths().await)
}

/// Serializes the current watch list and upserts it into `settings`.
async fn persist_watched_paths(
    settings: &SettingsRepository,
    paths: Vec<PathBuf>,
) -> Result<(), WatcherError> {
    let string_paths: Vec<String> = paths
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let json = serde_json::to_string(&string_paths).expect("Vec<String> serialization cannot fail");

    settings.set(WATCHED_PATHS_SETTINGS_KEY, &json).await?;
    Ok(())
}

/// Loads the persisted watch list and starts watching each path. Called
/// once from `lib.rs`'s `setup()`, after the database and `FileWatcher`
/// are both ready.
///
/// A path that no longer exists (e.g. an external drive that isn't
/// currently connected, or a directory deleted since the last run) is
/// logged and skipped rather than failing — one bad path must never
/// prevent the application from launching. Likewise, a corrupt setting
/// value is logged and treated as "no watched paths" rather than
/// propagated as a startup error.
pub async fn restore_watched_paths(
    watcher: &FileWatcher,
    settings: &SettingsRepository,
) -> Result<(), DatabaseError> {
    let Some(raw) = settings.get(WATCHED_PATHS_SETTINGS_KEY).await? else {
        tracing::debug!("no persisted watch paths to restore");
        return Ok(());
    };

    let paths: Vec<String> = match serde_json::from_str(&raw) {
        Ok(paths) => paths,
        Err(err) => {
            tracing::error!(error = %err, "corrupt watched_paths setting, treating as empty");
            return Ok(());
        }
    };

    for path in paths {
        let path_buf = PathBuf::from(&path);
        match watcher.watch(path_buf).await {
            Ok(()) => tracing::info!(path, "restored watch path"),
            Err(err) => {
                tracing::warn!(path, error = %err, "failed to restore watch path, skipping")
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::repositories::{FileRepository, TimelineRepository, WorkspaceRepository};
    use crate::services::{TimelineService, WorkspaceService};
    use crate::timeline::recorder::TimelineRecorder;
    use crate::timeline::TimelineEngine;
    use crate::workspace::WorkspaceManager;
    use tempfile::tempdir;

    async fn test_watcher_and_settings() -> (
        FileWatcher,
        SettingsRepository,
        sqlx::SqlitePool,
        tempfile::TempDir,
    ) {
        let (database, db_guard) = test_database().await;
        let pool = database.pool().clone();
        let workspace_manager = WorkspaceManager::new(WorkspaceService::new(
            WorkspaceRepository::new(pool.clone()),
            TimelineRepository::new(pool.clone()),
        ));
        let timeline_repository = TimelineRepository::new(pool.clone());
        let timeline_engine = TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(
                FileRepository::new(pool.clone()),
                timeline_repository.clone(),
            ),
            timeline_repository,
        ));
        let watcher = FileWatcher::new(workspace_manager, timeline_engine);
        let settings = SettingsRepository::new(pool.clone());

        (watcher, settings, pool, db_guard)
    }

    fn watcher_from_pool(pool: &sqlx::SqlitePool) -> FileWatcher {
        let workspace_manager = WorkspaceManager::new(WorkspaceService::new(
            WorkspaceRepository::new(pool.clone()),
            TimelineRepository::new(pool.clone()),
        ));
        let timeline_repository = TimelineRepository::new(pool.clone());
        let timeline_engine = TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(
                FileRepository::new(pool.clone()),
                timeline_repository.clone(),
            ),
            timeline_repository,
        ));
        FileWatcher::new(workspace_manager, timeline_engine)
    }

    #[tokio::test]
    async fn persist_and_restore_round_trip() {
        let (watcher, settings, pool, _db_guard) = test_watcher_and_settings().await;
        let root_a = tempdir().unwrap();
        let root_b = tempdir().unwrap();
        let ca =
            std::fs::canonicalize(root_a.path()).unwrap_or_else(|_| root_a.path().to_path_buf());
        let cb =
            std::fs::canonicalize(root_b.path()).unwrap_or_else(|_| root_b.path().to_path_buf());

        watcher.watch(root_a.path().to_path_buf()).await.unwrap();
        watcher.watch(root_b.path().to_path_buf()).await.unwrap();
        persist_watched_paths(&settings, watcher.watched_paths().await)
            .await
            .unwrap();

        // Simulate a fresh launch: a brand-new FileWatcher (nothing
        // watched yet) built against the same underlying database,
        // restored purely from the persisted setting — exactly the shape
        // `lib.rs`'s `setup()` uses in production.
        let fresh_watcher = watcher_from_pool(&pool);
        restore_watched_paths(&fresh_watcher, &settings)
            .await
            .unwrap();

        let mut restored = fresh_watcher.watched_paths().await;
        restored.sort();
        let mut expected = vec![ca, cb];
        expected.sort();
        assert_eq!(restored, expected);
    }

    #[tokio::test]
    async fn restore_with_no_persisted_paths_is_a_no_op() {
        let (watcher, settings, _pool, _db_guard) = test_watcher_and_settings().await;

        restore_watched_paths(&watcher, &settings).await.unwrap();
        assert!(watcher.watched_paths().await.is_empty());
    }

    #[tokio::test]
    async fn restore_skips_a_path_that_no_longer_exists() {
        let (watcher, settings, _pool, _db_guard) = test_watcher_and_settings().await;
        settings
            .set(
                WATCHED_PATHS_SETTINGS_KEY,
                "[\"/definitely/not/a/real/path\"]",
            )
            .await
            .unwrap();

        // Must not error/panic — the bad path is logged and skipped.
        restore_watched_paths(&watcher, &settings).await.unwrap();
        assert!(watcher.watched_paths().await.is_empty());
    }

    #[tokio::test]
    async fn restore_treats_corrupt_json_as_no_paths() {
        let (watcher, settings, _pool, _db_guard) = test_watcher_and_settings().await;
        settings
            .set(WATCHED_PATHS_SETTINGS_KEY, "not valid json")
            .await
            .unwrap();

        restore_watched_paths(&watcher, &settings).await.unwrap();
        assert!(watcher.watched_paths().await.is_empty());
    }
}
