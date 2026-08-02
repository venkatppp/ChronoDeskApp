//! LLM Provider Trait

use async_trait::async_trait;
use futures::stream::BoxStream;

use super::models::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// Stream event from LLM provider
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Chunk(StreamChunk),
    Done(LLMResponse),
    Error(String),
}

/// LLM provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a completion request
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;

    /// Send a streaming completion request
    async fn complete_stream(
        &self,
        request: LLMRequest,
    ) -> Result<BoxStream<'static, StreamEvent>, LLMError>;

    /// Test provider connection and credentials
    async fn test_connection(&self) -> Result<(), LLMError>;

    /// Get provider name
    fn name(&self) -> &str;

    /// Get available models
    async fn list_models(&self) -> Result<Vec<String>, LLMError>;
}
