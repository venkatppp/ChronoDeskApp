//! Action repository for persisting action history.

use crate::actions::models::{ActionHistory, ActionType, UndoState};
use crate::errors::DatabaseError;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// Repository for action history persistence.
#[derive(Clone)]
pub struct ActionRepository {
    pool: SqlitePool,
}

impl ActionRepository {
    /// Creates a new action repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Records an executed action.
    pub async fn create(
        &self,
        action_type: ActionType,
        workspace_id: Option<i64>,
        recommendation_id: Option<String>,
        success: bool,
        metadata: serde_json::Value,
        undo_state: Option<UndoState>,
    ) -> Result<ActionHistory, DatabaseError> {
        let action_type_str = serde_json::to_string(&action_type)?;
        let metadata_str = serde_json::to_string(&metadata)?;
        let undo_state_str = undo_state.as_ref().map(serde_json::to_string).transpose()?;

        let result = sqlx::query(
            r#"
            INSERT INTO action_history (
                action_type, workspace_id, recommendation_id, success, metadata, undo_state
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&action_type_str)
        .bind(workspace_id)
        .bind(recommendation_id.as_deref())
        .bind(success)
        .bind(&metadata_str)
        .bind(undo_state_str.as_deref())
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();

        Ok(ActionHistory {
            id,
            action_type,
            workspace_id,
            recommendation_id,
            executed_at: Utc::now(),
            success,
            metadata,
            undo_state,
        })
    }

    /// Gets an action by ID.
    pub async fn get_by_id(&self, id: i64) -> Result<Option<ActionHistory>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT id, action_type, workspace_id, recommendation_id, executed_at, 
                   success, metadata, undo_state
            FROM action_history
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let action_type_str: String = row.get("action_type");
                let action_type: ActionType = serde_json::from_str(&action_type_str)?;

                let metadata_str: String = row.get("metadata");
                let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

                let undo_state_str: Option<String> = row.get("undo_state");
                let undo_state: Option<UndoState> = undo_state_str
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?;

                let executed_at_str: String = row.get("executed_at");
                let executed_at = DateTime::parse_from_rfc3339(&executed_at_str)
                    .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc);

                Ok(Some(ActionHistory {
                    id: row.get("id"),
                    action_type,
                    workspace_id: row.get("workspace_id"),
                    recommendation_id: row.get("recommendation_id"),
                    executed_at,
                    success: row.get("success"),
                    metadata,
                    undo_state,
                }))
            }
            None => Ok(None),
        }
    }

    /// Gets action history for a workspace.
    pub async fn get_by_workspace(
        &self,
        workspace_id: i64,
        limit: i64,
    ) -> Result<Vec<ActionHistory>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, workspace_id, recommendation_id, executed_at, 
                   success, metadata, undo_state
            FROM action_history
            WHERE workspace_id = ?
            ORDER BY executed_at DESC
            LIMIT ?
            "#,
        )
        .bind(workspace_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type_str: String = row.get("action_type");
            let action_type: ActionType = serde_json::from_str(&action_type_str)?;

            let metadata_str: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

            let undo_state_str: Option<String> = row.get("undo_state");
            let undo_state: Option<UndoState> = undo_state_str
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?;

            let executed_at_str: String = row.get("executed_at");
            let executed_at = DateTime::parse_from_rfc3339(&executed_at_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            actions.push(ActionHistory {
                id: row.get("id"),
                action_type,
                workspace_id: row.get("workspace_id"),
                recommendation_id: row.get("recommendation_id"),
                executed_at,
                success: row.get("success"),
                metadata,
                undo_state,
            });
        }

        Ok(actions)
    }

    /// Gets all action history.
    pub async fn get_all(&self, limit: i64) -> Result<Vec<ActionHistory>, DatabaseError> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, workspace_id, recommendation_id, executed_at, 
                   success, metadata, undo_state
            FROM action_history
            ORDER BY executed_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type_str: String = row.get("action_type");
            let action_type: ActionType = serde_json::from_str(&action_type_str)?;

            let metadata_str: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;

            let undo_state_str: Option<String> = row.get("undo_state");
            let undo_state: Option<UndoState> = undo_state_str
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?;

            let executed_at_str: String = row.get("executed_at");
            let executed_at = DateTime::parse_from_rfc3339(&executed_at_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            actions.push(ActionHistory {
                id: row.get("id"),
                action_type,
                workspace_id: row.get("workspace_id"),
                recommendation_id: row.get("recommendation_id"),
                executed_at,
                success: row.get("success"),
                metadata,
                undo_state,
            });
        }

        Ok(actions)
    }

    /// Clears all action history.
    pub async fn clear_all(&self) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM action_history")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clears action history for a workspace.
    pub async fn clear_by_workspace(&self, workspace_id: i64) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM action_history WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
