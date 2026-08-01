//! Automatic Recovery System
//!
//! Handles recovery from crashes, interrupted indexing, and unexpected shutdowns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::errors::DatabaseError;

/// Recovery state for runtime components.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    /// Runtime started cleanly.
    Clean,

    /// Runtime shutdown cleanly.
    Shutdown,

    /// Runtime crashed or was interrupted.
    Interrupted,
}

/// Recovery checkpoint for tracking runtime state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCheckpoint {
    pub state: RecoveryState,
    pub last_heartbeat: DateTime<Utc>,
    pub active_jobs: Vec<String>,
    pub metadata: serde_json::Value,
}

/// Manages recovery checkpoints and automatic recovery.
#[derive(Clone)]
pub struct RecoveryService {
    pool: SqlitePool,
}

impl RecoveryService {
    /// Creates a new recovery service.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initializes recovery table if it doesn't exist.
    pub async fn initialize(&self) -> Result<(), DatabaseError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_recovery (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                state TEXT NOT NULL,
                last_heartbeat TEXT NOT NULL,
                active_jobs TEXT NOT NULL,
                metadata TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Records a recovery checkpoint.
    pub async fn checkpoint(
        &self,
        state: RecoveryState,
        active_jobs: Vec<String>,
        metadata: serde_json::Value,
    ) -> Result<(), DatabaseError> {
        let state_str = serde_json::to_string(&state)?;
        let active_jobs_json = serde_json::to_string(&active_jobs)?;
        let metadata_json = serde_json::to_string(&metadata)?;
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO runtime_recovery (id, state, last_heartbeat, active_jobs, metadata)
            VALUES (1, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                state = excluded.state,
                last_heartbeat = excluded.last_heartbeat,
                active_jobs = excluded.active_jobs,
                metadata = excluded.metadata
            "#,
        )
        .bind(state_str)
        .bind(now)
        .bind(active_jobs_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets the last recovery checkpoint.
    pub async fn get_last_checkpoint(&self) -> Result<Option<RecoveryCheckpoint>, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT state, last_heartbeat, active_jobs, metadata
            FROM runtime_recovery
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let state_str: String = row.get("state");
            let state: RecoveryState = serde_json::from_str(&state_str)?;

            let heartbeat_str: String = row.get("last_heartbeat");
            let last_heartbeat = DateTime::parse_from_rfc3339(&heartbeat_str)
                .map_err(|e| DatabaseError::InvalidInput(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&Utc);

            let active_jobs_json: String = row.get("active_jobs");
            let active_jobs: Vec<String> = serde_json::from_str(&active_jobs_json)?;

            let metadata_json: String = row.get("metadata");
            let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;

            Ok(Some(RecoveryCheckpoint {
                state,
                last_heartbeat,
                active_jobs,
                metadata,
            }))
        } else {
            Ok(None)
        }
    }

    /// Checks if recovery is needed based on last checkpoint.
    pub async fn needs_recovery(&self) -> Result<bool, DatabaseError> {
        if let Some(checkpoint) = self.get_last_checkpoint().await? {
            // If last state was not Shutdown or Clean, recovery is needed
            Ok(matches!(checkpoint.state, RecoveryState::Interrupted))
        } else {
            // No checkpoint means first run, no recovery needed
            Ok(false)
        }
    }

    /// Performs automatic recovery after detecting interrupted state.
    pub async fn recover(&self) -> Result<Vec<String>, DatabaseError> {
        let checkpoint = self.get_last_checkpoint().await?;

        if let Some(checkpoint) = checkpoint {
            tracing::info!(
                "Recovering from {:?} state, last heartbeat: {}",
                checkpoint.state,
                checkpoint.last_heartbeat
            );

            let recovered_jobs = checkpoint.active_jobs.clone();

            // Mark as clean after recovery
            self.checkpoint(RecoveryState::Clean, vec![], serde_json::Value::Null)
                .await?;

            Ok(recovered_jobs)
        } else {
            Ok(vec![])
        }
    }

    /// Records a clean shutdown.
    pub async fn shutdown(&self) -> Result<(), DatabaseError> {
        self.checkpoint(RecoveryState::Shutdown, vec![], serde_json::Value::Null)
            .await
    }

    /// Records active job for tracking.
    pub async fn register_job(&self, job_name: String) -> Result<(), DatabaseError> {
        let mut checkpoint = self
            .get_last_checkpoint()
            .await?
            .unwrap_or(RecoveryCheckpoint {
                state: RecoveryState::Clean,
                last_heartbeat: Utc::now(),
                active_jobs: vec![],
                metadata: serde_json::Value::Null,
            });

        checkpoint.active_jobs.push(job_name);
        self.checkpoint(
            checkpoint.state,
            checkpoint.active_jobs,
            checkpoint.metadata,
        )
        .await
    }

    /// Removes completed job from tracking.
    pub async fn complete_job(&self, job_name: &str) -> Result<(), DatabaseError> {
        let mut checkpoint = self
            .get_last_checkpoint()
            .await?
            .unwrap_or(RecoveryCheckpoint {
                state: RecoveryState::Clean,
                last_heartbeat: Utc::now(),
                active_jobs: vec![],
                metadata: serde_json::Value::Null,
            });

        checkpoint.active_jobs.retain(|j| j != job_name);
        self.checkpoint(
            checkpoint.state,
            checkpoint.active_jobs,
            checkpoint.metadata,
        )
        .await
    }

    /// Updates heartbeat to signal runtime is alive.
    pub async fn heartbeat(&self) -> Result<(), DatabaseError> {
        let checkpoint = self
            .get_last_checkpoint()
            .await?
            .unwrap_or(RecoveryCheckpoint {
                state: RecoveryState::Clean,
                last_heartbeat: Utc::now(),
                active_jobs: vec![],
                metadata: serde_json::Value::Null,
            });

        self.checkpoint(
            checkpoint.state,
            checkpoint.active_jobs,
            checkpoint.metadata,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn setup() -> (RecoveryService, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("test.db");
        let db = Database::initialize_at(&db_path).await.unwrap();
        let service = RecoveryService::new(db.pool().clone());
        service.initialize().await.unwrap();
        (service, tmp)
    }

    #[tokio::test]
    async fn initial_state_needs_no_recovery() {
        let (service, _tmp) = setup().await;
        assert!(!service.needs_recovery().await.unwrap());
    }

    #[tokio::test]
    async fn interrupted_state_needs_recovery() {
        let (service, _tmp) = setup().await;

        service
            .checkpoint(RecoveryState::Interrupted, vec![], serde_json::Value::Null)
            .await
            .unwrap();

        assert!(service.needs_recovery().await.unwrap());
    }

    #[tokio::test]
    async fn shutdown_state_needs_no_recovery() {
        let (service, _tmp) = setup().await;

        service.shutdown().await.unwrap();

        assert!(!service.needs_recovery().await.unwrap());
    }

    #[tokio::test]
    async fn recover_clears_interrupted_state() {
        let (service, _tmp) = setup().await;

        service
            .checkpoint(
                RecoveryState::Interrupted,
                vec!["test_job".to_string()],
                serde_json::Value::Null,
            )
            .await
            .unwrap();

        let recovered = service.recover().await.unwrap();
        assert_eq!(recovered, vec!["test_job".to_string()]);

        // Give the database time to commit
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(!service.needs_recovery().await.unwrap());
    }

    #[tokio::test]
    async fn job_tracking_works() {
        let (service, _tmp) = setup().await;

        service.register_job("job1".to_string()).await.unwrap();
        service.register_job("job2".to_string()).await.unwrap();

        let checkpoint = service.get_last_checkpoint().await.unwrap().unwrap();
        assert_eq!(checkpoint.active_jobs.len(), 2);

        service.complete_job("job1").await.unwrap();

        let checkpoint = service.get_last_checkpoint().await.unwrap().unwrap();
        assert_eq!(checkpoint.active_jobs.len(), 1);
        assert_eq!(checkpoint.active_jobs[0], "job2");
    }
}
