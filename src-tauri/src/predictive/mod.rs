//! Predictive Intelligence & Workflow Automation (Phase 5F)
//!
//! Provides predictive capabilities and workflow automation based on
//! historical timeline data, session patterns, and context memory.

pub mod automation;
pub mod engine;
pub mod learning;
pub mod models;
pub mod repository;
pub mod workflow;

pub use automation::AutomationEngine;
pub use engine::PredictiveEngine;
pub use learning::AdaptiveLearning;
pub use models::*;
pub use repository::PredictiveRepository;
pub use workflow::WorkflowEngine;
