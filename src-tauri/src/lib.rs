//! ChronoDesk backend library.
//!
//! This crate is split by domain, mirroring the engines described in the
//! product blueprint (§4 Software Architecture).
//!
//! | Module      | Owns                                   | Ships in |
//! |-------------|-----------------------------------------|----------|
//! | `commands`  | Tauri IPC command handlers              | Phase 1  |
//! | `database`  | SQLite connection pool & migrations     | Phase 2 ✅ |
//! | `watcher`   | OS file-system event watcher            | Phase 3 ✅ |
//! | `workspace` | Workspace Engine (lifecycle, detection) | Phase 3 ✅ |
//! | `timeline`  | Timeline Engine (append-only event log) | Phase 3 ✅ |
//! | `search`    | Hybrid keyword + vector search engine   | Phase 4  |
//! | `graph`     | Knowledge Graph Engine                  | Phase 4  |
//! | `ml`        | ONNX Runtime inference layer            | Phase 5  |
//!
//! Plus three supporting layers cutting across the table above:
//! `errors` (shared error types), `models` (typed domain structs + DTOs),
//! `repositories` (all SQL, one module per aggregate), and `services`
//! (business logic composing repositories) — `commands` depends on
//! `services`/engines, engines depend on `services`, `services` depend on
//! `repositories`, `repositories` depend on `database` and `models`. A
//! strict one-way chain, enforced by convention. `app_events` is the one
//! deliberate exception: it's reached from both `commands` (thin,
//! Tauri-aware) and `watcher` (a background engine), since both are the
//! places a user-visible change actually happens and needs to reach the
//! frontend.
//!
//! ## Phase 3 event pipeline
//!
//! ```text
//! notify::Event
//!     │
//!     ▼
//! watcher::event_handler::normalize()   (drop ignored paths, collapse rename→remove+create)
//!     │
//!     ▼
//! watcher::debounce::Debouncer          (coalesce rapid same-path events)
//!     │
//!     ▼
//! workspace::WorkspaceManager           (find-or-create the workspace this path belongs to)
//!     │
//!     ▼
//! timeline::TimelineEngine              (record the activity, auto-creating the files row)
//!     │
//!     ▼
//! repositories::* → database::Database  (SQLite, WAL mode)
//!     │
//!     ▼
//! app_events::AppEventEmitter           (workspace:updated, file:changed, timeline:event_added)
//!     │
//!     ▼
//! frontend (@tauri-apps/api/event listen())
//! ```

pub mod app_events;
pub mod commands;
pub mod database;
pub mod errors;
pub mod graph;
pub mod ml;
pub mod models;
pub mod repositories;
pub mod search;
pub mod services;
pub mod timeline;
pub mod watcher;
pub mod workspace;

use std::sync::Arc;

use tauri::Manager;

use graph::GraphEngine;
use repositories::{
    FileRepository, GraphRepository, SearchRepository, SettingsRepository, TimelineRepository,
    WorkspaceRepository,
};
use search::SearchEngine;
use services::{GraphService, SearchService, TimelineService, WorkspaceService};
use timeline::recorder::TimelineRecorder;
use timeline::TimelineEngine;
use watcher::FileWatcher;
use workspace::WorkspaceManager;

/// Builds and runs the Tauri application. Called from `main.rs`; kept in
/// the library crate (rather than inline in `main`) so it can also be
/// exercised from integration tests and, eventually, mobile targets.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().level(log_level()).build())
        .setup(|app| {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .try_init();

            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                "ChronoDesk backend starting"
            );

            // Database initialization is async (sqlx) but `setup` is sync,
            // and the app must not accept commands before the schema is
            // ready — so this blocks startup on it rather than spawning
            // it in the background. For a local SQLite file this is a
            // few milliseconds; not worth deferring command availability
            // for.
            let app_handle = app.handle().clone();
            let database =
                tauri::async_runtime::block_on(database::Database::initialize(&app_handle))?;
            let pool = database.pool().clone();

            // --- Repositories (data access, one per aggregate) ---
            let workspace_repository = WorkspaceRepository::new(pool.clone());
            let file_repository = FileRepository::new(pool.clone());
            let timeline_repository = TimelineRepository::new(pool.clone());
            let settings_repository = SettingsRepository::new(pool.clone());
            let search_repository = SearchRepository::new(pool.clone());
            let graph_repository = GraphRepository::new(pool.clone());

            // --- Services (business logic composing repositories) ---
            let workspace_service =
                WorkspaceService::new(workspace_repository.clone(), timeline_repository.clone());
            let timeline_recorder =
                TimelineRecorder::new(file_repository.clone(), timeline_repository.clone());
            let timeline_service =
                TimelineService::new(timeline_recorder, timeline_repository.clone());
            let search_service = SearchService::new(search_repository.clone());
            let graph_service = GraphService::new(graph_repository.clone());

            // --- Engines (the public facades commands and the watcher pipeline hold) ---
            let workspace_manager = WorkspaceManager::new(workspace_service.clone());
            let timeline_engine = TimelineEngine::new(timeline_service.clone());
            let search_engine = SearchEngine::new(search_service.clone());
            let graph_engine = GraphEngine::new(graph_service.clone());

            // --- File Watcher, wired to a real AppEventEmitter (the AppHandle) ---
            let file_watcher = FileWatcher::new(workspace_manager, timeline_engine.clone())
                .with_event_emitter(
                    Arc::new(app_handle.clone()) as Arc<dyn app_events::AppEventEmitter>
                );

            // Restore watch paths persisted from a previous launch before
            // the window is shown, so watching resumes exactly where the
            // user left off with no manual re-add step.
            tauri::async_runtime::block_on(commands::watcher::restore_watched_paths(
                &file_watcher,
                &settings_repository,
            ))?;

            // Every service/engine/repository above is managed as Tauri
            // state so command handlers can pull the one they need via
            // `tauri::State<'_, T>`. `Database` itself is managed too,
            // for any future code that needs the raw pool directly.
            app.manage(database);
            app.manage(workspace_repository);
            app.manage(file_repository);
            app.manage(timeline_repository);
            app.manage(settings_repository);
            app.manage(search_repository);
            app.manage(graph_repository);
            app.manage(workspace_service);
            app.manage(timeline_service);
            app.manage(search_service);
            app.manage(graph_service);
            app.manage(timeline_engine);
            app.manage(search_engine);
            app.manage(graph_engine);
            app.manage(file_watcher);

            tracing::info!("ChronoDesk backend ready");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_version,
            commands::system::health_check,
            commands::workspace::list_active_workspaces,
            commands::workspace::list_archived_workspaces,
            commands::workspace::get_workspace,
            commands::workspace::create_workspace,
            commands::workspace::update_workspace,
            commands::workspace::delete_workspace,
            commands::workspace::switch_workspace,
            commands::timeline::list_workspace_timeline,
            commands::timeline::get_recent_activity,
            commands::watcher::add_watch_path,
            commands::watcher::remove_watch_path,
            commands::watcher::list_watch_paths,
            commands::search::search,
            commands::search::get_search_history,
            commands::search::save_search_query,
            commands::search::clear_search_history,
            commands::search::save_search,
            commands::search::list_saved_searches,
            commands::search::delete_saved_search,
            commands::search::get_recent_files,
            commands::search::get_workspace_stats,
            commands::graph::get_graph,
            commands::graph::get_node_details,
            commands::graph::get_graph_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ChronoDesk");
}

fn log_level() -> log::LevelFilter {
    if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    }
}
