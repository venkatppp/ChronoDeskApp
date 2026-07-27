# ChronoDesk Event Pipeline

The full path a single filesystem change takes, from the OS to the
screen, with no polling anywhere in the chain.

```
                         +-------------------------+
                         |   OS filesystem event    |
                         | (inotify/FSEvents/etc.)  |
                         +------------+-------------+
                                      |
                                      v
                    notify::RecommendedWatcher callback
                    (runs on notify's own internal thread)
                                      |
                        watcher::watcher::run_watch_loop
                        forwards raw notify::Event via
                        an unbounded mpsc channel
                                      |
                                      v
                 watcher::event_handler::normalize()
                 - drops paths under .git/, node_modules/,
                   target/, dist/, build/, .next/,
                   __pycache__/, .venv/
                 - drops OS metadata files (.DS_Store,
                   Thumbs.db, desktop.ini), dotfiles, and
                   editor temp files (~, .tmp, .swp, ~$...)
                 - collapses a same-event rename
                   (ModifyKind::Name(RenameMode::Both))
                   into a Removed+Created pair
                                      |
                                      v
                   watcher::debounce::Debouncer::push()
                 - coalesces rapid repeated events per path
                   within a 500ms window
                 - Created immediately followed by Removed
                   within the window cancels out entirely
                 - periodic 100ms tick calls drain_ready()
                   to flush anything whose window elapsed
                                      |
                                      v
              workspace::manager::WorkspaceManager
                    ::resolve_workspace_for_path()
                 - workspace::detector::detect_workspace_root()
                   walks from the file's parent directory up
                   to (and including) the watched root,
                   checking workspace::heuristics::detect_markers()
                   at each level (.git, Cargo.toml, package.json,
                   pom.xml, build.gradle[.kts], pyproject.toml/
                   setup.py, README*) until confidence_score()
                   clears DETECTION_THRESHOLD (0.8), or gives up
                 - WorkspaceRepository::find_by_root_path() --
                   existing workspace found? touch_last_active()
                   (+ reactivate if archived) via
                   WorkspaceService::open_workspace()
                 - otherwise: WorkspaceService::create_workspace()
                   with the detected root_path, then open_workspace()
                 - no ancestor qualifies? return None -- the event
                   is dropped, not force-fit into a workspace
                                      |
                                      v
                timeline::engine::TimelineEngine::record_now()
                    --> services::TimelineService::record_activity_now()
                    --> timeline::recorder::TimelineRecorder::record()
                 - maps the debounced event kind to a
                   TimelineActivity (FileCreated/Modified/Deleted)
                 - resolves (or creates) the `files` row for that
                   path within the workspace via
                   FileRepository::find_by_workspace_and_path()
                 - TimelineActivity::to_event_type_and_metadata()
                   maps onto the storage-level TimelineEventType
                   + a structured JSON metadata payload
                                      |
                                      v
              TimelineRepository::create() / FileRepository::create()
                              (sqlx, one INSERT each)
                                      |
                                      v
                     SQLite (WAL mode, foreign keys on)
                                      |
                                      v
                  app_events::emit() (best-effort, logged not
                  propagated on failure) via the AppEventEmitter
                  trait -- tauri::AppHandle in production
                 - file:changed          { workspaceId, path, kind }
                 - timeline:event_added  <TimelineEvent JSON>
                 - workspace:updated     <Workspace JSON>
                                      |
                                      v
              @tauri-apps/api/event listen() (frontend)
              hooks/useAppEvents.ts subscribes
              features/dashboard/hooks/useDashboardData.ts
              re-runs its fetch on any DASHBOARD_REFRESH_EVENTS
                                      |
                                      v
                    React re-renders: WorkspaceCard,
                    RecentActivityFeed, BriefingBanner,
                    RecommendationsPanel -- no manual refresh
```

## The command-driven path (not filesystem-triggered)

Creating/updating/deleting a workspace from the UI (the "+ New
workspace" button) takes a shorter path that still ends at the same
place:

```
React (DashboardView) --> invoke("create_workspace", ...)
   --> commands::workspace::create_workspace(app: AppHandle, ...)
   --> services::WorkspaceService::create_workspace()
   --> WorkspaceRepository::create() + TimelineRepository::create()
        (the creation-time timeline event)
   --> SQLite
   --> app_events::emit(&app, EVENT_WORKSPACE_CREATED, &workspace)
   --> frontend listen() --> useDashboardData refetches
```

The `#[tauri::command]` wrapper itself does nothing but this: pull the
managed `WorkspaceService` out of state, call one method, emit one event.
Everything above is identical whether the trigger was a file write on
disk or a button click -- both funnel through the same
`WorkspaceService`/`TimelineService` methods, which is exactly the point
of the layering described in `ARCHITECTURE.md`.

## Debounce window and tick cadence

| Constant | Value | Why |
|---|---|---|
| `DEBOUNCE_WINDOW` | 500ms | Long enough to coalesce an editor's "write temp file, then rename over original" save pattern into one event; short enough that the dashboard's activity feed still feels immediate. |
| `DEBOUNCE_TICK` | 100ms | Independent of the window -- just needs to be small enough that a flushed event doesn't lag noticeably behind its window elapsing. |
| `RECONNECT_DELAY` | 2s | Backoff before retrying a failed OS watch (e.g. a disconnected network volume), to avoid a tight retry loop against something that isn't coming back immediately. |
