//! Recommendation scoring engine.

use super::models::{Recommendation, RecommendationPriority};

/// Engine for scoring and prioritizing recommendations.
#[derive(Clone)]
pub struct RecommendationScoringEngine;

impl RecommendationScoringEngine {
    /// Creates a new scoring engine.
    pub fn new() -> Self {
        Self
    }

    /// Scores a recommendation and assigns priority, confidence, impact, and effort.
    pub fn score_recommendation(&self, mut recommendation: Recommendation) -> Recommendation {
        // Calculate composite score
        let composite_score = self.calculate_composite_score(&recommendation);

        // Assign priority based on composite score
        recommendation.priority = self.determine_priority(composite_score, &recommendation);

        recommendation
    }

    /// Scores multiple recommendations and sorts by priority and composite score.
    pub fn score_and_rank(&self, recommendations: Vec<Recommendation>) -> Vec<Recommendation> {
        let mut scored: Vec<_> = recommendations
            .into_iter()
            .map(|rec| self.score_recommendation(rec))
            .collect();

        // Sort by priority (descending) and then by composite score (descending)
        scored.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| {
                let score_a = self.calculate_composite_score(a);
                let score_b = self.calculate_composite_score(b);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        scored
    }

    /// Calculates a composite score from confidence, impact, and effort.
    fn calculate_composite_score(&self, recommendation: &Recommendation) -> f64 {
        // Score = (confidence * impact) - (effort_penalty)
        // Higher confidence and impact = higher score
        // Higher effort = lower score (but less weighted than impact)
        let effort_penalty = recommendation.effort * 0.3;
        let benefit = recommendation.confidence * recommendation.impact;

        (benefit - effort_penalty).clamp(0.0, 1.0)
    }

    /// Determines priority based on composite score and recommendation characteristics.
    fn determine_priority(
        &self,
        composite_score: f64,
        recommendation: &Recommendation,
    ) -> RecommendationPriority {
        // High impact + high confidence = higher priority
        let is_high_impact = recommendation.impact >= 0.7;
        let is_high_confidence = recommendation.confidence >= 0.7;
        let is_low_effort = recommendation.effort < 0.3;

        // Critical: high score, high impact, high confidence
        if composite_score >= 0.8 && is_high_impact && is_high_confidence {
            return RecommendationPriority::Critical;
        }

        // High: good score or high impact with reasonable confidence
        if composite_score >= 0.6 || (is_high_impact && recommendation.confidence >= 0.5) {
            return RecommendationPriority::High;
        }

        // Medium: moderate score or low effort with reasonable benefit
        if composite_score >= 0.4 || (is_low_effort && composite_score >= 0.3) {
            return RecommendationPriority::Medium;
        }

        // Low: everything else
        RecommendationPriority::Low
    }

    /// Filters out expired recommendations.
    pub fn filter_expired(&self, recommendations: Vec<Recommendation>) -> Vec<Recommendation> {
        recommendations
            .into_iter()
            .filter(|rec| !rec.is_expired())
            .collect()
    }

    /// Limits recommendations to top N by score.
    pub fn limit_top_n(
        &self,
        recommendations: Vec<Recommendation>,
        n: usize,
    ) -> Vec<Recommendation> {
        recommendations.into_iter().take(n).collect()
    }
}

impl Default for RecommendationScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}
