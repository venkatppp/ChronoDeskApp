//! Deep Editing Calculator
//!
//! Measures how deeply the user engaged with files by counting repeated
//! edits to the same files. Multiple edits to the same file suggest
//! focused, iterative work rather than superficial scanning.

use crate::models::TimelineEventType;
use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ScoreFactor, SessionContext};
use std::collections::HashMap;
use uuid::Uuid;

/// Calculates score based on edit depth (repeated edits per file).
///
/// Scoring logic:
/// - No edits: 0.0
/// - 1 edit per file average: 0.4 (superficial)
/// - 2-3 edits per file average: 0.7 (engaged)
/// - 4+ edits per file average: 1.0 (deep work)
#[derive(Debug)]
pub struct DeepEditingCalculator;

impl ScoreCalculator for DeepEditingCalculator {
    fn name(&self) -> &str {
        "Deep Editing"
    }

    fn weight(&self) -> f64 {
        0.20
    }

    fn calculate(&self, context: &SessionContext) -> ScoreFactor {
        // Count edit events per file
        let mut edits_per_file: HashMap<Uuid, usize> = HashMap::new();

        for event in &context.events {
            if matches!(
                event.event_type,
                TimelineEventType::Edit | TimelineEventType::Create
            ) {
                if let Some(file_id) = event.file_id {
                    *edits_per_file.entry(file_id).or_insert(0) += 1;
                }
            }
        }

        if edits_per_file.is_empty() {
            return ScoreFactor {
                name: self.name().to_string(),
                weight: self.weight(),
                value: 0.0,
                reason: "No file edits recorded".to_string(),
            };
        }

        let total_edits: usize = edits_per_file.values().sum();
        let file_count = edits_per_file.len();
        let avg_edits_per_file = total_edits as f64 / file_count as f64;

        let (value, reason) = if avg_edits_per_file < 1.5 {
            (
                0.4,
                format!(
                    "Superficial editing ({:.1} edits/file across {} files)",
                    avg_edits_per_file, file_count
                ),
            )
        } else if avg_edits_per_file < 3.5 {
            (
                0.7,
                format!(
                    "Engaged editing ({:.1} edits/file across {} files)",
                    avg_edits_per_file, file_count
                ),
            )
        } else {
            (
                1.0,
                format!(
                    "Deep work ({:.1} edits/file across {} files)",
                    avg_edits_per_file, file_count
                ),
            )
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
    use crate::models::TimelineEvent;
    use chrono::Utc;

    fn make_event(file_id: Option<Uuid>, event_type: TimelineEventType) -> TimelineEvent {
        TimelineEvent {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            file_id,
            event_type,
            occurred_at: Utc::now(),
            metadata: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn no_edits_scores_zero() {
        let calc = DeepEditingCalculator;
        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 0,
            file_count: 0,
            events: vec![],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 0.0);
        assert!(factor.reason.contains("No file edits"));
    }

    #[test]
    fn single_edit_per_file_scores_low() {
        let calc = DeepEditingCalculator;
        let file1 = Uuid::new_v4();
        let file2 = Uuid::new_v4();

        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 2,
            file_count: 2,
            events: vec![
                make_event(Some(file1), TimelineEventType::Edit),
                make_event(Some(file2), TimelineEventType::Edit),
            ],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 0.4);
        assert!(factor.reason.contains("Superficial"));
    }

    #[test]
    fn multiple_edits_per_file_scores_high() {
        let calc = DeepEditingCalculator;
        let file1 = Uuid::new_v4();

        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 5,
            file_count: 1,
            events: vec![
                make_event(Some(file1), TimelineEventType::Edit),
                make_event(Some(file1), TimelineEventType::Edit),
                make_event(Some(file1), TimelineEventType::Edit),
                make_event(Some(file1), TimelineEventType::Edit),
                make_event(Some(file1), TimelineEventType::Edit),
            ],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 1.0);
        assert!(factor.reason.contains("Deep work"));
    }
}
