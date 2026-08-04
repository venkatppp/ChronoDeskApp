//! LLM Settings Repository

use std::sync::Arc;

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::llm::{ApiKeyStorageState, LLMProviderType, LLMSettings, SecretStore, TokenUsage};

const API_KEY_MARKER: &str = "__KEYCHAIN__";
const SECRET_SERVICE: &str = "ChronoDesk LLM API Key";
const SECRET_ACCOUNT: &str = "default";

/// Repository for LLM settings and usage tracking
pub struct LLMRepository {
    pool: SqlitePool,
    secret_store: Arc<dyn SecretStore>,
}

impl LLMRepository {
    /// Creates a new LLM repository
    pub fn new(pool: SqlitePool, secret_store: Arc<dyn SecretStore>) -> Self {
        Self { pool, secret_store }
    }

    /// Gets the current LLM settings
    pub async fn get_settings(&self) -> Result<LLMSettings, DatabaseError> {
        self.migrate_plaintext_api_key().await?;

        let row = sqlx::query(
            r#"
            SELECT provider, base_url, api_key, model, temperature, max_tokens, context_window
            FROM llm_settings
            WHERE id = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let provider_str: String = row.get("provider");
        let provider = provider_from_str(&provider_str);
        let stored_api_key: String = row.get("api_key");
        let api_key = if stored_api_key == API_KEY_MARKER {
            self.secret_store
                .get(SECRET_SERVICE, SECRET_ACCOUNT)
                .map_err(|e| DatabaseError::InvalidInput(format!("LLM API key unavailable: {e}")))?
        } else {
            stored_api_key
        };

        Ok(LLMSettings {
            provider,
            base_url: row.get("base_url"),
            api_key,
            model: row.get("model"),
            temperature: row.get::<f64, _>("temperature") as f32,
            max_tokens: row.get::<i64, _>("max_tokens") as usize,
            context_window: row.get::<i64, _>("context_window") as usize,
        })
    }

    /// Updates LLM settings
    pub async fn update_settings(&self, settings: &LLMSettings) -> Result<(), DatabaseError> {
        let provider = settings.provider.to_string();

        self.secret_store
            .store(SECRET_SERVICE, SECRET_ACCOUNT, &settings.api_key)
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("failed to store LLM API key: {e}"))
            })?;

        sqlx::query(
            r#"
            UPDATE llm_settings
            SET provider = ?,
                base_url = ?,
                api_key = ?,
                model = ?,
                temperature = ?,
                max_tokens = ?,
                context_window = ?,
                updated_at = datetime('now')
            WHERE id = 1
            "#,
        )
        .bind(&provider)
        .bind(&settings.base_url)
        .bind(API_KEY_MARKER)
        .bind(&settings.model)
        .bind(settings.temperature as f64)
        .bind(settings.max_tokens as i64)
        .bind(settings.context_window as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// How the LLM API key is stored today. Read-only and side-effect
    /// free — unlike `get_settings`, it never migrates a plaintext key —
    /// so the RC-10 M4 security validator can inspect storage without
    /// changing it. The raw key value is never returned.
    pub async fn api_key_storage_state(&self) -> Result<ApiKeyStorageState, DatabaseError> {
        let row = sqlx::query("SELECT api_key FROM llm_settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        let stored_api_key: String = row.get("api_key");

        if stored_api_key.is_empty() {
            return Ok(ApiKeyStorageState::None);
        }
        if stored_api_key != API_KEY_MARKER {
            return Ok(ApiKeyStorageState::Plaintext);
        }

        match self.secret_store.get(SECRET_SERVICE, SECRET_ACCOUNT) {
            Ok(_) => Ok(ApiKeyStorageState::Secure),
            Err(_) => Ok(ApiKeyStorageState::SecretStoreUnavailable),
        }
    }

    /// Deletes the stored LLM credential and clears the SQLite marker.
    pub async fn delete_api_key(&self) -> Result<(), DatabaseError> {
        self.secret_store
            .delete(SECRET_SERVICE, SECRET_ACCOUNT)
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("failed to delete LLM API key: {e}"))
            })?;

        sqlx::query(
            r#"
            UPDATE llm_settings
            SET api_key = '', updated_at = datetime('now')
            WHERE id = 1
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Records token usage
    pub async fn record_usage(
        &self,
        conversation_id: Option<Uuid>,
        usage: &TokenUsage,
        model: &str,
    ) -> Result<(), DatabaseError> {
        let id = Uuid::new_v4().to_string();
        let conversation_id_str = conversation_id.map(|c| c.to_string());

        sqlx::query(
            r#"
            INSERT INTO llm_usage (id, conversation_id, prompt_tokens, completion_tokens, total_tokens, model)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&conversation_id_str)
        .bind(usage.prompt_tokens as i64)
        .bind(usage.completion_tokens as i64)
        .bind(usage.total_tokens as i64)
        .bind(model)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets total token usage for a conversation
    pub async fn get_conversation_usage(
        &self,
        conversation_id: Uuid,
    ) -> Result<TokenUsage, DatabaseError> {
        let conversation_id_str = conversation_id.to_string();

        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
                COALESCE(SUM(completion_tokens), 0) as completion_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens
            FROM llm_usage
            WHERE conversation_id = ?
            "#,
        )
        .bind(&conversation_id_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(TokenUsage {
            prompt_tokens: row.get::<i64, _>("prompt_tokens") as usize,
            completion_tokens: row.get::<i64, _>("completion_tokens") as usize,
            total_tokens: row.get::<i64, _>("total_tokens") as usize,
        })
    }

    /// Gets total token usage across all conversations
    pub async fn get_total_usage(&self) -> Result<TokenUsage, DatabaseError> {
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(prompt_tokens), 0) as prompt_tokens,
                COALESCE(SUM(completion_tokens), 0) as completion_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens
            FROM llm_usage
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(TokenUsage {
            prompt_tokens: row.get::<i64, _>("prompt_tokens") as usize,
            completion_tokens: row.get::<i64, _>("completion_tokens") as usize,
            total_tokens: row.get::<i64, _>("total_tokens") as usize,
        })
    }

    async fn migrate_plaintext_api_key(&self) -> Result<(), DatabaseError> {
        let row = sqlx::query("SELECT api_key FROM llm_settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        let api_key: String = row.get("api_key");

        if api_key.is_empty() || api_key == API_KEY_MARKER {
            return Ok(());
        }

        self.secret_store
            .store(SECRET_SERVICE, SECRET_ACCOUNT, &api_key)
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("failed to migrate LLM API key: {e}"))
            })?;

        let stored = self
            .secret_store
            .get(SECRET_SERVICE, SECRET_ACCOUNT)
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("failed to verify migrated LLM API key: {e}"))
            })?;

        if stored != api_key {
            return Err(DatabaseError::InvalidInput(
                "failed to verify migrated LLM API key".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE llm_settings
            SET api_key = ?, updated_at = datetime('now')
            WHERE id = 1
            "#,
        )
        .bind(API_KEY_MARKER)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

fn provider_from_str(provider: &str) -> LLMProviderType {
    match provider {
        "openai" => LLMProviderType::OpenAI,
        "ollama" => LLMProviderType::Ollama,
        "custom" => LLMProviderType::Custom,
        _ => LLMProviderType::OpenAI,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::database::test_database;
    use crate::llm::{InMemorySecretStore, LLMProviderType, LLMSettings, SecretStore};

    use super::{LLMRepository, API_KEY_MARKER, SECRET_ACCOUNT, SECRET_SERVICE};

    async fn repository() -> (LLMRepository, Arc<InMemorySecretStore>, tempfile::TempDir) {
        let (database, temp_dir) = test_database().await;
        let store = Arc::new(InMemorySecretStore::new());
        let repository = LLMRepository::new(database.pool().clone(), store.clone());
        (repository, store, temp_dir)
    }

    async fn raw_api_key(repository: &LLMRepository) -> String {
        sqlx::query_scalar("SELECT api_key FROM llm_settings WHERE id = 1")
            .fetch_one(&repository.pool)
            .await
            .expect("api key row should exist")
    }

    async fn write_plaintext_api_key(repository: &LLMRepository, api_key: &str) {
        sqlx::query("UPDATE llm_settings SET api_key = ? WHERE id = 1")
            .bind(api_key)
            .execute(&repository.pool)
            .await
            .expect("plaintext api key should be written");
    }

    #[tokio::test]
    async fn first_run_migration_imports_plaintext_and_clears_sqlite() {
        let (repository, store, _temp_dir) = repository().await;
        write_plaintext_api_key(&repository, "plain-secret").await;

        let settings = repository
            .get_settings()
            .await
            .expect("settings should load after migration");

        assert_eq!(settings.api_key, "plain-secret");
        assert_eq!(raw_api_key(&repository).await, API_KEY_MARKER);
        assert_eq!(
            store
                .get(SECRET_SERVICE, SECRET_ACCOUNT)
                .expect("credential should be stored"),
            "plain-secret"
        );
    }

    #[tokio::test]
    async fn migration_failure_leaves_plaintext_in_sqlite() {
        let (database, temp_dir) = test_database().await;
        let store = Arc::new(InMemorySecretStore::with_store_failure(
            "keychain unavailable",
        ));
        let repository = LLMRepository::new(database.pool().clone(), store);
        write_plaintext_api_key(&repository, "plain-secret").await;

        let result = repository.get_settings().await;

        assert!(result.is_err());
        assert_eq!(raw_api_key(&repository).await, "plain-secret");
        drop(temp_dir);
    }

    #[tokio::test]
    async fn save_stores_key_only_in_secret_store() {
        let (repository, store, _temp_dir) = repository().await;
        let settings = LLMSettings::openai("saved-secret".to_string(), "gpt-4o".to_string());

        repository
            .update_settings(&settings)
            .await
            .expect("settings should save");

        assert_eq!(raw_api_key(&repository).await, API_KEY_MARKER);
        assert_eq!(
            store
                .get(SECRET_SERVICE, SECRET_ACCOUNT)
                .expect("credential should be saved"),
            "saved-secret"
        );
    }

    #[tokio::test]
    async fn load_reads_key_from_secret_store() {
        let (repository, store, _temp_dir) = repository().await;
        store
            .store(SECRET_SERVICE, SECRET_ACCOUNT, "stored-secret")
            .expect("credential should be seeded");
        sqlx::query("UPDATE llm_settings SET api_key = ?, provider = ?, model = ? WHERE id = 1")
            .bind(API_KEY_MARKER)
            .bind("custom")
            .bind("custom-model")
            .execute(&repository.pool)
            .await
            .expect("settings marker should be written");

        let settings = repository
            .get_settings()
            .await
            .expect("settings should load");

        assert_eq!(settings.provider, LLMProviderType::Custom);
        assert_eq!(settings.model, "custom-model");
        assert_eq!(settings.api_key, "stored-secret");
    }

    #[tokio::test]
    async fn update_replaces_secret_store_value() {
        let (repository, store, _temp_dir) = repository().await;
        repository
            .update_settings(&LLMSettings::openai(
                "first-secret".to_string(),
                "gpt-4o".to_string(),
            ))
            .await
            .expect("first settings should save");
        repository
            .update_settings(&LLMSettings::custom(
                "https://example.test/v1".to_string(),
                "second-secret".to_string(),
                "example-model".to_string(),
            ))
            .await
            .expect("updated settings should save");

        let settings = repository
            .get_settings()
            .await
            .expect("updated settings should load");

        assert_eq!(settings.provider, LLMProviderType::Custom);
        assert_eq!(settings.api_key, "second-secret");
        assert_eq!(raw_api_key(&repository).await, API_KEY_MARKER);
        assert_eq!(
            store
                .get(SECRET_SERVICE, SECRET_ACCOUNT)
                .expect("credential should be updated"),
            "second-secret"
        );
    }

    #[tokio::test]
    async fn delete_removes_secret_and_clears_marker() {
        let (repository, store, _temp_dir) = repository().await;
        repository
            .update_settings(&LLMSettings::openai(
                "secret-to-delete".to_string(),
                "gpt-4o".to_string(),
            ))
            .await
            .expect("settings should save");

        repository
            .delete_api_key()
            .await
            .expect("credential should delete");

        assert_eq!(raw_api_key(&repository).await, "");
        assert!(store.get(SECRET_SERVICE, SECRET_ACCOUNT).is_err());
    }

    #[tokio::test]
    async fn missing_credential_returns_clear_error() {
        let (repository, _store, _temp_dir) = repository().await;
        sqlx::query("UPDATE llm_settings SET api_key = ? WHERE id = 1")
            .bind(API_KEY_MARKER)
            .execute(&repository.pool)
            .await
            .expect("settings marker should be written");

        let error = repository
            .get_settings()
            .await
            .expect_err("missing credential should error");

        assert!(error.to_string().contains("LLM API key unavailable"));
    }

    #[tokio::test]
    async fn unsupported_keychain_backend_returns_clear_error() {
        let (database, temp_dir) = test_database().await;
        let store = Arc::new(InMemorySecretStore::with_get_failure(
            "unsupported keychain backend",
        ));
        let repository = LLMRepository::new(database.pool().clone(), store);
        sqlx::query("UPDATE llm_settings SET api_key = ? WHERE id = 1")
            .bind(API_KEY_MARKER)
            .execute(&repository.pool)
            .await
            .expect("settings marker should be written");

        let error = repository
            .get_settings()
            .await
            .expect_err("unsupported backend should error");

        assert!(error.to_string().contains("LLM API key unavailable"));
        assert!(error.to_string().contains("unsupported keychain backend"));
        drop(temp_dir);
    }
}
