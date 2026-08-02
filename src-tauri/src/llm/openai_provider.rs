//! OpenAI-compatible LLM Provider

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::models::{
    LLMError, LLMMessage, LLMRequest, LLMResponse, LLMTool, LLMToolCall, StreamChunk, TokenUsage,
};
use super::provider::{LLMProvider, StreamEvent};

/// OpenAI API compatible provider
pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    #[allow(dead_code)]
    api_key: String,
    model: String,
    #[allow(dead_code)]
    timeout: Duration,
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

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
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            base_url,
            api_key,
            model,
            timeout: REQUEST_TIMEOUT,
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

    fn error_for_status(status: reqwest::StatusCode, error_text: String) -> LLMError {
        match status.as_u16() {
            401 | 403 => LLMError::InvalidApiKey,
            429 => LLMError::RateLimitExceeded,
            400 if error_text.contains("context_length_exceeded") => {
                LLMError::ContextLengthExceeded
            }
            400 | 404 => LLMError::InvalidRequest(format!("HTTP {}", status)),
            code if code >= 500 => LLMError::ApiError(format!("HTTP {}: provider error", status)),
            _ => LLMError::ApiError(format!("HTTP {}: provider error", status)),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        let api_request = OpenAICompletionRequest {
            model: self.model.clone(),
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stream: Some(false),
            tools: request.tools.clone(),
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

            return Err(Self::error_for_status(status, error_text));
        }

        let api_response: OpenAICompletionResponse = response.json().await?;

        let (content, tool_calls, finish_reason) = match api_response.choices.into_iter().next() {
            Some(choice) => (
                choice.message.content,
                choice.message.tool_calls,
                choice.finish_reason,
            ),
            None => (String::new(), None, None),
        };

        Ok(LLMResponse {
            content,
            tool_calls,
            usage: TokenUsage {
                prompt_tokens: api_response.usage.prompt_tokens,
                completion_tokens: api_response.usage.completion_tokens,
                total_tokens: api_response.usage.total_tokens,
            },
            model: api_response.model,
            finish_reason,
        })
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
            tools: request.tools.clone(),
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

            return Err(Self::error_for_status(status, error_text));
        }

        let stream = response.bytes_stream();

        // Aggregates deltas so tool calls (which arrive incrementally across
        // chunks) can be handed back as a complete `Done` response, while text
        // chunks are re-emitted immediately for live UI streaming.
        let event_stream = futures::stream::unfold(
            (stream, StreamAggregator::default()),
            |mut state| async move {
                if state.1.finished {
                    return None;
                }

                while let Some(result) = state.0.next().await {
                    match result {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            let mut saw_done = false;

                            for line in text.lines() {
                                let Some(data) = line.strip_prefix("data: ") else {
                                    continue;
                                };
                                if data == "[DONE]" {
                                    saw_done = true;
                                    continue;
                                }

                                let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data)
                                else {
                                    continue;
                                };

                                if let Some(usage) = chunk.usage {
                                    state.1.usage = Some(usage);
                                }

                                let Some(choice) = chunk.choices.first() else {
                                    continue;
                                };

                                if let Some(content) = &choice.delta.content {
                                    state.1.content.push_str(content);
                                    return Some((
                                        StreamEvent::Chunk(StreamChunk {
                                            content: content.clone(),
                                            finish_reason: choice.finish_reason.clone(),
                                        }),
                                        state,
                                    ));
                                }

                                if let Some(tool_calls) = &choice.delta.tool_calls {
                                    for call in tool_calls {
                                        let index = call.index.unwrap_or(0);
                                        let slot = &mut state.1.tool_calls;
                                        slot.resize(index + 1, StreamToolCall::default());
                                        if let Some(id) = &call.id {
                                            slot[index].id = Some(id.clone());
                                        }
                                        if let Some(function) = &call.function {
                                            if let Some(name) = &function.name {
                                                slot[index].name = Some(name.clone());
                                            }
                                            if let Some(arguments) = &function.arguments {
                                                let args = slot[index]
                                                    .arguments
                                                    .get_or_insert_with(String::new);
                                                args.push_str(arguments);
                                            }
                                        }
                                    }
                                }

                                if choice.finish_reason.is_some() {
                                    state.1.finish_reason = choice.finish_reason.clone();
                                }
                            }

                            if saw_done {
                                state.1.finished = true;
                                return Some((state.1.finalize(), state));
                            }
                        }
                        Err(error) => {
                            state.1.finished = true;
                            return Some((StreamEvent::Error(error.to_string()), state));
                        }
                    }
                }

                state.1.finished = true;
                if state.1.is_empty() {
                    None
                } else {
                    Some((state.1.finalize(), state))
                }
            },
        );

        Ok(Box::pin(event_stream))
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        let test_request = LLMRequest {
            messages: vec![LLMMessage::new("user", "test")],
            temperature: Some(0.1),
            max_tokens: Some(5),
            top_p: Some(1.0),
            stream: Some(false),
            tools: None,
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
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Self::error_for_status(status, error_text));
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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<LLMTool>>,
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

#[derive(Debug, Clone, Default, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIStreamToolCallDelta>>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIStreamToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    function: Option<OpenAIStreamFunctionCall>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIStreamFunctionCall {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct StreamToolCall {
    id: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

/// Accumulates deltas across a streaming response into a single final
/// [`LLMResponse`], mirroring what a non-streaming completion returns.
#[derive(Debug, Default)]
struct StreamAggregator {
    content: String,
    tool_calls: Vec<StreamToolCall>,
    finish_reason: Option<String>,
    usage: Option<OpenAIUsage>,
    finished: bool,
}

impl StreamAggregator {
    fn is_empty(&self) -> bool {
        self.content.is_empty() && self.tool_calls.is_empty() && self.finish_reason.is_none()
    }

    fn finalize(&self) -> StreamEvent {
        let tool_calls = if self.tool_calls.is_empty() {
            None
        } else {
            Some(
                self.tool_calls
                    .iter()
                    .filter(|call| call.name.is_some() || call.id.is_some())
                    .map(|call| LLMToolCall {
                        id: call.id.clone().unwrap_or_default(),
                        name: call.name.clone().unwrap_or_default(),
                        arguments: call
                            .arguments
                            .as_deref()
                            .and_then(|arguments| serde_json::from_str(arguments).ok())
                            .unwrap_or(serde_json::Value::Null),
                    })
                    .collect(),
            )
        };

        let usage = self.usage.clone().unwrap_or_default();
        StreamEvent::Done(LLMResponse {
            content: self.content.clone(),
            tool_calls,
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            },
            model: String::new(),
            finish_reason: self.finish_reason.clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
}
