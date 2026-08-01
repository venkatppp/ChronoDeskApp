//! Analytics Repository
//!
//! Efficient SQL aggregations for analytics data. This repository provides
//! the raw aggregated data that AnalyticsService transforms into insights.
//!
//! All queries are optimized to use indexes and avoid loading unnecessary
//! rows into memory.

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;

/// Raw aggregated counts from database queries.
#[derive(Debug, sqlx::FromRow)]
pub struct EventCounts {
    pub total_events: i64,
    pub edit_count: i64,
    pub commit_count: i64,
}

/// Workspace activity for a time range.
#[derive(Debug, sqlx::FromRow)]
pub struct WorkspaceActivity {
    pub workspace_id: Uuid,
    pub event_count: i64,
}

/// File edit frequency.
#[derive(Debug, sqlx::FromRow)]
pub struct FileEditCount {
    pub file_id: Uuid,
    pub edit_count: i64,
}

/// Analytics Repository: efficient SQL aggregations.
#[derive(Debug, Clone)]
pub struct AnalyticsRepository {
    pool: SqlitePool,
}

impl AnalyticsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Counts timeline events within a time range.
    ///
    /// Returns total events, edit count, and commit count.
    pub async fn count_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<EventCounts, DatabaseError> {
        let row = sqlx::query_as::<_, EventCounts>(
            "SELECT 
                COUNT(*) as total_events,
                COUNT(CASE WHEN event_type IN ('edit', 'create') THEN 1 END) as edit_count,
                COUNT(CASE WHEN event_type = 'commit' THEN 1 END) as commit_count
             FROM timeline_events
             WHERE occurred_at >= ? AND occurred_at < ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Counts distinct files edited within a time range.
    pub async fn count_distinct_files(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT file_id)
             FROM timeline_events
             WHERE occurred_at >= ? AND occurred_at < ?
               AND file_id IS NOT NULL",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Counts distinct workspaces active within a time range.
    pub async fn count_distinct_workspaces(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT workspace_id)
             FROM timeline_events
             WHERE occurred_at >= ? AND occurred_at < ?",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Gets workspace activity ranked by event count.
    ///
    /// Returns workspaces ordered by most active first.
    pub async fn get_workspace_activity(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WorkspaceActivity>, DatabaseError> {
        let rows = sqlx::query_as::<_, WorkspaceActivity>(
            "SELECT workspace_id, COUNT(*) as event_count
             FROM timeline_events
             WHERE occurred_at >= ? AND occurred_at < ?
             GROUP BY workspace_id
             ORDER BY event_count DESC
             LIMIT ?",
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Gets most edited files within a time range for a workspace.
    pub async fn get_most_edited_files(
        &self,
        workspace_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<FileEditCount>, DatabaseError> {
        let rows = sqlx::query_as::<_, FileEditCount>(
            "SELECT file_id, COUNT(*) as edit_count
             FROM timeline_events
             WHERE workspace_id = ?
               AND occurred_at >= ? AND occurred_at < ?
               AND event_type IN ('edit', 'create')
               AND file_id IS NOT NULL
             GROUP BY file_id
             ORDER BY edit_count DESC
             LIMIT ?",
        )
        .bind(workspace_id)
        .bind(start)
        .bind(end)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Counts events for a specific workspace within a time range.
    pub async fn count_workspace_events(
        &self,
        workspace_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<EventCounts, DatabaseError> {
        let row = sqlx::query_as::<_, EventCounts>(
            "SELECT 
                COUNT(*) as total_events,
                COUNT(CASE WHEN event_type IN ('edit', 'create') THEN 1 END) as edit_count,
                COUNT(CASE WHEN event_type = 'commit' THEN 1 END) as commit_count
             FROM timeline_events
             WHERE workspace_id = ?
               AND occurred_at >= ? AND occurred_at < ?",
        )
        .bind(workspace_id)
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Gets the most recent timeline event for a workspace.
    pub async fn get_last_activity(
        &self,
        workspace_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DatabaseError> {
        let row: Option<(DateTime<Utc>,)> = sqlx::query_as(
            "SELECT occurred_at
             FROM timeline_events
             WHERE workspace_id = ?
             ORDER BY occurred_at DESC
             LIMIT 1",
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }

    /// Gets timeline events for a specific workspace within a time range.
    ///
    /// Used for computing session counts and durations via ContextService.
    pub async fn get_workspace_event_ids(
        &self,
        workspace_id: Uuid,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Uuid>, DatabaseError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT id
             FROM timeline_events
             WHERE workspace_id = ?
               AND occurred_at >= ? AND occurred_at < ?
             ORDER BY occurred_at ASC",
        )
        .bind(workspace_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;
    use crate::models::{CreateWorkspaceInput, NewTimelineEvent, TimelineEventType};
    use crate::repositories::{TimelineRepository, WorkspaceRepository};
    use chrono::Duration;

    async fn setup() -> (AnalyticsRepository, Uuid, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let pool = database.pool().clone();

        let workspace_repo = WorkspaceRepository::new(pool.clone());
        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test Workspace".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        (AnalyticsRepository::new(pool), workspace.id, temp_dir)
    }

    #[tokio::test]
    async fn count_events_returns_zero_for_empty_range() {
        let (repo, _workspace_id, _guard) = setup().await;

        let now = Utc::now();
        let counts = repo
            .count_events(now, now + Duration::hours(1))
            .await
            .unwrap();

        assert_eq!(counts.total_events, 0);
        assert_eq!(counts.edit_count, 0);
        assert_eq!(counts.commit_count, 0);
    }

    #[tokio::test]
    async fn count_events_aggregates_correctly() {
        let (_repo, _workspace_id, _guard) = setup().await;

        let (database, _temp_dir) = test_database().await;
        let timeline_repo = TimelineRepository::new(database.pool().clone());
        let workspace_repo = WorkspaceRepository::new(database.pool().clone());

        let workspace = workspace_repo
            .create(CreateWorkspaceInput {
                name: "Test".to_string(),
                description: None,
                root_path: None,
            })
            .await
            .unwrap();

        let now = Utc::now();

        // Create events
        timeline_repo
            .create(NewTimelineEvent {
                workspace_id: workspace.id,
                file_id: None,
                event_type: TimelineEventType::Edit,
                occurred_at: now,
                metadata: None,
            })
            .await
            .unwrap();

        timeline_repo
            .create(NewTimelineEvent {
                workspace_id: workspace.id,
                file_id: None,
                event_type: TimelineEventType::Commit,
                occurred_at: now + Duration::minutes(10),
                metadata: None,
            })
            .await
            .unwrap();

        let analytics_repo = AnalyticsRepository::new(database.pool().clone());
        let counts = analytics_repo
            .count_events(now - Duration::minutes(5), now + Duration::hours(1))
            .await
            .unwrap();

        assert_eq!(counts.total_events, 2);
        assert_eq!(counts.edit_count, 1);
        assert_eq!(counts.commit_count, 1);
    }
}
