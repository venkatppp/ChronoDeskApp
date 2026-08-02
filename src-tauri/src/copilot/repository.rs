//! Copilot Repository - Database layer for conversations and tool executions.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::copilot::models::*;
use crate::errors::DatabaseError;

/// Repository for copilot data persistence.
#[derive(Clone)]
pub struct CopilotRepository {
    pub(crate) pool: SqlitePool,
}

impl CopilotRepository {
    /// Creates a new copilot repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Creates a new conversation.
    pub async fn create_conversation(
        &self,
        workspace_id: Option<Uuid>,
        title: &str,
    ) -> Result<Conversation, DatabaseError> {
        let conversation = Conversation {
            id: Uuid::new_v4(),
            workspace_id,
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 0,
        };

        sqlx::query(
            r#"
            INSERT INTO copilot_conversations (id, workspace_id, title, created_at, updated_at, message_count)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(conversation.id.to_string())
        .bind(conversation.workspace_id.map(|id| id.to_string()))
        .bind(&conversation.title)
        .bind(conversation.created_at.to_rfc3339())
        .bind(conversation.updated_at.to_rfc3339())
        .bind(conversation.message_count)
        .execute(&self.pool)
        .await?;

        Ok(conversation)
    }

    /// Gets a conversation by ID.
    pub async fn get_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<Conversation>, DatabaseError> {
        let row: Option<(String, Option<String>, String, String, String, i32)> = sqlx::query_as(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at, message_count
            FROM copilot_conversations
            WHERE id = ?
            "#,
        )
        .bind(conversation_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, workspace_id, title, created_at, updated_at, message_count)) = row {
            Ok(Some(Conversation {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                workspace_id: workspace_id
                    .map(|id| Uuid::parse_str(&id))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                title,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                message_count,
            }))
        } else {
            Ok(None)
        }
    }

