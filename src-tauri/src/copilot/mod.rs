//! Copilot - AI-powered workspace assistant.

pub mod conversation;
pub mod engine;
pub mod models;
pub mod proactive_detector;
pub mod proactive_engine;
pub mod proactive_models;
pub mod repository;
pub mod tools;

pub use conversation::ConversationManager;
pub use engine::CopilotEngine;
pub use models::*;
pub use proactive_detector::ProactiveDetector;
pub use proactive_engine::ProactiveEngine;
pub use proactive_models::*;
pub use repository::CopilotRepository;
pub use tools::ToolExecutor;
