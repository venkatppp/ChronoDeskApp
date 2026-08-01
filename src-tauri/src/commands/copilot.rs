//! Copilot IPC Commands - Natural language workspace assistant.

use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use crate::copilot::engine::CopilotEngine;
use crate::copilot::models::*;
use crate::copilot::tools::ToolExecutor;

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
