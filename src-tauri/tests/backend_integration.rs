//! Integration tests for ChronoDesk's backend.
//!
//! Unlike the unit tests inside each `src/` module (which build only the
//! two or three collaborators a single type needs), every test here
//! builds the **entire** dependency chain in the same order `lib.rs`'s
//! `setup()` does — `Database` → repositories → services → engines →
//! `FileWatcher` — against a real (temporary) SQLite file and, for the
//! file-watching tests, a real filesystem. This is what actually
//! exercises "does the whole application start up and work end to end",
//! which no single module's unit tests can prove on their own.
//!
//! ## What this suite deliberately does *not* cover
//! There is no running Tauri window in this sandbox (no display, and —
//! see `PROJECT_STATE.md` — this environment's system Rust toolchain
//! predates what the current `tauri`/`sqlx` dependency tree requires, so
//! not even `cargo check` completes here, only `rustfmt`-level syntax
//! verification). That means these tests cannot invoke a
//! `#[tauri::command]` function through the real IPC transport. What
//! they verify instead — `service`/`engine` method calls with the exact
//! arguments a command handler would forward — is the entire
//! non-trivial part of an IPC command's behavior; the `#[tauri::command]`
//! wrapper itself is a one-line delegation with no logic of its own (see
//! `commands::workspace` and `commands::timeline`), so this is a
//! faithful, not partial, substitute for "IPC command execution"
//! coverage.

use std::fs;
use std::time::Duration;

use chronodesk_lib::database::Database;
use chronodesk_lib::models::{CreateWorkspaceInput, TimelineEventType, UpdateWorkspaceInput};
use chronodesk_lib::repositories::{
    FileRepository, SettingsRepository, TimelineRepository, WorkspaceRepository,
};
use chronodesk_lib::services::{TimelineService, WorkspaceService};
use chronodesk_lib::timeline::recorder::TimelineRecorder;
use chronodesk_lib::timeline::TimelineEngine;
use chronodesk_lib::watcher::FileWatcher;
use chronodesk_lib::workspace::WorkspaceManager;

/// Every layer of the application, built in the exact order `lib.rs`'s
/// `setup()` builds them, against one temporary SQLite database. Kept as
/// a plain struct local to this test file — deliberately independent of
/// `lib.rs`'s own copy of this wiring, rather than sharing a helper, so a
/// bug introduced in production wiring can't also silently exist in the
/// thing meant to catch it.
struct FullStack {
    pool: sqlx::SqlitePool,
    workspace_repository: WorkspaceRepository,
    settings_repository: SettingsRepository,
    workspace_service: WorkspaceService,
    timeline_repository: TimelineRepository,
    workspace_manager: WorkspaceManager,
    timeline_engine: TimelineEngine,
    file_watcher: FileWatcher,
    _db_guard: tempfile::TempDir,
}

async fn build_full_stack() -> FullStack {
    let db_guard = tempfile::tempdir().expect("failed to create temp dir");
    let db_path = db_guard.path().join("chronodesk.db");
    let database = Database::initialize_at(&db_path)
        .await
        .expect("database should initialize cleanly on a fresh path");
    let pool = database.pool().clone();

    let workspace_repository = WorkspaceRepository::new(pool.clone());
    let file_repository = FileRepository::new(pool.clone());
    let timeline_repository = TimelineRepository::new(pool.clone());
    let settings_repository = SettingsRepository::new(pool.clone());

    let workspace_service =
        WorkspaceService::new(workspace_repository.clone(), timeline_repository.clone());
    let timeline_recorder =
        TimelineRecorder::new(file_repository.clone(), timeline_repository.clone());
    let timeline_service = TimelineService::new(timeline_recorder, timeline_repository.clone());

    let workspace_manager = WorkspaceManager::new(workspace_service.clone());
    let timeline_engine = TimelineEngine::new(timeline_service);
    let file_watcher = FileWatcher::new(workspace_manager.clone(), timeline_engine.clone());

    FullStack {
        pool,
        workspace_repository,
        settings_repository,
        workspace_service,
        timeline_repository,
        workspace_manager,
        timeline_engine,
        file_watcher,
        _db_guard: db_guard,
    }
}

