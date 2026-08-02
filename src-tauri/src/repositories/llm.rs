//! LLM Settings Repository

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::llm::{LLMSettings, TokenUsage};

/// Repository for LLM settings and usage tracking
pub struct LLMRepository {
    pool: SqlitePool,
}

impl LLMRepository {
    /// Creates a new LLM repository
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Gets the current LLM settings
    pub async fn get_settings(&self) -> Result<LLMSettings, DatabaseError> {
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
        let provider = match provider_str.as_str() {
            "openai" => crate::llm::LLMProviderType::OpenAI,
            "ollama" => crate::llm::LLMProviderType::Ollama,
            "custom" => crate::llm::LLMProviderType::Custom,
            _ => crate::llm::LLMProviderType::OpenAI,
        };

        Ok(LLMSettings {
            provider,
            base_url: row.get("base_url"),
            api_key: row.get("api_key"),
            model: row.get("model"),
            temperature: row.get::<f64, _>("temperature") as f32,
            max_tokens: row.get::<i64, _>("max_tokens") as usize,
            context_window: row.get::<i64, _>("context_window") as usize,
        })
    }

    /// Updates LLM settings
    pub async fn update_settings(&self, settings: &LLMSettings) -> Result<(), DatabaseError> {
        let provider = settings.provider.to_string();

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
        .bind(&settings.api_key)
        .bind(&settings.model)
        .bind(settings.temperature as f64)
        .bind(settings.max_tokens as i64)
        .bind(settings.context_window as i64)
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
}
