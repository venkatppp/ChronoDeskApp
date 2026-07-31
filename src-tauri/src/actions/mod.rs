//! Intelligent Actions & Automation
//!
//! This module provides the action execution layer that converts recommendations
//! into concrete, executable actions with full undo support.

pub mod engine;
pub mod executors;
pub mod models;
pub mod repository;
pub mod service;

pub use engine::ActionEngine;
pub use models::{ActionHistory, ActionResult, ActionType, ExecuteActionRequest, UndoState};
pub use repository::ActionRepository;
pub use service::ActionService;
