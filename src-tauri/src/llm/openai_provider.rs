//! OpenAI-compatible LLM Provider

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::models::{LLMError, LLMMessage, LLMRequest, LLMResponse, StreamChunk, TokenUsage};
use super::provider::{LLMProvider, StreamEvent};

/// OpenAI API compatible provider
pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    #[allow(dead_code)]
    api_key: String,
    model: String,
    max_retries: u32,
    #[allow(dead_code)]
    timeout: Duration,
}

impl OpenAIProvider {
    /// Creates a new OpenAI provider
    pub fn new(base_url: String, api_key: String, model: String) -> Result<Self, LLMError> {
        if api_key.is_empty() {
            return Err(LLMError::InvalidApiKey);
        }

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|_| LLMError::InvalidApiKey)?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            base_url,
            api_key,
            model,
            max_retries: 3,
            timeout: Duration::from_secs(120),
        })
    }

    /// Creates an OpenAI provider (official API)
    pub fn openai(api_key: String, model: String) -> Result<Self, LLMError> {
        Self::new("https://api.openai.com/v1".to_string(), api_key, model)
    }

    /// Creates an Ollama provider (local)
    pub fn ollama(base_url: String, model: String) -> Result<Self, LLMError> {
        Self::new(base_url, "ollama".to_string(), model)
    }

    /// Retry logic with exponential backoff
    async fn retry_with_backoff<F, Fut, T>(&self, mut f: F) -> Result<T, LLMError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, LLMError>>,
    {
        let mut attempt = 0;
        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;
                    if attempt >= self.max_retries {
                        return Err(e);
                    }

                    // Don't retry on certain errors
                    match &e {
                        LLMError::InvalidApiKey
                        | LLMError::InvalidRequest(_)
                        | LLMError::NotConfigured => return Err(e),
                        _ => {}
                    }

                    // Exponential backoff: 1s, 2s, 4s
                    let delay = Duration::from_secs(2u64.pow(attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        self.retry_with_backoff(|| async {
            let api_request = OpenAICompletionRequest {
                model: self.model.clone(),
                messages: request.messages.clone(),
                temperature: request.temperature,
                max_tokens: request.max_tokens,
                top_p: request.top_p,
                stream: Some(false),
            };

            let response = self
                .client
                .post(format!("{}/chat/completions", self.base_url))
                .json(&api_request)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();

                return Err(match status.as_u16() {
                    401 => LLMError::InvalidApiKey,
                    429 => LLMError::RateLimitExceeded,
                    400 if error_text.contains("context_length_exceeded") => {
                        LLMError::ContextLengthExceeded
                    }
                    _ => LLMError::ApiError(format!("{}: {}", status, error_text)),
                });
            }

            let api_response: OpenAICompletionResponse = response.json().await?;

            let content = api_response
                .choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_default();

            let finish_reason = api_response
                .choices
                .first()
                .and_then(|c| c.finish_reason.clone());

            Ok(LLMResponse {
                content,
                usage: TokenUsage {
                    prompt_tokens: api_response.usage.prompt_tokens,
                    completion_tokens: api_response.usage.completion_tokens,
                    total_tokens: api_response.usage.total_tokens,
                },
                model: api_response.model,
                finish_reason,
            })
        })
        .await
    }

    async fn complete_stream(
        &self,
        request: LLMRequest,
    ) -> Result<BoxStream<'static, StreamEvent>, LLMError> {
        let api_request = OpenAICompletionRequest {
            model: self.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stream: Some(true),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&api_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                401 => LLMError::InvalidApiKey,
                429 => LLMError::RateLimitExceeded,
                400 if error_text.contains("context_length_exceeded") => {
                    LLMError::ContextLengthExceeded
                }
                _ => LLMError::ApiError(format!("{}: {}", status, error_text)),
            });
        }

        let stream = response.bytes_stream();

        let event_stream = stream.filter_map(|result| {
            async move {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        // Parse SSE format: "data: {...}"
                        for line in text.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }

                                if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            return Some(StreamEvent::Chunk(StreamChunk {
                                                content: content.clone(),
                                                finish_reason: choice.finish_reason.clone(),
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                        None
                    }
                    Err(e) => Some(StreamEvent::Error(e.to_string())),
                }
            }
        });

        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        let test_request = LLMRequest {
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: Some(0.1),
            max_tokens: Some(5),
            top_p: Some(1.0),
            stream: Some(false),
        };

        self.complete(test_request).await?;
        Ok(())
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn list_models(&self) -> Result<Vec<String>, LLMError> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(LLMError::ApiError("Failed to list models".to_string()));
        }

        let models_response: OpenAIModelsResponse = response.json().await?;
        Ok(models_response.data.into_iter().map(|m| m.id).collect())
    }
}

// OpenAI API structures

#[derive(Debug, Serialize)]
struct OpenAICompletionRequest {
    model: String,
    messages: Vec<LLMMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OpenAICompletionResponse {
    #[allow(dead_code)]
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: LLMMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}
