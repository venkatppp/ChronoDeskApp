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
//! | `search`    | Hybrid keyword + vector search engine   | Phase 4 ✅ |
//! | `graph`     | Knowledge Graph Engine                  | Phase 4 ✅ |
//! | `session`   | Session Intelligence & Context Scoring  | Phase 5  |
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

pub mod actions;
pub mod ai;
pub mod analytics;
pub mod app_events;
pub mod commands;
pub mod context_memory;
pub mod copilot;
pub mod database;
pub mod duplicates;
pub mod errors;
pub mod graph;
pub mod hashing;
pub mod intelligence;
pub mod learning;
pub mod llm;
pub mod ml;
pub mod models;
pub mod predictive;
pub mod repositories;
pub mod runtime;
pub mod search;
pub mod semantic;
pub mod services;
pub mod session;
pub mod timeline;
pub mod watcher;
pub mod workspace;

use std::sync::Arc;

use tauri::Manager;

use actions::{ActionEngine, ActionRepository, ActionService};
use analytics::AnalyticsEngine;
use context_memory::{ContextMemoryEngine, ContextMemoryRepository};
use duplicates::DuplicateDetectionEngine;
use graph::GraphEngine;
use intelligence::health::{HealthService, WorkspaceHealthEngine};
use intelligence::recommendation::RecommendationEngine;
use predictive::{
    AdaptiveLearning, AutomationEngine, PredictiveEngine, PredictiveRepository, WorkflowEngine,
};
use repositories::{
    FileRepository, GraphRepository, LLMRepository, MLRepository, SearchRepository,
    SettingsRepository, TimelineRepository, WorkspaceRepository,
};
use search::SearchEngine;
use services::{
    ContextService, GraphService, MLService, SearchService, TimelineService, WorkspaceService,
};
use session::SessionEngine;
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
            let ml_repository = MLRepository::new(pool.clone());
            let secret_store = Arc::new(llm::KeyringSecretStore::new());
            let llm_repository = Arc::new(LLMRepository::new(pool.clone(), secret_store));

            // --- Services (business logic composing repositories) ---
            let workspace_service =
                WorkspaceService::new(workspace_repository.clone(), timeline_repository.clone());
            let timeline_recorder =
                TimelineRecorder::new(file_repository.clone(), timeline_repository.clone());
            let timeline_service =
                TimelineService::new(timeline_recorder, timeline_repository.clone());
            let search_service = SearchService::new(search_repository.clone());
            let graph_service = GraphService::new(graph_repository.clone());
            let ml_service = MLService::new(ml_repository.clone(), file_repository.clone());

            // --- Engines (the public facades commands and the watcher pipeline hold) ---
            let workspace_manager = WorkspaceManager::new(workspace_service.clone());
            let timeline_engine = TimelineEngine::new(timeline_service.clone());
            let search_engine = SearchEngine::new(search_service.clone());
            let graph_engine = GraphEngine::new(graph_service.clone());

            // --- Session Engine & Context Service (Phase 5A) ---
            let session_engine =
                SessionEngine::new(timeline_repository.clone(), file_repository.clone());
            let context_service = ContextService::new(
                session_engine.clone(),
                workspace_repository.clone(),
                settings_repository.clone(),
            );

            // --- Analytics Engine & Service (Phase 5B) ---
            let analytics_repository =
                analytics::repository::AnalyticsRepository::new(pool.clone());
            let analytics_service = analytics::service::AnalyticsService::new(
                analytics_repository,
                context_service.clone(),
                workspace_repository.clone(),
                file_repository.clone(),
            );
            let analytics_engine = AnalyticsEngine::new(analytics_service);

            // --- Intelligence Layer (Phase 5C) ---
            let health_service = HealthService::new(pool.clone());
            let health_engine = WorkspaceHealthEngine::new(
                health_service,
                workspace_repository.clone(),
                timeline_repository.clone(),
                file_repository.clone(),
                context_service.clone(),
            );
            let recommendation_engine = RecommendationEngine::new(
                workspace_repository.clone(),
                file_repository.clone(),
                context_service.clone(),
            );

            // --- Action Engine & Service (Phase 5D) ---
            let action_repository = ActionRepository::new(pool.clone());
            let action_engine = ActionEngine::new(
                action_repository.clone(),
                workspace_repository.clone(),
                file_repository.clone(),
            );
            let action_service = ActionService::new(action_repository.clone(), action_engine);

            // --- Context Memory Engine (Phase 5E) ---
            let context_memory_repository = ContextMemoryRepository::new(pool.clone());
            let context_memory_engine = ContextMemoryEngine::new(
                context_memory_repository,
                workspace_repository.clone(),
                context_service.clone(),
            );

            // --- Duplicate Detection Engine (Phase 5 Stage 2) ---
            let duplicate_engine = DuplicateDetectionEngine::new(file_repository.clone())
                .with_event_emitter(
                    Arc::new(app_handle.clone()) as Arc<dyn app_events::AppEventEmitter>
                );

            // --- Predictive Intelligence & Workflow Automation (Phase 5F) ---
            let predictive_repository = PredictiveRepository::new(pool.clone());

            let predictive_engine = PredictiveEngine::new(
                workspace_repository.clone(),
                timeline_repository.clone(),
                context_service.clone(),
                analytics_engine.clone(),
                context_memory_engine.clone(),
            );

            let workflow_engine = WorkflowEngine::new(
                timeline_repository.clone(),
                file_repository.clone(),
                context_service.clone(),
            );

            let adaptive_learning = AdaptiveLearning::new(
                predictive_repository.clone(),
                workspace_repository.clone(),
                timeline_repository.clone(),
                context_service.clone(),
            );

            let automation_engine = AutomationEngine::new(
                predictive_repository.clone(),
                workspace_repository.clone(),
                file_repository.clone(),
                context_memory_engine.clone(),
                recommendation_engine.clone(),
            );

            // --- Real-Time Intelligence Runtime (Phase 5G) ---
            let emitter = runtime::IntelligenceEmitter::new(
                Arc::new(app_handle.clone()) as Arc<dyn app_events::AppEventEmitter>
            );
            let cache = runtime::IntelligenceCache::new();

            // --- Runtime Health & Diagnostics (Phase 5H) ---
            let health_service = runtime::RuntimeHealthService::new(cache.clone());
            let diagnostics_service = runtime::DiagnosticsService::new(health_service.clone());
            let recovery_service = runtime::RecoveryService::new(pool.clone());

            // Initialize recovery system
            tauri::async_runtime::block_on(recovery_service.initialize())?;

            // Check if recovery is needed
            if tauri::async_runtime::block_on(recovery_service.needs_recovery())? {
                tracing::warn!("Detected interrupted shutdown, performing recovery");
                let recovered_jobs = tauri::async_runtime::block_on(recovery_service.recover())?;
                tracing::info!("Recovered {} interrupted jobs", recovered_jobs.len());
            }

            // Record clean startup
            tauri::async_runtime::block_on(recovery_service.checkpoint(
                runtime::RecoveryState::Clean,
                vec![],
                serde_json::Value::Null,
            ))?;

            let runtime_workers = Arc::new(runtime::RuntimeWorkers::new(
                emitter.clone(),
                cache.clone(),
                predictive_engine.clone(),
                workflow_engine.clone(),
                health_engine.clone(),
                recommendation_engine.clone(),
                context_memory_engine.clone(),
            ));

            // Start background workers
            runtime_workers.clone().start();

            // --- AI & Model Management (Phase 6B) ---
            let models_dir = app_handle
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir")
                .join("models");

            let ai_settings = ai::AISettings::with_models_dir(models_dir);
            let model_manager = ai::ModelManager::new(ai_settings);

            // Initialize with LocalEmbeddingProvider as fallback
            let embedding_provider: Arc<dyn semantic::EmbeddingProvider> =
                Arc::new(semantic::LocalEmbeddingProvider::default());

            let ai_state = commands::ai::AIState {
                manager: model_manager,
                reranker: None,
                embedding_provider: None,
            };

            // --- Semantic Intelligence Layer (Phase 6A) ---
            let semantic_repository = semantic::SemanticRepository::new(pool.clone());
            tauri::async_runtime::block_on(semantic_repository.initialize())?;

            let semantic_engine = semantic::SemanticMemoryEngine::new(
                semantic_repository.clone(),
                embedding_provider,
            );
            let semantic_search = semantic::SemanticSearchEngine::new(
                semantic_engine.clone(),
                semantic_repository.clone(),
            );
            let reasoning_engine = semantic::ContextReasoningEngine::new(
                semantic_engine.clone(),
                semantic_search.clone(),
                predictive_engine.clone(),
                recommendation_engine.clone(),
                context_memory_engine.clone(),
            );

            // --- Adaptive Learning Engine (Phase 6C) ---
            let learning_repository = learning::LearningRepository::new(pool.clone());
            let learning_engine = Arc::new(learning::AdaptiveLearningEngine::new(Arc::new(
                learning_repository.clone(),
            )));

            // Start learning workers
            let learning_worker = learning::LearningWorker::new(learning_engine.clone(), 3600);
            let preference_worker =
                learning::PreferenceLearningWorker::new(learning_engine.clone(), 1800);
            let calibration_worker =
                learning::ConfidenceCalibrationWorker::new(learning_engine.clone(), 7200);

            tauri::async_runtime::spawn(learning_worker.start());
            tauri::async_runtime::spawn(preference_worker.start());
            tauri::async_runtime::spawn(calibration_worker.start());

            // --- Copilot Engine (Phase 7A) ---
            let copilot_repository = copilot::CopilotRepository::new(pool.clone());

            // Initialize LLM service
            let llm_service = Arc::new(llm::LLMService::new(llm_repository.clone()));
            tauri::async_runtime::block_on(llm_service.initialize())?;

            let tool_executor = Arc::new(copilot::ToolExecutor::new(
                Arc::new(workspace_service.clone()),
                Arc::new(session_engine.clone()),
                Arc::new(timeline_engine.clone()),
            ));
            let conversation_manager = Arc::new(copilot::ConversationManager::new(
                Arc::new(copilot_repository.clone()),
                Arc::new(context_memory_engine.clone()),
                Arc::new(session_engine.clone()),
                Arc::new(timeline_engine.clone()),
            ));
            let streaming_manager = Arc::new(copilot::StreamingSessionManager::new(Arc::new(
                app_handle.clone(),
            )));
            let copilot_engine = Arc::new(copilot::CopilotEngine::new(
                conversation_manager,
                tool_executor.clone(),
                Arc::new(copilot_repository),
                llm_service.clone(),
                streaming_manager,
                Arc::new(reasoning_engine.clone()),
                Arc::new(predictive_engine.clone()),
                learning_engine.clone(),
                Arc::new(recommendation_engine.clone()),
                Arc::new(context_memory_engine.clone()),
                Arc::new(session_engine.clone()),
                Arc::new(timeline_engine.clone()),
            ));

            // --- Proactive AI Engine (Phase 7B) ---
            let proactive_engine = Arc::new(copilot::ProactiveEngine::new(
                Arc::new(timeline_engine.clone()),
                Arc::new(session_engine.clone()),
                Arc::new(predictive_engine.clone()),
                learning_engine.clone(),
                Arc::new(recommendation_engine.clone()),
                Arc::new(context_memory_engine.clone()),
                Arc::new(reasoning_engine.clone()),
            ));

            // --- Execution Engine (RC-2) ---
            let execution_repository = Arc::new(copilot::ExecutionRepository::new(pool.clone()));
            let execution_engine = Arc::new(copilot::ExecutionEngine::new(
                execution_repository,
                tool_executor.clone(),
            ));

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
            app.manage(ml_repository);
            app.manage(workspace_service);
            app.manage(timeline_service);
            app.manage(search_service);
            app.manage(graph_service);
            app.manage(ml_service);
            app.manage(context_service);
            app.manage(analytics_engine);
            app.manage(health_engine);
            app.manage(recommendation_engine);
            app.manage(action_service);
            app.manage(context_memory_engine.clone());
            app.manage(predictive_engine);
            app.manage(workflow_engine);
            app.manage(adaptive_learning);
            app.manage(automation_engine);
            app.manage(timeline_engine);
            app.manage(search_engine);
            app.manage(graph_engine);
            app.manage(duplicate_engine);
            app.manage(file_watcher);
            app.manage(emitter);
            app.manage(cache);
            app.manage(runtime_workers);
            app.manage(health_service);
            app.manage(diagnostics_service);
            app.manage(recovery_service);
            app.manage(semantic_engine);
            app.manage(semantic_search);
            app.manage(reasoning_engine);
            app.manage(learning_repository);
            app.manage(learning_engine);
            app.manage(ai_state);
            app.manage(llm_repository);
            app.manage(llm_service);
            app.manage(tool_executor);
            app.manage(copilot_engine);
            app.manage(proactive_engine);
            app.manage(execution_engine);

            tracing::info!("ChronoDesk backend ready");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system::get_app_version,
            commands::system::health_check,
            commands::system::open_file,
            commands::workspace::list_active_workspaces,
            commands::workspace::list_archived_workspaces,
            commands::workspace::get_workspace,
            commands::workspace::get_workspace_statistics,
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
            commands::duplicates::scan_workspace_for_duplicates,
            commands::duplicates::scan_file,
            commands::duplicates::get_duplicate_groups,
            commands::duplicates::find_duplicates,
            commands::duplicates::get_scan_progress,
            commands::duplicates::cancel_scan,
            commands::session::get_smart_resume_session,
            commands::session::get_workspace_sessions,
            commands::session::get_latest_workspace_session,
            commands::session::set_session_inactivity_threshold,
            commands::session::get_session_inactivity_threshold,
            commands::analytics::get_daily_briefing,
            commands::analytics::get_today_summary,
            commands::analytics::get_yesterday_summary,
            commands::analytics::get_this_week_summary,
            commands::analytics::get_last_week_summary,
            commands::analytics::get_this_month_summary,
            commands::analytics::get_workspace_insight,
            commands::intelligence::get_workspace_health,
            commands::intelligence::get_latest_workspace_health,
            commands::intelligence::get_workspace_health_history,
            commands::intelligence::get_workspace_recommendations,
            commands::intelligence::get_category_recommendations,
            commands::intelligence::get_priority_recommendations,
            commands::actions::execute_action,
            commands::actions::undo_action,
            commands::actions::get_action_history,
            commands::actions::get_all_action_history,
            commands::actions::clear_action_history,
            commands::actions::clear_workspace_action_history,
            commands::context_memory::create_context_snapshot,
            commands::context_memory::get_workspace_snapshots,
            commands::context_memory::get_latest_snapshot,
            commands::context_memory::detect_workspace_relationships,
            commands::context_memory::get_related_workspaces,
            commands::context_memory::search_knowledge,
            commands::context_memory::snapshot_milestone,
            commands::predictive::get_predictions_summary,
            commands::predictive::get_current_workflow,
            commands::predictive::get_learning_profile,
            commands::predictive::update_learning_profile,
            commands::predictive::create_automation_rule,
            commands::predictive::list_automation_rules,
            commands::predictive::update_automation_rule_enabled,
            commands::predictive::delete_automation_rule,
            commands::runtime::get_runtime_health,
            commands::runtime::get_runtime_diagnostics,
            commands::runtime::get_runtime_summary,
            commands::semantic::semantic_search,
            commands::semantic::find_similar_documents,
            commands::semantic::infer_related_work,
            commands::semantic::detect_recurring_workflows,
            commands::semantic::find_similar_sessions,
            commands::semantic::explain_recommendation,
            commands::semantic::infer_missing_context,
            commands::ai::list_models,
            commands::ai::get_model,
            commands::ai::download_model,
            commands::ai::load_model,
            commands::ai::unload_model,
            commands::ai::get_active_embedding_model,
            commands::ai::get_active_reranker_model,
            commands::ai::get_model_status,
            commands::ai::get_inference_statistics,
            commands::ai::get_ai_diagnostics,
            commands::ai::rerank_documents,
            commands::learning::submit_feedback,
            commands::learning::get_learning_insights,
            commands::learning::adjust_prediction_confidence,
            commands::learning::learn_workflow_patterns,
            commands::learning::get_user_preferences,
            commands::learning::get_behavioral_patterns,
            commands::learning::get_confidence_trends,
            commands::learning::get_learning_stats,
            commands::copilot::copilot_send_message,
            commands::copilot::copilot_send_message_stream,
            commands::copilot::copilot_cancel_stream,
            commands::copilot::copilot_get_streaming_diagnostics,
            commands::copilot::copilot_get_conversation,
            commands::copilot::copilot_get_recent_conversations,
            commands::copilot::copilot_search_conversations,
            commands::copilot::copilot_get_daily_briefing,
            commands::copilot::copilot_get_tools,
            commands::copilot::copilot_discover_tools,
            commands::copilot::copilot_get_tool_diagnostics,
            commands::copilot::copilot_ask_question,
            commands::proactive::copilot_get_notifications,
            commands::proactive::copilot_dismiss_notification,
            commands::proactive::copilot_get_resume_context,
            commands::proactive::copilot_generate_plan,
            commands::proactive::copilot_set_permission,
            commands::proactive::copilot_check_permission,
            commands::proactive::copilot_get_enhanced_briefing,
            commands::proactive::copilot_query_timeline,
            commands::proactive::copilot_check_opportunities,
            commands::llm::llm_get_settings,
            commands::llm::llm_update_settings,
            commands::llm::llm_test_connection,
            commands::llm::llm_is_configured,
            commands::llm::llm_get_diagnostics,
            commands::execution::execution_start,
            commands::execution::execution_pause,
            commands::execution::execution_resume,
            commands::execution::execution_cancel,
            commands::execution::execution_get_progress,
            commands::conversation::copilot_rename_conversation,
            commands::conversation::copilot_delete_conversation,
            commands::conversation::copilot_pin_conversation,
            commands::conversation::copilot_export_conversation_json,
            commands::conversation::copilot_export_conversation_markdown,
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