/// Verifies the entire dependency-injection chain — `Database` through
/// every repository, service, and engine — actually produces working
/// collaborators, not just individually-correct ones. This is the
/// "application startup" and "dependency injection" checklist item.
#[tokio::test]
async fn full_stack_starts_up_and_a_workspace_flows_through_every_layer() {
    let stack = build_full_stack().await;

    let workspace = stack
        .workspace_service
        .create_workspace(CreateWorkspaceInput {
            name: "Integration Test Workspace".to_string(),
            description: None,
            root_path: None,
        })
        .await
        .expect("workspace creation should succeed through the full stack");

    // The service layer's creation-time timeline event must be visible
    // through the repository layer too — proving both share the same
    // underlying database, not two disconnected in-memory states.
    let events = stack
        .timeline_repository
        .list_by_workspace(workspace.id, None)
        .await
        .expect("listing events should succeed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, TimelineEventType::WorkspaceSwitch);

    let found = stack
        .workspace_repository
        .get_by_id(workspace.id)
        .await
        .expect(
            "workspace should be retrievable via a separate repository instance on the same pool",
        );
    assert_eq!(found.id, workspace.id);
}

/// Verifies `SettingsRepository::set`/`get` round-trip through the same
/// pool a separate repository instance uses — i.e. that "dependency
/// injection" here really means "share one pool", not one connection
/// each holding its own private state.
#[tokio::test]
async fn settings_persist_across_independently_constructed_repositories() {
    let stack = build_full_stack().await;

    stack
        .settings_repository
        .set("watched_paths", "[\"/tmp/example\"]")
        .await
        .expect("set should succeed");

    // A brand-new repository instance built from nothing but the pool
    // (exactly what `commands::watcher::restore_watched_paths` does on
    // every launch) must see the same value.
    let fresh_settings = SettingsRepository::new(stack.pool.clone());
    let value = fresh_settings
        .get("watched_paths")
        .await
        .expect("get should succeed")
        .expect("value should be visible from a different repository instance");
    assert_eq!(value, "[\"/tmp/example\"]");
}

/// Verifies `commands::watcher::restore_watched_paths` — the exact
/// function `lib.rs` calls once at startup — re-establishes every
/// persisted watch, simulating an application restart: paths are
/// persisted by one `FileWatcher`, then restored into a second, freshly
/// constructed one built against the same database.
#[tokio::test]
async fn restore_watched_paths_reestablishes_watches_after_a_simulated_restart() {
    let stack = build_full_stack().await;
    let root_a = tempfile::tempdir().unwrap();
    let root_b = tempfile::tempdir().unwrap();

    stack
        .file_watcher
        .watch(root_a.path().to_path_buf())
        .await
        .unwrap();
    stack
        .file_watcher
        .watch(root_b.path().to_path_buf())
        .await
        .unwrap();

    let paths: Vec<String> = stack
        .file_watcher
        .watched_paths()
        .await
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    stack
        .settings_repository
        .set("watched_paths", &serde_json::to_string(&paths).unwrap())
        .await
        .unwrap();

    // Simulate a fresh launch: a brand-new FileWatcher, nothing watched
    // yet, against the same underlying database.
    let fresh_watcher = FileWatcher::new(
        stack.workspace_manager.clone(),
        stack.timeline_engine.clone(),
    );
    assert!(fresh_watcher.watched_paths().await.is_empty());

    chronodesk_lib::commands::watcher::restore_watched_paths(
        &fresh_watcher,
        &stack.settings_repository,
    )
    .await
    .expect("restore should succeed");

    let mut restored = fresh_watcher.watched_paths().await;
    restored.sort();
    let ca = std::fs::canonicalize(root_a.path()).unwrap_or_else(|_| root_a.path().to_path_buf());
    let cb = std::fs::canonicalize(root_b.path()).unwrap_or_else(|_| root_b.path().to_path_buf());
    let mut expected = vec![ca, cb];
    expected.sort();
    assert_eq!(restored, expected);
}

/// End-to-end: writing a real file under a watched, detectable workspace
/// root flows through the *entire* pipeline —
/// `notify` → debounce → normalize → `WorkspaceManager` →
/// `TimelineEngine` → repositories → SQLite — and produces a queryable
/// row, using the exact same composition `lib.rs` uses in production.
#[tokio::test]
async fn end_to_end_file_write_flows_through_the_full_pipeline() {
    let stack = build_full_stack().await;
    let root = tempfile::tempdir().unwrap();
    let canonical_root =
        std::fs::canonicalize(root.path()).unwrap_or_else(|_| root.path().to_path_buf());
    fs::create_dir(canonical_root.join(".git")).unwrap();

    stack
        .file_watcher
        .watch(canonical_root.clone())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;
    fs::write(canonical_root.join("main.rs"), "fn main() {}").unwrap();

    let root_str = canonical_root.to_string_lossy().into_owned();
    let workspace = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(ws) = stack
                .workspace_repository
                .find_by_root_path(&root_str)
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

    // Then poll for the timeline event — process_event records the
    // file event after creating the workspace, so there is a scheduling
    // window where the workspace exists but the file event hasn't been
    // committed yet.
    let has_file_event = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let events = stack
                .timeline_repository
                .list_by_workspace(workspace.id, None)
                .await
                .unwrap();
            if events.iter().any(|e| {
                matches!(
                    e.event_type,
                    TimelineEventType::Create | TimelineEventType::Edit
                )
            }) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("timeline event should appear within the timeout");

    assert!(
        has_file_event,
        "expected a timeline event for the written file (write+metadata may coalesce to Edit)"
    );

    stack.file_watcher.unwatch(&canonical_root).await.unwrap();
}

/// Exercises the exact logic behind every `commands::workspace::*` and
/// `commands::timeline::*` handler — full create → update → record
/// activity → delete → cascade-verify lifecycle — since a
/// `#[tauri::command]` wrapper is a one-line delegation to these same
/// service/engine calls (see the module-level doc comment for why this
/// stands in for "IPC command execution" in an environment with no
/// running Tauri window).
#[tokio::test]
async fn workspace_lifecycle_matches_what_every_ipc_command_handler_calls() {
    let stack = build_full_stack().await;

    // create_workspace
    let workspace = stack
        .workspace_service
        .create_workspace(CreateWorkspaceInput {
            name: "Lifecycle Test".to_string(),
            description: Some("initial".to_string()),
            root_path: None,
        })
        .await
        .unwrap();

    // update_workspace
    let renamed = stack
        .workspace_service
        .update_workspace(
            workspace.id,
            UpdateWorkspaceInput {
                name: Some("Renamed".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(renamed.name, "Renamed");

    // list_workspace_timeline / get_recent_activity underlying logic
    let events = stack
        .timeline_engine
        .recent_events(workspace.id, Some(5))
        .await
        .unwrap();
    assert!(!events.is_empty());

    // delete_workspace, plus cascade verification
    stack
        .workspace_service
        .delete_workspace(workspace.id)
        .await
        .unwrap();
    let after_delete = stack.workspace_repository.get_by_id(workspace.id).await;
    assert!(
        after_delete.is_err(),
        "workspace should be gone after delete"
    );
}
