//! Owns every SQL statement that touches the `settings` table
//! (blueprint §7.2). A minimal key/value repository — Phase 3 needs it
//! for exactly one key (`watched_paths`), but any future setting reuses
//! it rather than getting its own bespoke column somewhere.

use chrono::Utc;
use sqlx::SqlitePool;

use crate::errors::DatabaseError;

#[derive(Debug, Clone)]
pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Fetches a setting's raw value, if it has ever been set. Values are
    /// stored as opaque strings (typically JSON-encoded by the caller,
    /// as [`crate::commands::watcher`] does for the watched-paths list) —
    /// this repository doesn't interpret them.
    pub async fn get(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|(value,)| value))
    }

    /// Upserts a setting's value: inserts it if unset, overwrites it if
    /// already present.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await?;

        tracing::debug!(key, "setting updated");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_database;

    #[tokio::test]
    async fn get_returns_none_for_an_unset_key() {
        let (database, _guard) = test_database().await;
        let repo = SettingsRepository::new(database.pool().clone());

        assert_eq!(repo.get("watched_paths").await.unwrap(), None);
    }

    #[tokio::test]
    async fn set_then_get_round_trip() {
        let (database, _guard) = test_database().await;
        let repo = SettingsRepository::new(database.pool().clone());

        repo.set("watched_paths", "[\"/a\",\"/b\"]").await.unwrap();

        assert_eq!(
            repo.get("watched_paths").await.unwrap(),
            Some("[\"/a\",\"/b\"]".to_string())
        );
    }

    #[tokio::test]
    async fn set_overwrites_an_existing_value() {
        let (database, _guard) = test_database().await;
        let repo = SettingsRepository::new(database.pool().clone());

        repo.set("k", "1").await.unwrap();
        repo.set("k", "2").await.unwrap();

        assert_eq!(repo.get("k").await.unwrap(), Some("2".to_string()));
    }
}
