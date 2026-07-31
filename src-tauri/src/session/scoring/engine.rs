//! Context Scoring Engine implementation.
//!
//! Combines multiple `ScoreCalculator` implementations into a single
//! scoring system. The engine normalizes weights and computes the final
//! score as a weighted average.

use crate::session::scoring::calculators::{
    CompletionSignalsCalculator, ContextSwitchingCalculator, DeepEditingCalculator,
    FocusDurationCalculator, WorkspaceConsistencyCalculator,
};
use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ProductivityScore, SessionContext};

/// Context Scoring Engine for sessions and other intelligence features.
///
/// Computes scores by combining multiple independent scoring factors.
/// Each factor is transparent and includes a human-readable reason.
#[derive(Debug)]
pub struct ContextScoringEngine {
    calculators: Vec<Box<dyn ScoreCalculator>>,
}

impl Default for ContextScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextScoringEngine {
    /// Creates a new scoring engine with the default set of calculators.
    pub fn new() -> Self {
        Self {
            calculators: vec![
                Box::new(FocusDurationCalculator),
                Box::new(DeepEditingCalculator),
                Box::new(ContextSwitchingCalculator),
                Box::new(CompletionSignalsCalculator),
                Box::new(WorkspaceConsistencyCalculator),
            ],
        }
    }

    /// Calculates a productivity score for the given session context.
    ///
    /// Returns a `ProductivityScore` with the final score (0-100) and all
    /// contributing factors with their explanations.
    pub fn calculate_score(&self, context: &SessionContext) -> ProductivityScore {
        let factors = self
            .calculators
            .iter()
            .map(|calc| calc.calculate(context))
            .collect::<Vec<_>>();

        // Compute weighted average
        let weighted_sum: f64 = factors.iter().map(|f| f.value * f.weight).sum();
        let total_weight: f64 = factors.iter().map(|f| f.weight).sum();

        let normalized_score = if total_weight > 0.0 {
            (weighted_sum / total_weight) * 100.0
        } else {
            0.0
        };

        ProductivityScore {
            score: normalized_score.clamp(0.0, 100.0),
            factors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewTimelineEvent, TimelineEventType};
    use crate::session::types::Session;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn scoring_engine_computes_score_with_factors() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();

        let events = vec![
            NewTimelineEvent {
                workspace_id,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: now,
                metadata: None,
            },
            NewTimelineEvent {
                workspace_id,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: now + chrono::Duration::minutes(30),
                metadata: None,
            },
        ];

        let session = Session {
            workspace_id,
            started_at: now,
            ended_at: now + chrono::Duration::minutes(60),
            duration_seconds: 3600,
            event_count: events.len(),
            file_count: 2,
            languages: vec!["Rust".to_string()],
            productivity_score: None,
            events: vec![], // Empty for this test
        };

        let context = SessionContext::from(&session);
        let engine = ContextScoringEngine::new();
        let score = engine.calculate_score(&context);

        assert!(score.score >= 0.0 && score.score <= 100.0);
        assert_eq!(score.factors.len(), 5); // 5 default calculators
        assert!(score.factors.iter().all(|f| f.weight > 0.0));
        assert!(score
            .factors
            .iter()
            .all(|f| f.value >= 0.0 && f.value <= 1.0));
        assert!(score.factors.iter().all(|f| !f.reason.is_empty()));
    }

    #[test]
    fn scoring_engine_clamps_to_valid_range() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();

        let session = Session {
            workspace_id,
            started_at: now,
            ended_at: now,
            duration_seconds: 0,
            event_count: 0,
            file_count: 0,
            languages: vec![],
            productivity_score: None,
            events: vec![],
        };

        let context = SessionContext::from(&session);
        let engine = ContextScoringEngine::new();
        let score = engine.calculate_score(&context);

        assert!(score.score >= 0.0);
        assert!(score.score <= 100.0);
    }
}
