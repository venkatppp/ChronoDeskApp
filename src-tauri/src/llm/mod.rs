//! LLM Module - Large Language Model Provider Abstraction
//!
//! Provides a pluggable interface for LLM providers with support for:
//! - OpenAI-compatible APIs
//! - Streaming responses
//! - Token usage tracking
//! - Conversation context management
//! - Retry/backoff logic
//! - Provider configuration

pub mod models;
pub mod openai_provider;
pub mod provider;
pub mod secret_store;
pub mod service;
pub mod settings;
pub mod token_counter;

pub use models::*;
pub use openai_provider::OpenAIProvider;
pub use provider::{LLMProvider, StreamEvent};
#[cfg(test)]
pub use secret_store::InMemorySecretStore;
pub use secret_store::{KeyringSecretStore, SecretStore};
pub use service::LLMService;
pub use settings::LLMSettings;
pub use token_counter::TokenCounter;
