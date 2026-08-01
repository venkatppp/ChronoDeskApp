//! Copilot - AI-powered workspace assistant.

pub mod conversation;
pub mod engine;
pub mod models;
pub mod repository;
pub mod tools;

pub use conversation::ConversationManager;
pub use engine::CopilotEngine;
pub use models::*;
pub use repository::CopilotRepository;
pub use tools::ToolExecutor;
