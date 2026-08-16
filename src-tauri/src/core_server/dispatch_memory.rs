//! Memory / AI / learning / LLM command dispatch.

use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::core_server::{pget, RpcError, rpc_state, rpc_state_tail};

use crate::commands::ai::AIState;
use crate::ai::models::RerankRequest;
use crate::copilot::memory::models::{MemoryKind, MemoryStatus, RetentionPolicy};
use crate::copilot::memory::MemoryEngine;
use crate::learning::models::SubmitFeedbackRequest;
use crate::learning::{AdaptiveLearningEngine, LearningRepository};
use crate::llm::LLMService;
use crate::llm::LLMSettings;

pub async fn dispatch_memory(
    app: &AppHandle,
    method: &str,
    params: &Value,
) -> Result<Value, RpcError> {
    let result: Value = match method {
        // ------------------------------------------------------------- memory
        "memory_search" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_search, ("query": String, "kind": Option<MemoryKind>, "workspace_id": Option<String>, "status": Option<MemoryStatus>, "limit": Option<usize>)),
        "memory_recommend" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_recommend, ("goal": String, "workspace_id": Option<String>, "limit": Option<usize>)),
        "memory_avoid" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_avoid, ("goal": String, "workspace_id": Option<String>, "limit": Option<usize>)),
        "memory_learned_workflows" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_learned_workflows, ()),
        "memory_stats" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_stats, ()),
        "memory_index_status" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_index_status, ()),
        "memory_reindex" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_reindex, ()),
        "memory_recommendation_feedback" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_recommendation_feedback, ("memory_id": String, "accepted": bool)),
        "memory_learning_health" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_learning_health, ()),
        "memory_failure_patterns" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_failure_patterns, ()),
        "memory_workflow_families" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_workflow_families, ()),
        "memory_aging_summary" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_aging_summary, ()),
        "memory_duplicate_groups" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_duplicate_groups, ()),
        "memory_merge_duplicates" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_merge_duplicates, ()),
        "memory_set_retention" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_set_retention, ("memory_id": String, "policy": RetentionPolicy, "retention_until": Option<String>)),
        "memory_cleanup_now" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_cleanup_now, ()),
        "memory_compress_oversized" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_compress_oversized, ()),
        "memory_restore_compressed" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_restore_compressed, ("memory_id": String)),
        "memory_lineage" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_lineage, ("memory_id": String)),
        "memory_export_json" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_export_json, ()),
        "memory_import_json" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_import_json, ("content": String)),
        "memory_snapshot_create" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_snapshot_create, ("label": Option<String>)),
        "memory_snapshot_list" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_snapshot_list, ()),
        "memory_snapshot_restore" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_snapshot_restore, ("snapshot_id": String)),
        "memory_storage_stats" => rpc_state!(app, params, Arc<MemoryEngine>, crate::commands::memory::memory_storage_stats, ()),

        // ----------------------------------------------------------------- ai
        "list_models" => rpc_state!(app, params, AIState, crate::commands::ai::list_models, ()),
        "get_model" => rpc_state_tail!(app, params, AIState, crate::commands::ai::get_model, ("model_id": String)),
        "download_model" => {
            let model_id: String = pget(params, "model_id")?;
            let r = crate::commands::ai::download_model(model_id, app.state::<AIState>(), app.clone()).await;
            serde_json::to_value(r).map_err(|e| RpcError::message(e.to_string()))?
        }
        "load_model" => rpc_state_tail!(app, params, AIState, crate::commands::ai::load_model, ("model_id": String)),
        "unload_model" => rpc_state_tail!(app, params, AIState, crate::commands::ai::unload_model, ("model_id": String)),
        "get_active_embedding_model" => rpc_state!(app, params, AIState, crate::commands::ai::get_active_embedding_model, ()),
        "get_active_reranker_model" => rpc_state!(app, params, AIState, crate::commands::ai::get_active_reranker_model, ()),
        "get_model_status" => rpc_state_tail!(app, params, AIState, crate::commands::ai::get_model_status, ("model_id": String)),
        "get_inference_statistics" => rpc_state!(app, params, AIState, crate::commands::ai::get_inference_statistics, ()),
        "get_ai_diagnostics" => rpc_state!(app, params, AIState, crate::commands::ai::get_ai_diagnostics, ()),
        "rerank_documents" => rpc_state_tail!(app, params, AIState, crate::commands::ai::rerank_documents, ("request": RerankRequest)),

        // ------------------------------------------------------------ learning
        "submit_feedback" => {
            let request: SubmitFeedbackRequest = pget(params, "request")?;
            let r = crate::commands::learning::submit_feedback(
                app.state::<Arc<AdaptiveLearningEngine>>(),
                app.state::<Arc<MemoryEngine>>(),
                request,
            )
            .await;
            serde_json::to_value(r).map_err(|e| RpcError::message(e.to_string()))?
        }
        "get_learning_insights" => rpc_state!(app, params, Arc<AdaptiveLearningEngine>, crate::commands::learning::get_learning_insights, ()),
        "adjust_prediction_confidence" => rpc_state!(app, params, Arc<AdaptiveLearningEngine>, crate::commands::learning::adjust_prediction_confidence, ("target_type": crate::learning::models::FeedbackTargetType, "target_id": String, "base_confidence": f64)),
        "learn_workflow_patterns" => rpc_state!(app, params, Arc<AdaptiveLearningEngine>, crate::commands::learning::learn_workflow_patterns, ("workflow_type": String, "duration_seconds": i64, "files": Vec<String>, "time_of_day": i32)),
        "get_user_preferences" => rpc_state!(app, params, LearningRepository, crate::commands::learning::get_user_preferences, ()),
        "get_behavioral_patterns" => rpc_state!(app, params, LearningRepository, crate::commands::learning::get_behavioral_patterns, ()),
        "get_confidence_trends" => rpc_state!(app, params, LearningRepository, crate::commands::learning::get_confidence_trends, ("days": i64)),
        "get_learning_stats" => rpc_state!(app, params, LearningRepository, crate::commands::learning::get_learning_stats, ()),

        // ----------------------------------------------------------------- llm
        "llm_get_settings" => rpc_state!(app, params, Arc<LLMService>, crate::commands::llm::llm_get_settings, ()),
        "llm_update_settings" => rpc_state!(app, params, Arc<LLMService>, crate::commands::llm::llm_update_settings, ("settings": LLMSettings)),
        "llm_test_connection" => rpc_state!(app, params, Arc<LLMService>, crate::commands::llm::llm_test_connection, ()),
        "llm_is_configured" => rpc_state!(app, params, Arc<LLMService>, crate::commands::llm::llm_is_configured, ()),
        "llm_get_diagnostics" => rpc_state!(app, params, Arc<LLMService>, crate::commands::llm::llm_get_diagnostics, ()),

        _ => return Err(RpcError::message(format!("unknown method `{method}`"))),
    };
    Ok(result)
}
