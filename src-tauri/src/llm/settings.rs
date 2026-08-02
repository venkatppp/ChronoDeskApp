//! LLM Settings and Configuration

use serde::{Deserialize, Serialize};

use super::models::LLMProviderType;

/// LLM provider settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: LLMProviderType,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub context_window: usize,
}

impl Default for LLMSettings {
    fn default() -> Self {
        Self {
            provider: LLMProviderType::OpenAI,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            max_tokens: 2000,
            context_window: 128000,
        }
    }
}

impl LLMSettings {
    /// Creates settings for OpenAI
    pub fn openai(api_key: String, model: String) -> Self {
        Self {
            provider: LLMProviderType::OpenAI,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key,
            model,
            temperature: 0.7,
            max_tokens: 2000,
            context_window: 128000,
        }
    }

    /// Creates settings for Ollama
    pub fn ollama(base_url: String, model: String) -> Self {
        Self {
            provider: LLMProviderType::Ollama,
            base_url,
            api_key: "ollama".to_string(),
            model,
            temperature: 0.7,
            max_tokens: 2000,
            context_window: 8192,
        }
    }

    /// Creates custom provider settings
    pub fn custom(base_url: String, api_key: String, model: String) -> Self {
        Self {
            provider: LLMProviderType::Custom,
            base_url,
            api_key,
            model,
            temperature: 0.7,
            max_tokens: 2000,
            context_window: 8192,
        }
    }

    /// Validates settings
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("Base URL is required".to_string());
        }

        if self.api_key.is_empty() {
            return Err("API key is required".to_string());
        }

        if self.model.is_empty() {
            return Err("Model is required".to_string());
        }

        if self.temperature < 0.0 || self.temperature > 2.0 {
            return Err("Temperature must be between 0.0 and 2.0".to_string());
        }

        if self.max_tokens == 0 {
            return Err("Max tokens must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Checks if settings are configured
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.model.is_empty()
    }
}