    /// Gets all conversations for a workspace.
    pub async fn get_workspace_conversations(
        &self,
        workspace_id: Uuid,
    ) -> Result<Vec<Conversation>, DatabaseError> {
        let rows: Vec<(String, Option<String>, String, String, String, i32)> = sqlx::query_as(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at, message_count
            FROM copilot_conversations
            WHERE workspace_id = ?
            ORDER BY updated_at DESC
            "#,
        )
        .bind(workspace_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for (id, workspace_id, title, created_at, updated_at, message_count) in rows {
            conversations.push(Conversation {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                workspace_id: workspace_id
                    .map(|id| Uuid::parse_str(&id))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                title,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                message_count,
            });
        }

        Ok(conversations)
    }

    /// Gets recent conversations.
    pub async fn get_recent_conversations(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, DatabaseError> {
        let rows: Vec<(String, Option<String>, String, String, String, i32)> = sqlx::query_as(
            r#"
            SELECT id, workspace_id, title, created_at, updated_at, message_count
            FROM copilot_conversations
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::new();
        for (id, workspace_id, title, created_at, updated_at, message_count) in rows {
            conversations.push(Conversation {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                workspace_id: workspace_id
                    .map(|id| Uuid::parse_str(&id))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                title,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                message_count,
            });
        }

        Ok(conversations)
    }

    /// Adds a message to a conversation.
    pub async fn add_message(&self, message: &Message) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO copilot_messages (id, conversation_id, role, content, tool_calls, reasoning, sources, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.conversation_id.to_string())
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(message.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default()))
        .bind(&message.reasoning)
        .bind(message.sources.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()))
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        // Update conversation updated_at and message_count
        sqlx::query(
            r#"
            UPDATE copilot_conversations
            SET updated_at = ?, message_count = message_count + 1
            WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(message.conversation_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets messages in a conversation.
    pub async fn get_messages(
        &self,
        conversation_id: Uuid,
        limit: Option<usize>,
    ) -> Result<Vec<Message>, DatabaseError> {
        let query = if let Some(limit) = limit {
            format!(
                r#"
                SELECT id, conversation_id, role, content, tool_calls, reasoning, sources, created_at
                FROM copilot_messages
                WHERE conversation_id = ?
                ORDER BY created_at ASC
                LIMIT {}
                "#,
                limit
            )
        } else {
            r#"
            SELECT id, conversation_id, role, content, tool_calls, reasoning, sources, created_at
            FROM copilot_messages
            WHERE conversation_id = ?
            ORDER BY created_at ASC
            "#
            .to_string()
        };

        type MessageRow = (
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        );

        let rows: Vec<MessageRow> = sqlx::query_as(&query)
            .bind(conversation_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for (id, conversation_id, role, content, tool_calls, reasoning, sources, created_at) in rows
        {
            messages.push(Message {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                conversation_id: Uuid::parse_str(&conversation_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                role: MessageRole::parse(&role).ok_or_else(|| {
                    DatabaseError::InvalidInput(format!("Invalid role: {}", role))
                })?,
                content,
                tool_calls: tool_calls.and_then(|tc| serde_json::from_str(&tc).ok()),
                reasoning,
                sources: sources.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
            });
        }

        Ok(messages)
    }

    /// Records a tool execution.
    pub async fn record_tool_execution(
        &self,
        execution: &ToolExecution,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO copilot_tool_executions (
                id, message_id, tool_name, arguments, result, status,
                requires_confirmation, confirmed, error, executed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(execution.id.to_string())
        .bind(execution.message_id.to_string())
        .bind(&execution.tool_name)
        .bind(serde_json::to_string(&execution.arguments).unwrap_or_default())
        .bind(
            execution
                .result
                .as_ref()
                .map(|r| serde_json::to_string(r).unwrap_or_default()),
        )
        .bind(execution.status.as_str())
        .bind(execution.requires_confirmation)
        .bind(execution.confirmed)
        .bind(&execution.error)
        .bind(execution.executed_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates a tool execution status.
    pub async fn update_tool_execution_status(
        &self,
        execution_id: Uuid,
        status: ToolExecutionStatus,
        result: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE copilot_tool_executions
            SET status = ?, result = ?, error = ?
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(result.map(|r| serde_json::to_string(&r).unwrap_or_default()))
        .bind(error)
        .bind(execution_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Saves a context snapshot.
    pub async fn save_context_snapshot(
        &self,
        snapshot: &ContextSnapshot,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO copilot_context_snapshots (
                id, conversation_id, workspace_id, active_files, recent_events,
                session_summary, captured_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(snapshot.id.to_string())
        .bind(snapshot.conversation_id.to_string())
        .bind(snapshot.workspace_id.map(|id| id.to_string()))
        .bind(serde_json::to_string(&snapshot.active_files).unwrap_or_default())
        .bind(serde_json::to_string(&snapshot.recent_events).unwrap_or_default())
        .bind(&snapshot.session_summary)
        .bind(snapshot.captured_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Creates a plan.
    pub async fn create_plan(&self, plan: &Plan) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            INSERT INTO copilot_plans (
                id, message_id, goal, steps, current_step, status, created_at, completed_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(plan.id.to_string())
        .bind(plan.message_id.to_string())
        .bind(&plan.goal)
        .bind(serde_json::to_string(&plan.steps).unwrap_or_default())
        .bind(plan.current_step as i64)
        .bind(plan.status.as_str())
        .bind(plan.created_at.to_rfc3339())
        .bind(plan.completed_at.map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Updates a plan.
    pub async fn update_plan(&self, plan: &Plan) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE copilot_plans
            SET steps = ?, current_step = ?, status = ?, completed_at = ?
            WHERE id = ?
            "#,
        )
        .bind(serde_json::to_string(&plan.steps).unwrap_or_default())
        .bind(plan.current_step as i64)
        .bind(plan.status.as_str())
        .bind(plan.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(plan.id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets a plan by message ID.
    pub async fn get_plan_by_message(
        &self,
        message_id: Uuid,
    ) -> Result<Option<Plan>, DatabaseError> {
        type PlanRow = (
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            Option<String>,
        );

        let row: Option<PlanRow> = sqlx::query_as(
            r#"
            SELECT id, message_id, goal, steps, current_step, status, created_at, completed_at
            FROM copilot_plans
            WHERE message_id = ?
            "#,
        )
        .bind(message_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some((id, message_id, goal, steps, current_step, status, created_at, completed_at)) =
            row
        {
            Ok(Some(Plan {
                id: Uuid::parse_str(&id).map_err(|e| DatabaseError::IoError(e.to_string()))?,
                message_id: Uuid::parse_str(&message_id)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?,
                goal,
                steps: serde_json::from_str(&steps).unwrap_or_default(),
                current_step: current_step as usize,
                status: PlanStatus::parse(&status).ok_or_else(|| {
                    DatabaseError::InvalidInput(format!("Invalid plan status: {}", status))
                })?,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .with_timezone(&Utc),
                completed_at: completed_at
                    .map(|dt| chrono::DateTime::parse_from_rfc3339(&dt))
                    .transpose()
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?
                    .map(|dt| dt.with_timezone(&Utc)),
            }))
        } else {
            Ok(None)
        }
    }

    /// Renames a conversation.
    pub async fn rename_conversation(
        &self,
        conversation_id: Uuid,
        new_title: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE copilot_conversations
            SET title = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_title)
        .bind(Utc::now().to_rfc3339())
        .bind(conversation_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a conversation and all its messages.
    pub async fn delete_conversation(&self, conversation_id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            DELETE FROM copilot_conversations
            WHERE id = ?
            "#,
        )
        .bind(conversation_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Pins or unpins a conversation.
    pub async fn pin_conversation(
        &self,
        conversation_id: Uuid,
        pinned: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            UPDATE copilot_conversations
            SET pinned = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(pinned)
        .bind(Utc::now().to_rfc3339())
        .bind(conversation_id.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets conversation messages (alias for compatibility).
    pub async fn get_conversation_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>, DatabaseError> {
        self.get_messages(conversation_id, None).await
    }
}
