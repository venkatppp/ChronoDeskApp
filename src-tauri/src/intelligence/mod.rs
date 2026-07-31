//! Intelligence layer - recommendations and workspace health monitoring.
//!
//! This module provides intelligent insights and recommendations based on
//! user behavior, workspace state, and analytics data.

pub mod health;
pub mod recommendation;

pub use health::{HealthFactor, WorkspaceHealth, WorkspaceHealthEngine};
pub use recommendation::{
    Recommendation, RecommendationCategory, RecommendationEngine, RecommendationPriority,
};
