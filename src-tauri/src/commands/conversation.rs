//! Conversation Management IPC Commands - Rename, delete, export, pin conversations

use tauri::State;
use uuid::Uuid;

use crate::copilot::CopilotRepository;

/// Renames a conversation.
#[tauri::command]
pub async fn copilot_rename_conversation(
    repository: State<'_, CopilotRepository>,
    conversation_id: String,
    new_title: String,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    repository
        .rename_conversation(cid, &new_title)
        .await
        .map_err(|e| e.to_string())
}

/// Deletes a conversation and all its messages.
#[tauri::command]
pub async fn copilot_delete_conversation(
    repository: State<'_, CopilotRepository>,
    conversation_id: String,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    repository
        .delete_conversation(cid)
        .await
        .map_err(|e| e.to_string())
}

/// Pins/unpins a conversation.
#[tauri::command]
pub async fn copilot_pin_conversation(
    repository: State<'_, CopilotRepository>,
    conversation_id: String,
    pinned: bool,
) -> Result<(), String> {
    let cid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;
    repository
        .pin_conversation(cid, pinned)
        .await
        .map_err(|e| e.to_string())
}

/// Exports a conversation to JSON.
#[tauri::command]
pub async fn copilot_export_conversation_json(
    repository: State<'_, CopilotRepository>,
    conversation_id: String,
) -> Result<String, String> {
    let cid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;

    let conversation = repository
        .get_conversation(cid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Conversation not found".to_string())?;

    let messages = repository
        .get_conversation_messages(cid)
        .await
        .map_err(|e| e.to_string())?;

    let export = serde_json::json!({
        "conversation": conversation,
        "messages": messages,
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

/// Exports a conversation to Markdown.
#[tauri::command]
pub async fn copilot_export_conversation_markdown(
    repository: State<'_, CopilotRepository>,
    conversation_id: String,
) -> Result<String, String> {
    let cid = Uuid::parse_str(&conversation_id).map_err(|e| e.to_string())?;

    let conversation = repository
        .get_conversation(cid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Conversation not found".to_string())?;

    let messages = repository
        .get_conversation_messages(cid)
        .await
        .map_err(|e| e.to_string())?;

    let mut markdown = String::new();
    markdown.push_str(&format!("# {}\n\n", conversation.title));
    markdown.push_str(&format!(
        "**Created:** {}\n\n",
        conversation.created_at.format("%Y-%m-%d %H:%M:%S")
    ));
    markdown.push_str("---\n\n");

    for message in messages {
        let role_label = match message.role {
            crate::copilot::MessageRole::User => "👤 User",
            crate::copilot::MessageRole::Assistant => "🤖 Assistant",
            crate::copilot::MessageRole::System => "⚙️ System",
        };

        markdown.push_str(&format!("## {}\n\n", role_label));
        markdown.push_str(&format!("{}\n\n", message.content));

        if let Some(reasoning) = message.reasoning {
            markdown.push_str(&format!("*Reasoning:* {}\n\n", reasoning));
        }

        markdown.push_str(&format!(
            "*{:?} at {}*\n\n",
            message.role,
            message.created_at.format("%Y-%m-%d %H:%M:%S")
        ));
    }

    Ok(markdown)
}
