//! Recommendation system for intelligent workspace insights.

mod engine;
mod generators;
mod models;
mod scoring;

pub use engine::RecommendationEngine;
pub use models::{
    Recommendation, RecommendationAction, RecommendationCategory, RecommendationPriority,
};
pub use scoring::RecommendationScoringEngine;
