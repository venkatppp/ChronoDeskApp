//! [`FileWatcher`]: wraps `notify::RecommendedWatcher`, recursively
//! watching a directory and feeding normalized, debounced events into the
//! workspace-detection + timeline-recording pipeline. Runs entirely on
//! background tokio tasks — nothing here blocks the Tauri event loop, and
//! a failed OS-level watch reconnects automatically rather than silently
//! going dark.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::interval;

use crate::app_events::{self, AppEventEmitter, NoopEmitter};
use crate::errors::WatcherError;
use crate::timeline::{TimelineActivity, TimelineEngine};
use crate::workspace::WorkspaceManager;

use super::debounce::{DebouncedEvent, DebouncedEventKind, Debouncer};
use super::event_handler::normalize;

/// How long a path's events must be quiet before the debouncer emits a
/// coalesced event for it.
const DEBOUNCE_WINDOW: Duration = Duration::from_millis(500);

/// How often the background pipeline checks the debouncer for events
/// ready to flush. Independent of [`DEBOUNCE_WINDOW`] — just small enough
/// that flushed events don't lag noticeably behind the window elapsing.
const DEBOUNCE_TICK: Duration = Duration::from_millis(100);

/// Delay before attempting to re-establish a watch after the underlying
/// OS watch fails (e.g. a watched network volume disconnects).
const RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Watches directory trees and drives the full event pipeline:
/// notify → normalize → debounce → workspace detection → timeline
/// recording. One [`FileWatcher`] can hold any number of independent
/// watched roots simultaneously, each with its own background tasks.
#[derive(Clone)]
pub struct FileWatcher {
    workspace_manager: WorkspaceManager,
    timeline_engine: TimelineEngine,
    active: Arc<Mutex<HashMap<PathBuf, WatchHandle>>>,
    event_emitter: Arc<dyn AppEventEmitter>,
}

impl FileWatcher {
    pub fn new(workspace_manager: WorkspaceManager, timeline_engine: TimelineEngine) -> Self {
        Self {
            workspace_manager,
            timeline_engine,
            active: Arc::new(Mutex::new(HashMap::new())),
            event_emitter: Arc::new(NoopEmitter),
        }
    }

    /// Swaps in a real event emitter (e.g. a [`tauri::AppHandle`]) so the
    /// pipeline's "EventEmitter → Frontend" stage actually reaches a
    /// window. Defaults to a no-op emitter from [`FileWatcher::new`] so
    /// every existing test — none of which need a running Tauri app —
    /// keeps working unchanged; `lib.rs` calls this once, in production,
    /// before the first `watch()`.
    pub fn with_event_emitter(mut self, emitter: Arc<dyn AppEventEmitter>) -> Self {
        self.event_emitter = emitter;
        self
    }

    /// Starts recursively watching `root` in the background and returns
    /// once the watch is registered — spawning the background tasks is
    /// fast and non-blocking; this does not wait for the first event.
    ///
    /// # Errors
    /// - [`WatcherError::InvalidPath`] if `root` doesn't exist or isn't a
    ///   directory.
    /// - [`WatcherError::AlreadyWatching`] if `root` is already watched.
    pub async fn watch(&self, root: PathBuf) -> Result<(), WatcherError> {
        if !root.is_dir() {
            return Err(WatcherError::InvalidPath(root));
        }

        // Canonicalize so macOS /private/var vs /var paths are
        // consistent with the paths notify returns in events.
        let canonical = std::fs::canonicalize(&root).unwrap_or(root);

        let mut active = self.active.lock().await;
        if active.contains_key(&canonical) {
            return Err(WatcherError::AlreadyWatching(canonical));
        }

        let handle = self.spawn_watch(canonical.clone());
        active.insert(canonical, handle);
        Ok(())
    }

    /// Stops watching `root` and aborts its background tasks.
    ///
    /// # Errors
    /// [`WatcherError::NotWatching`] if `root` isn't currently watched.
    pub async fn unwatch(&self, root: &Path) -> Result<(), WatcherError> {
        let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let mut active = self.active.lock().await;
        match active.remove(&canonical) {
            Some(handle) => {
                handle.stop();
                Ok(())
            }
            None => Err(WatcherError::NotWatching(root.to_path_buf())),
        }
    }

