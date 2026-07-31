//! Focus Duration Calculator
//!
//! Measures how well the session duration matches optimal focus periods.
//! Research suggests 30-120 minute sessions are ideal for deep work.

use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ScoreFactor, SessionContext};

/// Calculates score based on session duration.
///
/// Scoring logic:
/// - < 5 minutes: Very short, likely a context switch (0.0)
/// - 5-30 minutes: Brief session (0.5)
/// - 30-120 minutes: Optimal focus period (1.0)
/// - 120-180 minutes: Extended session (0.8)
/// - > 180 minutes: Marathon session (0.6)
#[derive(Debug)]
pub struct FocusDurationCalculator;

impl ScoreCalculator for FocusDurationCalculator {
    fn name(&self) -> &str {
        "Focus Duration"
    }

    fn weight(&self) -> f64 {
        0.25
    }

    fn calculate(&self, context: &SessionContext) -> ScoreFactor {
        let duration_minutes = context.duration_seconds / 60;

        let (value, reason) = if duration_minutes < 5 {
            (
                0.0,
                format!("Very short session ({} min)", duration_minutes),
            )
        } else if duration_minutes < 30 {
            (0.5, format!("Brief session ({} min)", duration_minutes))
        } else if duration_minutes <= 120 {
            (
                1.0,
                format!("Optimal focus period ({} min)", duration_minutes),
            )
        } else if duration_minutes <= 180 {
            (0.8, format!("Extended session ({} min)", duration_minutes))
        } else {
            (0.6, format!("Marathon session ({} min)", duration_minutes))
        };

        ScoreFactor {
            name: self.name().to_string(),
            weight: self.weight(),
            value,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_context(duration_seconds: i64) -> SessionContext {
        SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds,
            event_count: 1,
            file_count: 1,
            events: vec![],
        }
    }

    #[test]
    fn very_short_session_scores_zero() {
        let calc = FocusDurationCalculator;
        let context = make_context(60); // 1 minute
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.0);
        assert!(factor.reason.contains("Very short"));
    }

    #[test]
    fn brief_session_scores_medium() {
        let calc = FocusDurationCalculator;
        let context = make_context(15 * 60); // 15 minutes
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.5);
        assert!(factor.reason.contains("Brief"));
    }

    #[test]
    fn optimal_session_scores_perfect() {
        let calc = FocusDurationCalculator;
        let context = make_context(60 * 60); // 60 minutes
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 1.0);
        assert!(factor.reason.contains("Optimal"));
    }

    #[test]
    fn extended_session_scores_high() {
        let calc = FocusDurationCalculator;
        let context = make_context(150 * 60); // 150 minutes
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.8);
        assert!(factor.reason.contains("Extended"));
    }

    #[test]
    fn marathon_session_scores_medium() {
        let calc = FocusDurationCalculator;
        let context = make_context(240 * 60); // 240 minutes
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.6);
        assert!(factor.reason.contains("Marathon"));
    }
}
