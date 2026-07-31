//! Context Switching Calculator
//!
//! Measures how often the user switched between different files.
//! Fewer context switches suggest better focus.

use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ScoreFactor, SessionContext};

/// Calculates score based on file switching frequency.
///
/// Scoring logic:
/// - 1-2 files: Excellent focus (1.0)
/// - 3-5 files: Good focus (0.8)
/// - 6-10 files: Moderate switching (0.6)
/// - 11-15 files: Frequent switching (0.4)
/// - 16+ files: Scattered attention (0.2)
#[derive(Debug)]
pub struct ContextSwitchingCalculator;

impl ScoreCalculator for ContextSwitchingCalculator {
    fn name(&self) -> &str {
        "Context Switching"
    }

    fn weight(&self) -> f64 {
        0.20
    }

    fn calculate(&self, context: &SessionContext) -> ScoreFactor {
        let file_count = context.file_count;

        let (value, reason) = if file_count == 0 {
            (0.5, "No file activity recorded".to_string())
        } else if file_count <= 2 {
            (
                1.0,
                format!(
                    "Excellent focus ({} file{})",
                    file_count,
                    if file_count == 1 { "" } else { "s" }
                ),
            )
        } else if file_count <= 5 {
            (0.8, format!("Good focus ({} files)", file_count))
        } else if file_count <= 10 {
            (0.6, format!("Moderate switching ({} files)", file_count))
        } else if file_count <= 15 {
            (0.4, format!("Frequent switching ({} files)", file_count))
        } else {
            (0.2, format!("Scattered attention ({} files)", file_count))
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

    fn make_context(file_count: usize) -> SessionContext {
        SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 10,
            file_count,
            events: vec![],
        }
    }

    #[test]
    fn single_file_scores_perfect() {
        let calc = ContextSwitchingCalculator;
        let context = make_context(1);
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 1.0);
        assert!(factor.reason.contains("Excellent focus"));
    }

    #[test]
    fn few_files_scores_high() {
        let calc = ContextSwitchingCalculator;
        let context = make_context(4);
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.8);
        assert!(factor.reason.contains("Good focus"));
    }

    #[test]
    fn many_files_scores_low() {
        let calc = ContextSwitchingCalculator;
        let context = make_context(12);
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.4);
        assert!(factor.reason.contains("Frequent switching"));
    }

    #[test]
    fn scattered_attention_scores_very_low() {
        let calc = ContextSwitchingCalculator;
        let context = make_context(20);
        let factor = calc.calculate(&context);

        assert_eq!(factor.value, 0.2);
        assert!(factor.reason.contains("Scattered attention"));
    }
}