    /// Lists every directory currently being watched.
    pub async fn watched_paths(&self) -> Vec<PathBuf> {
        self.active.lock().await.keys().cloned().collect()
    }

    /// Spawns the three background tasks that make up one watch's
    /// pipeline (OS watch + reconnect, intake/normalize, debounce-drain
    /// + record) and returns a handle to stop them.
    fn spawn_watch(&self, root: PathBuf) -> WatchHandle {
        let (raw_tx, mut raw_rx) = mpsc::unbounded_channel::<notify::Event>();
        let debouncer = Arc::new(Debouncer::new(DEBOUNCE_WINDOW));

        // Task 1: owns the OS watch, reconnecting on failure, forwarding
        // every raw event onto `raw_tx`.
        let watcher_task = tokio::spawn(run_watch_loop(root.clone(), raw_tx));

        // Task 2: consumes raw events, normalizes them, feeds the debouncer.
        let intake_debouncer = debouncer.clone();
        let intake_task = tokio::spawn(async move {
            while let Some(event) = raw_rx.recv().await {
                for (path, kind) in normalize(&event) {
                    // Canonicalize — on macOS, notify may return /private/var/…
                    // while the watch root was registered as /var/…, which
                    // breaks the starts_with check in workspace detection.
                    let canonical = std::fs::canonicalize(&path).unwrap_or(path);
                    intake_debouncer.push(canonical, kind).await;
                }
            }
        });

        // Task 3: periodically drains debounced events and runs each
        // through workspace detection + timeline recording.
        let workspace_manager = self.workspace_manager.clone();
        let timeline_engine = self.timeline_engine.clone();
        let event_emitter = self.event_emitter.clone();
        let pipeline_root = root.clone();
        let pipeline_debouncer = debouncer;
        let pipeline_task = tokio::spawn(async move {
            let mut ticker = interval(DEBOUNCE_TICK);
            loop {
                ticker.tick().await;
                for event in pipeline_debouncer.drain_ready().await {
                    if let Err(err) = process_event(
                        &workspace_manager,
                        &timeline_engine,
                        event_emitter.as_ref(),
                        &pipeline_root,
                        event,
                    )
                    .await
                    {
                        tracing::error!(error = %err, "failed to process file watcher event");
                    }
                }
            }
        });

        WatchHandle {
            root,
            watcher_task,
            intake_task,
            pipeline_task,
        }
    }
}

/// Resolves the workspace a single debounced event belongs to, records
/// the matching timeline activity, and emits the pipeline's final
/// "EventEmitter → Frontend" stage: `file:changed`, `timeline:event_added`,
/// and `workspace:updated` (the workspace's `last_active_at` changed as
/// part of recording activity against it). A path with no detectable
/// workspace (blueprint §2.2's heuristics found nothing) is silently
/// skipped — see [`WorkspaceManager::resolve_workspace_for_path`] — not
/// treated as an error, and emits nothing.
async fn process_event(
    workspace_manager: &WorkspaceManager,
    timeline_engine: &TimelineEngine,
    event_emitter: &dyn AppEventEmitter,
    watch_root: &Path,
    event: DebouncedEvent,
) -> Result<(), crate::errors::DatabaseError> {
    // Ensure both the event path and watch root are canonicalized on
    // macOS — notify may return /private/var/… while the watch root
    // was registered with /var/…, which breaks starts_with checks.
    let canonical_root = std::fs::canonicalize(watch_root).unwrap_or_else(|_| watch_root.to_path_buf());
    let Some(workspace) = workspace_manager
        .resolve_workspace_for_path(&event.path, &canonical_root)
        .await?
    else {
        return Ok(());
    };

    let path_string = event.path.to_string_lossy().into_owned();
    let activity = match event.kind {
        DebouncedEventKind::Created => TimelineActivity::FileCreated {
            path: path_string.clone(),
        },
        DebouncedEventKind::Modified => TimelineActivity::FileModified {
            path: path_string.clone(),
        },
        DebouncedEventKind::Removed => TimelineActivity::FileDeleted {
            path: path_string.clone(),
        },
    };

    let timeline_event = timeline_engine.record_now(workspace.id, activity).await?;

    app_events::emit(
        event_emitter,
        app_events::EVENT_FILE_CHANGED,
        &serde_json::json!({ "workspaceId": workspace.id, "path": path_string, "kind": format!("{:?}", event.kind) }),
    );
    app_events::emit(
        event_emitter,
        app_events::EVENT_TIMELINE_EVENT_ADDED,
        &timeline_event,
    );
    app_events::emit(
        event_emitter,
        app_events::EVENT_WORKSPACE_UPDATED,
        &workspace,
    );

    Ok(())
}

