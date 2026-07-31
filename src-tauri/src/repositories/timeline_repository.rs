use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::timeline::TimelineEventRow;
use crate::models::{NewTimelineEvent, TimelineEvent};

/// Default number of rows returned by [`TimelineRepository::list_by_workspace`]
/// when the caller doesn't need the whole history — matches the
/// Timeline screen's initial page size (blueprint §3.2).
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Owns every SQL statement that touches the `timeline_events` table
/// (blueprint §10). There is deliberately no `update` method: the
/// Timeline is an append-only log, and every row is written exactly once
/// by [`TimelineRepository::create`].
#[derive(Debug, Clone)]
pub struct TimelineRepository {
    pool: SqlitePool,
}

const SELECT_COLUMNS: &str =
    "id, workspace_id, file_id, event_type, occurred_at, metadata, created_at";

impl TimelineRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Appends a new event to the log.
    ///
    /// # Errors
    /// [`DatabaseError::Constraint`] if `workspace_id` (or a non-`None`
    /// `file_id`) doesn't reference an existing row.
    pub async fn create(&self, input: NewTimelineEvent) -> Result<TimelineEvent, DatabaseError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let metadata_json = input
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("invalid timeline event metadata: {e}"))
            })?;

        sqlx::query(
            "INSERT INTO timeline_events (id, workspace_id, file_id, event_type, occurred_at, metadata, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(input.workspace_id)
        .bind(input.file_id)
        .bind(input.event_type.as_str())
        .bind(input.occurred_at)
        .bind(&metadata_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            event_id = %id,
            workspace_id = %input.workspace_id,
            event_type = input.event_type.as_str(),
            "timeline event recorded"
        );

        self.get_by_id(id).await
    }

    /// Fetches a single event by id.
    ///
    /// # Errors
    /// [`DatabaseError::NotFound`] if no event with that id exists.
    pub async fn get_by_id(&self, id: Uuid) -> Result<TimelineEvent, DatabaseError> {
        let row: TimelineEventRow = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM timeline_events WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DatabaseError::not_found("timeline_event", id.to_string()))?;

        TimelineEvent::try_from(row)
    }

    /// Lists the most recent events for a workspace, newest first —
    /// backs the Timeline screen's vertical feed (blueprint §3.2). Pass
    /// `None` for `limit` to use the screen's default page size
    /// ([`DEFAULT_LIST_LIMIT`]); pagination beyond the first page is a
    /// Phase 3 concern (offset/cursor param) once the Timeline UI exists
    /// to drive it.
    pub async fn list_by_workspace(
        &self,
        workspace_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<TimelineEvent>, DatabaseError> {
        let rows: Vec<TimelineEventRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM timeline_events
             WHERE workspace_id = ? ORDER BY occurred_at DESC LIMIT ?"
        ))
        .bind(workspace_id)
        .bind(limit.unwrap_or(DEFAULT_LIST_LIMIT))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TimelineEvent::try_from).collect()
    }

    /// Lists every event referencing a specific file, newest first —
    /// e.g. "when was this file opened/edited/moved".
    pub async fn list_by_file(&self, file_id: Uuid) -> Result<Vec<TimelineEvent>, DatabaseError> {
        let rows: Vec<TimelineEventRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM timeline_events
             WHERE file_id = ? ORDER BY occurred_at DESC"
        ))
        .bind(file_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TimelineEvent::try_from).collect()
    }

    /// Lists recent events across all workspaces, newest first.
    ///
    /// Used for Smart Resume to find the most recent session regardless
    /// of workspace. Limited to a reasonable number to avoid scanning
    /// the entire timeline table.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<TimelineEvent>, DatabaseError> {
        let rows: Vec<TimelineEventRow> = sqlx::query_as(&format!(
            "SELECT {SELECT_COLUMNS} FROM timeline_events
             ORDER BY occurred_at DESC LIMIT ?"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(TimelineEvent::try_from).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::{CreateWorkspaceInput, TimelineEventType};
    use crate::repositories::WorkspaceRepository;

    async fn repo_with_workspace() -> (TimelineRepository, Uuid, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let workspace_repo = WorkspaceRepository::new(database.pool().clone());
        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        (
            TimelineRepository::new(database.pool().clone()),
            workspace.id,
            temp_dir,
        )
    }

    #[tokio::test]
    async fn create_and_get_round_trip_with_metadata() {
        let (repo, workspace_id, _guard) = repo_with_workspace().await;

        let created = repo
            .create(NewTimelineEvent {
                workspace_id,
                file_id: None,
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: Some(serde_json::json!({ "diff_lines": 12 })),
            })
            .await
            .expect("create should succeed");

        let fetched = repo.get_by_id(created.id).await.unwrap();
        assert_eq!(fetched.event_type, TimelineEventType::Edit);
        assert_eq!(
            fetched.metadata.unwrap()["diff_lines"],
            serde_json::json!(12)
        );
    }

    #[tokio::test]
    async fn create_rejects_unknown_workspace() {
        let (database, _guard) = test_database().await;
        let repo = TimelineRepository::new(database.pool().clone());

        let result = repo
            .create(NewTimelineEvent {
                workspace_id: Uuid::new_v4(),
                file_id: None,
                event_type: TimelineEventType::Open,
                occurred_at: Utc::now(),
                metadata: None,
            })
            .await;

        assert!(matches!(result, Err(DatabaseError::Constraint(_))));
    }

    #[tokio::test]
    async fn list_by_workspace_orders_newest_first_and_respects_limit() {
        let (repo, workspace_id, _guard) = repo_with_workspace().await;

        for i in 0..5 {
            repo.create(NewTimelineEvent {
                workspace_id,
                file_id: None,
                event_type: TimelineEventType::Open,
                occurred_at: Utc::now() + chrono::Duration::seconds(i),
                metadata: None,
            })
            .await
            .unwrap();
        }

        let events = repo.list_by_workspace(workspace_id, Some(3)).await.unwrap();
        assert_eq!(events.len(), 3);
        // Newest first: each event's occurred_at should be >= the next one's.
        assert!(events[0].occurred_at >= events[1].occurred_at);
        assert!(events[1].occurred_at >= events[2].occurred_at);
    }
}
