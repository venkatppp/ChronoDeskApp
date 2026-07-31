//! Context Scoring Engine
//!
//! Provides modular, transparent scoring for sessions and other intelligence
//! features. Each scoring factor is independent, testable, and includes a
//! human-readable explanation of its contribution.
//!
//! ## Design
//!
//! The scoring engine is built around the `ScoreCalculator` trait. Each
//! calculator implements one scoring dimension (e.g. focus duration,
//! deep editing, context switching) and returns a `ScoreFactor` with:
//! - name: What aspect is being measured
//! - weight: How important this factor is (0-1)
//! - value: Normalized score for this factor (0-1)
//! - reason: Human-readable explanation
//!
//! The final score is computed as a weighted average of all factors.
//!
//! ## Extensibility
//!
//! New scoring dimensions can be added by:
//! 1. Implementing the `ScoreCalculator` trait
//! 2. Adding the calculator to the engine's registry
//!
//! No hard-coded scoring tables or if/else ladders.

pub mod calculators;
pub mod engine;

pub use engine::ContextScoringEngine;

use crate::session::types::{ScoreFactor, SessionContext};

/// Trait for modular scoring calculators.
///
/// Each calculator computes one dimension of a score (e.g. focus duration,
/// workspace consistency) and returns a transparent `ScoreFactor` with a
/// human-readable reason.
pub trait ScoreCalculator: Send + Sync + std::fmt::Debug {
    /// Name of this scoring dimension.
    fn name(&self) -> &str;

    /// Weight of this factor in the final score (0-1).
    /// All weights are normalized by the engine, so they don't need to sum to 1.
    fn weight(&self) -> f64;

    /// Calculate the score factor for the given context.
    ///
    /// Returns a `ScoreFactor` with:
    /// - value: Normalized score (0-1)
    /// - reason: Explanation of why this value was assigned
    fn calculate(&self, context: &SessionContext) -> ScoreFactor;
}