/// Owns the OS-level watch for one directory, reconnecting automatically
/// if the underlying watch fails (e.g. a watched volume disconnects and
/// later reconnects). Runs until its task is aborted via
/// [`WatchHandle::stop`], or the raw-event receiver is dropped (pipeline
/// shutdown).
async fn run_watch_loop(root: PathBuf, raw_tx: mpsc::UnboundedSender<notify::Event>) {
    loop {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<notify::Result<notify::Event>>();

        let mut watcher = match notify::recommended_watcher(
            move |res: notify::Result<notify::Event>| {
                // `notify`'s callback runs on its own internal thread; this
                // send is the hop back onto the tokio runtime.
                let _ = event_tx.send(res);
            },
        ) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!(error = %err, path = %root.display(), "failed to create file watcher, retrying");
                tokio::time::sleep(RECONNECT_DELAY).await;
                continue;
            }
        };

        if let Err(err) = watcher.watch(&root, RecursiveMode::Recursive) {
            tracing::error!(error = %err, path = %root.display(), "failed to start watching, retrying");
            tokio::time::sleep(RECONNECT_DELAY).await;
            continue;
        }

        tracing::info!(path = %root.display(), "file watcher started");

        let mut watch_failed = false;
        while let Some(result) = event_rx.recv().await {
            match result {
                Ok(event) => {
                    println!("EVENT: {:?}", event);
                    if raw_tx.send(event).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, path = %root.display(), "file watcher reported an error, reconnecting");
                    watch_failed = true;
                    break;
                }
            }
        }

        drop(watcher);
        tracing::info!(path = %root.display(), "file watcher stopped");

        if !watch_failed {
            // The event channel closed without an explicit watch error —
            // the watcher was dropped intentionally (shutdown), not a
            // failure. Don't reconnect.
            return;
        }

        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Handle to one watched root's background tasks.
struct WatchHandle {
    root: PathBuf,
    watcher_task: JoinHandle<()>,
    intake_task: JoinHandle<()>,
    pipeline_task: JoinHandle<()>,
}

impl WatchHandle {
    /// Aborts every background task associated with this watch.
    fn stop(&self) {
        self.watcher_task.abort();
        self.intake_task.abort();
        self.pipeline_task.abort();
        tracing::info!(path = %self.root.display(), "file watcher tasks stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::repositories::{FileRepository, TimelineRepository, WorkspaceRepository};
    use crate::services::{TimelineService, WorkspaceService};
    use crate::timeline::recorder::TimelineRecorder;
    use std::fs;
    use tempfile::tempdir;

    async fn test_watcher() -> (FileWatcher, tempfile::TempDir) {
        let (database, db_guard) = test_database().await;
        let workspace_manager = WorkspaceManager::new(WorkspaceService::new(
            WorkspaceRepository::new(database.pool().clone()),
            TimelineRepository::new(database.pool().clone()),
        ));
        let timeline_engine = TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(
                FileRepository::new(database.pool().clone()),
                TimelineRepository::new(database.pool().clone()),
            ),
            TimelineRepository::new(database.pool().clone()),
        ));

        (
            FileWatcher::new(workspace_manager, timeline_engine),
            db_guard,
        )
    }

