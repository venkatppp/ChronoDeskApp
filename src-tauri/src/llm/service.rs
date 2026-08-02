//! LLM Service - Manages LLM provider lifecycle and requests

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::errors::DatabaseError;
use crate::llm::{LLMProvider, LLMRequest, LLMResponse, LLMSettings, OpenAIProvider, TokenCounter};
use crate::repositories::LLMRepository;

/// LLM service that manages provider configuration and requests
pub struct LLMService {
    repository: Arc<LLMRepository>,
    provider: Arc<RwLock<Option<Arc<dyn LLMProvider>>>>,
}

impl LLMService {
    /// Creates a new LLM service
    pub fn new(repository: Arc<LLMRepository>) -> Self {
        Self {
            repository,
            provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Initializes the provider from stored settings
    pub async fn initialize(&self) -> Result<(), DatabaseError> {
        let settings = self.repository.get_settings().await?;

        if !settings.is_configured() {
            // No provider configured yet
            return Ok(());
        }

        self.update_provider(&settings).await?;
        Ok(())
    }

    /// Gets the current settings
    pub async fn get_settings(&self) -> Result<LLMSettings, DatabaseError> {
        self.repository.get_settings().await
    }

    /// Updates settings and reinitializes provider
    pub async fn update_settings(&self, settings: &LLMSettings) -> Result<(), DatabaseError> {
        settings.validate().map_err(DatabaseError::InvalidInput)?;

        self.repository.update_settings(settings).await?;
        self.update_provider(settings).await?;

        Ok(())
    }

    /// Tests the current provider connection
    pub async fn test_connection(&self) -> Result<(), String> {
        let provider = self.provider.read().await;

        match provider.as_ref() {
            Some(p) => p.test_connection().await.map_err(|e| e.to_string()),
            None => Err("LLM provider not configured".to_string()),
        }
    }

    /// Sends a completion request
    pub async fn complete(
        &self,
        request: LLMRequest,
        conversation_id: Option<uuid::Uuid>,
    ) -> Result<LLMResponse, String> {
        let provider = self.provider.read().await;

        let provider = provider.as_ref().ok_or_else(|| {
            "LLM provider not configured. Please configure API settings.".to_string()
        })?;

        // Ensure messages fit in context window
        let settings = self
            .repository
            .get_settings()
            .await
            .map_err(|e| e.to_string())?;
        let truncated_messages = TokenCounter::truncate_to_context(
            &request.messages,
            settings.context_window,
            settings.max_tokens,
        );

        let mut truncated_request = request.clone();
        truncated_request.messages = truncated_messages;

        // Send request
        let response = provider
            .complete(truncated_request)
            .await
            .map_err(|e| e.to_string())?;

        // Record usage
        let _ = self
            .repository
            .record_usage(conversation_id, &response.usage, &response.model)
            .await;

        Ok(response)
    }

    /// Checks if provider is configured
    pub fn is_configured(&self) -> bool {
        // Use try_read to avoid blocking
        match self.provider.try_read() {
            Ok(guard) => guard.is_some(),
            Err(_) => false,
        }
    }

    /// Updates the provider instance
    async fn update_provider(&self, settings: &LLMSettings) -> Result<(), DatabaseError> {
        let provider: Arc<dyn LLMProvider> = match settings.provider {
            crate::llm::LLMProviderType::OpenAI => Arc::new(
                OpenAIProvider::openai(settings.api_key.clone(), settings.model.clone())
                    .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?,
            ),
            crate::llm::LLMProviderType::Ollama => Arc::new(
                OpenAIProvider::ollama(settings.base_url.clone(), settings.model.clone())
                    .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?,
            ),
            crate::llm::LLMProviderType::Custom => Arc::new(
                OpenAIProvider::new(
                    settings.base_url.clone(),
                    settings.api_key.clone(),
                    settings.model.clone(),
                )
                .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?,
            ),
        };

        *self.provider.write().await = Some(provider);
        Ok(())
    }
}
