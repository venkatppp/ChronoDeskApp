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
        "metadata": {
            "format": "chronodesk.copilot.export.v1",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "message_count": messages.len(),
        },
        "conversation": conversation,
        "messages": messages,
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

    let mut markdown = export_markdown_header(&conversation, messages.len());

    for message in messages {
        let role_label = match message.role {
            crate::copilot::MessageRole::User => "User",
            crate::copilot::MessageRole::Assistant => "Assistant",
            crate::copilot::MessageRole::System => "System",
        };

        markdown.push_str(&format!("## {}\n\n", role_label));
        markdown.push_str(&format!(
            "- **Message ID:** `{}`\n- **Timestamp:** {}\n- **Role:** `{}`\n\n",
            message.id,
            message.created_at.to_rfc3339(),
            message.role.as_str()
        ));
        markdown.push_str(&format!("{}\n\n", message.content));

        if let Some(reasoning) = message.reasoning {
            markdown.push_str("### Reasoning\n\n");
            markdown.push_str(&format!("{}\n\n", reasoning));
        }

        if let Some(tool_calls) = message.tool_calls {
            markdown.push_str("### Tool Calls\n\n");
            for tool_call in tool_calls {
                markdown.push_str(&format!(
                    "- `{}` (`{}`)\n  - Arguments: `{}`\n  - Result: `{}`\n",
                    tool_call.tool_name,
                    tool_call.status.as_str(),
                    tool_call.arguments,
                    tool_call
                        .result
                        .map(|result| result.to_string())
                        .unwrap_or_else(|| "none".to_string())
                ));
            }
            markdown.push('\n');
        }

        if let Some(sources) = message.sources {
            markdown.push_str("### Sources\n\n");
            for source in sources {
                markdown.push_str(&format!(
                    "- **{}** (`{:?}`): {} ({:.0}% relevance)\n",
                    source.title,
                    source.source_type,
                    source.reference,
                    source.relevance * 100.0
                ));
            }
            markdown.push('\n');
        }

        markdown.push_str("### Attachments\n\nNone recorded.\n\n");
        markdown.push_str("---\n\n");
    }

    Ok(markdown)
}

fn export_markdown_header(
    conversation: &crate::copilot::Conversation,
    message_count: usize,
) -> String {
    format!(
        "# {}\n\n## Metadata\n\n- **Conversation ID:** `{}`\n- **Workspace ID:** {}\n- **Created:** {}\n- **Updated:** {}\n- **Messages:** {}\n- **Exported:** {}\n- **Format:** `chronodesk.copilot.markdown.v1`\n\n---\n\n",
        conversation.title,
        conversation.id,
        conversation
            .workspace_id
            .map(|id| format!("`{}`", id))
            .unwrap_or_else(|| "None".to_string()),
        conversation.created_at.to_rfc3339(),
        conversation.updated_at.to_rfc3339(),
        message_count,
        chrono::Utc::now().to_rfc3339()
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn markdown_export_header_contains_metadata() {
        let conversation = crate::copilot::Conversation {
            id: Uuid::new_v4(),
            workspace_id: None,
            title: "Export Test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 2,
        };

        let markdown = super::export_markdown_header(&conversation, 2);

        assert!(markdown.contains("# Export Test"));
        assert!(markdown.contains("Conversation ID"));
        assert!(markdown.contains("chronodesk.copilot.markdown.v1"));
    }
}
