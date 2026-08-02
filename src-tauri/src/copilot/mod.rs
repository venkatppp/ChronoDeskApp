//! Copilot - AI-powered workspace assistant.

pub mod conversation;
pub mod engine;
pub mod execution;
pub mod execution_checkpoint;
pub mod execution_context;
pub mod execution_engine;
pub mod execution_repository;
pub mod models;
pub mod planner;
pub mod proactive_detector;
pub mod proactive_engine;
pub mod proactive_models;
pub mod repository;
pub mod streaming;
pub mod tool_calling;
pub mod tools;

pub use conversation::ConversationManager;
pub use engine::CopilotEngine;
pub use execution::*;
pub use execution_checkpoint::ExecutionCheckpoint;
pub use execution_engine::ExecutionEngine;
pub use execution_repository::ExecutionRepository;
pub use models::*;
pub use planner::{Planner, PlannerError, PlannerReport};
pub use proactive_detector::ProactiveDetector;
pub use proactive_engine::ProactiveEngine;
pub use proactive_models::*;
pub use repository::CopilotRepository;
pub use streaming::StreamingSessionManager;
pub use tools::ToolExecutor;
pub use tools::ToolPermissionService;