    #[tokio::test]
    async fn watch_rejects_a_nonexistent_path() {
        let (watcher, _db_guard) = test_watcher().await;

        let result = watcher
            .watch(PathBuf::from("/definitely/not/a/real/path"))
            .await;
        assert!(matches!(result, Err(WatcherError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn watch_rejects_watching_the_same_path_twice() {
        let (watcher, _db_guard) = test_watcher().await;
        let root = tempdir().unwrap();
        let canonical = std::fs::canonicalize(root.path()).unwrap_or_else(|_| root.path().to_path_buf());

        watcher.watch(root.path().to_path_buf()).await.unwrap();
        let second = watcher.watch(root.path().to_path_buf()).await;

        assert!(matches!(second, Err(WatcherError::AlreadyWatching(_))));

        watcher.unwatch(&canonical).await.unwrap();
    }

    #[tokio::test]
    async fn unwatch_unknown_path_returns_not_watching() {
        let (watcher, _db_guard) = test_watcher().await;
        let root = tempdir().unwrap();

        let result = watcher.unwatch(root.path()).await;
        assert!(matches!(result, Err(WatcherError::NotWatching(_))));
    }

    #[tokio::test]
    async fn watched_paths_reflects_watch_and_unwatch() {
        let (watcher, _db_guard) = test_watcher().await;
        let root = tempdir().unwrap();
        let canonical = std::fs::canonicalize(root.path()).unwrap_or_else(|_| root.path().to_path_buf());

        assert!(watcher.watched_paths().await.is_empty());

        watcher.watch(root.path().to_path_buf()).await.unwrap();
        assert_eq!(
            watcher.watched_paths().await,
            vec![canonical.clone()]
        );

        watcher.unwatch(&canonical).await.unwrap();
        assert!(watcher.watched_paths().await.is_empty());
    }

    /// End-to-end: a real file write under a watched, detectable
    /// workspace root eventually produces a timeline event. Uses a
    /// generous timeout since this exercises the real OS filesystem
    /// notification mechanism, debounce window, and poll tick together.
    #[tokio::test]
    async fn writing_a_file_under_a_detectable_root_produces_a_timeline_event() {
        let (database, _db_guard) = test_database().await;
        let workspace_repository = WorkspaceRepository::new(database.pool().clone());
        let workspace_manager = WorkspaceManager::new(WorkspaceService::new(
            workspace_repository.clone(),
            TimelineRepository::new(database.pool().clone()),
        ));
        let timeline_repository = TimelineRepository::new(database.pool().clone());
        let timeline_engine = TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(
                FileRepository::new(database.pool().clone()),
                timeline_repository.clone(),
            ),
            timeline_repository.clone(),
        ));
        let watcher = FileWatcher::new(workspace_manager, timeline_engine);

        let root = tempdir().unwrap();
        let canonical_root = std::fs::canonicalize(root.path()).unwrap_or_else(|_| root.path().to_path_buf());
        fs::create_dir(canonical_root.join(".git")).unwrap();

        watcher.watch(canonical_root.clone()).await.unwrap();

        // Give the watch loop a moment to actually register with the OS
        // before writing, then write the file that should be detected.
        tokio::time::sleep(Duration::from_millis(200)).await;
        fs::write(canonical_root.join("main.rs"), "fn main() {}").unwrap();

        let root_path_str = canonical_root.to_string_lossy().into_owned();
        // Debounce window (500ms) + tick (100ms) + generous OS-event
        // latency margin.
        let workspace = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(ws) = workspace_repository
                    .find_by_root_path(&root_path_str)
                    .await
                    .unwrap()
                {
                    return ws;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("workspace should be auto-created within the timeout");

        let events = timeline_repository
            .list_by_workspace(workspace.id, None)
            .await
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e.event_type,
                    crate::models::TimelineEventType::Create |
                    crate::models::TimelineEventType::Edit)),
            "expected a timeline event for the written file (write+metadata may coalesce to Edit)"
        );

        watcher.unwatch(&canonical_root).await.unwrap();
    }
}
