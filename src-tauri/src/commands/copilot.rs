//! Copilot IPC Commands - Natural language workspace assistant.

use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::copilot::engine::CopilotEngine;
use crate::copilot::models::*;
use crate::copilot::streaming::StreamingDiagnostics;
use crate::copilot::tools::{
    ToolDefinition, ToolDiagnostics, ToolExecutor, ToolPermissionDecision, ToolPermissionPolicy,
    ToolPermissionService,
};

/// Sends a message to the copilot.
#[tauri::command]
pub async fn copilot_send_message(
    engine: State<'_, Arc<CopilotEngine>>,
    request: SendMessageRequest,
) -> Result<CopilotResponse, String> {
    engine
        .send_message(request)
        .await
        .map_err(|e| e.to_string())
}

/// Starts a streaming copilot message response.
#[tauri::command]
pub async fn copilot_send_message_stream(
    engine: State<'_, Arc<CopilotEngine>>,
    request: SendMessageRequest,
) -> Result<CopilotStreamResponse, String> {
    engine
        .inner()
        .clone()
        .send_message_stream(request)
        .await
        .map_err(|e| e.to_string())
}

/// Cancels an active streaming copilot response.
#[tauri::command]
pub async fn copilot_cancel_stream(
    engine: State<'_, Arc<CopilotEngine>>,
    stream_id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&stream_id).map_err(|e| e.to_string())?;
    engine.cancel_stream(uuid).await.map_err(|e| e.to_string())
}

/// Gets current copilot streaming diagnostics.
#[tauri::command]
pub async fn copilot_get_streaming_diagnostics(
    engine: State<'_, Arc<CopilotEngine>>,
) -> Result<StreamingDiagnostics, String> {
    Ok(engine.streaming_diagnostics().await)
}

/// Gets conversation history.
#[tauri::command]
pub async fn copilot_get_conversation(
    engine: State<'_, Arc<CopilotEngine>>,
    conversation_id: String,
) -> Result<Vec<Message>, String> {
    let uuid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    engine
        .get_conversation_history(uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Gets recent conversations.
#[tauri::command]
pub async fn copilot_get_recent_conversations(
    engine: State<'_, Arc<CopilotEngine>>,
    limit: usize,
) -> Result<Vec<Conversation>, String> {
    engine
        .get_recent_conversations(limit)
        .await
        .map_err(|e| e.to_string())
}

/// Searches persisted conversations using backend filters.
#[tauri::command]
pub async fn copilot_search_conversations(
    engine: State<'_, Arc<CopilotEngine>>,
    request: ConversationSearchRequest,
) -> Result<Vec<ConversationSearchResult>, String> {
    engine
        .search_conversations(request)
        .await
        .map_err(|e| e.to_string())
}

/// Gets a daily briefing.
#[tauri::command]
pub async fn copilot_get_daily_briefing(
    engine: State<'_, Arc<CopilotEngine>>,
    request: DailyBriefingRequest,
) -> Result<DailyBriefing, String> {
    engine
        .get_daily_briefing(request.workspace_id)
        .await
        .map_err(|e| e.to_string())
}

/// Gets available tools.
#[tauri::command]
pub async fn copilot_get_tools() -> Result<Vec<serde_json::Value>, String> {
    let tools = ToolExecutor::get_available_tools();
    Ok(tools
        .into_iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters.into_iter().map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "param_type": p.param_type,
                        "description": p.description,
                        "required": p.required
                    })
                }).collect::<Vec<_>>(),
                "requires_confirmation": t.requires_confirmation
            })
        })
        .collect())
}

/// Discovers available copilot tools from the runtime registry.
#[tauri::command]
pub async fn copilot_discover_tools(
    tool_executor: State<'_, Arc<ToolExecutor>>,
) -> Result<Vec<ToolDefinition>, String> {
    Ok(tool_executor.available_tools())
}

/// Gets current tool invocation diagnostics.
#[tauri::command]
pub async fn copilot_get_tool_diagnostics(
    tool_executor: State<'_, Arc<ToolExecutor>>,
) -> Result<ToolDiagnostics, String> {
    Ok(tool_executor.diagnostics())
}

/// Asks a question about workspace history.
#[tauri::command]
pub async fn copilot_ask_question(
    engine: State<'_, Arc<CopilotEngine>>,
    request: WorkspaceQuestionRequest,
) -> Result<WorkspaceAnswer, String> {
    // For now, use the send_message flow
    let send_request = SendMessageRequest {
        conversation_id: None,
        workspace_id: request.workspace_id,
        message: request.question,
        include_context: true,
    };

    let response = engine
        .send_message(send_request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(WorkspaceAnswer {
        answer: response.message.content,
        reasoning: response.message.reasoning.unwrap_or_default(),
        sources: response.message.sources.unwrap_or_default(),
        confidence: 0.8,
    })
}

/// Lists every persisted tool permission policy.
#[tauri::command]
pub async fn copilot_list_tool_permissions(
    permissions: State<'_, Arc<ToolPermissionService>>,
) -> Result<Vec<ToolPermissionPolicy>, String> {
    Ok(permissions.policies().await)
}

/// Sets (upserts) a persisted permission policy for a tool.
#[tauri::command]
pub async fn copilot_set_tool_permission(
    permissions: State<'_, Arc<ToolPermissionService>>,
    tool_name: String,
    workspace_id: Option<String>,
    decision: ToolPermissionDecision,
) -> Result<(), String> {
    if tool_name.trim().is_empty() {
        return Err("tool_name must not be empty".to_string());
    }
    let wid = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;

    permissions
        .set_policy(&tool_name, wid, decision)
        .await
        .map_err(|e| e.to_string())
}

/// Removes a persisted permission policy for a tool + scope.
#[tauri::command]
pub async fn copilot_clear_tool_permission(
    permissions: State<'_, Arc<ToolPermissionService>>,
    tool_name: String,
    workspace_id: Option<String>,
) -> Result<(), String> {
    let wid = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;

    permissions
        .clear_policy(&tool_name, wid)
        .await
        .map_err(|e| e.to_string())
}

/// Resolves the effective (workspace-or-global) decision for a tool.
#[tauri::command]
pub async fn copilot_check_tool_permission(
    permissions: State<'_, Arc<ToolPermissionService>>,
    tool_name: String,
    workspace_id: Option<String>,
) -> Result<Option<ToolPermissionDecision>, String> {
    let wid = workspace_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| e.to_string())?;

    Ok(permissions.resolve(&tool_name, wid).await)
}
