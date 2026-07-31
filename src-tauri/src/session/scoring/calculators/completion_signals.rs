//! Completion Signals Calculator
//!
//! Measures signals that indicate work completion or progress:
//! commits, builds, and other milestone events.

use crate::models::TimelineEventType;
use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ScoreFactor, SessionContext};

/// Calculates score based on completion signals (commits, builds).
///
/// Scoring logic:
/// - No signals: 0.4 (work in progress)
/// - 1 signal: 0.7 (some completion)
/// - 2+ signals: 1.0 (clear progress)
#[derive(Debug)]
pub struct CompletionSignalsCalculator;

impl ScoreCalculator for CompletionSignalsCalculator {
    fn name(&self) -> &str {
        "Completion Signals"
    }

    fn weight(&self) -> f64 {
        0.20
    }

    fn calculate(&self, context: &SessionContext) -> ScoreFactor {
        let commit_count = context
            .events
            .iter()
            .filter(|e| e.event_type == TimelineEventType::Commit)
            .count();

        // Future: could also detect build events from metadata
        let total_signals = commit_count;

        let (value, reason) = if total_signals == 0 {
            (0.4, "No completion signals (work in progress)".to_string())
        } else if total_signals == 1 {
            (0.7, format!("1 commit (some completion)"))
        } else {
            (1.0, format!("{} commits (clear progress)", total_signals))
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
    use uuid::Uuid;

    fn make_event(event_type: TimelineEventType) -> TimelineEvent {
        TimelineEvent {
            id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            file_id: Some(Uuid::new_v4()),
            event_type,
            occurred_at: Utc::now(),
            metadata: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn no_commits_scores_medium() {
        let calc = CompletionSignalsCalculator;
        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 5,
            file_count: 2,
            events: vec![
                make_event(TimelineEventType::Edit),
                make_event(TimelineEventType::Edit),
            ],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 0.4);
        assert!(factor.reason.contains("No completion signals"));
    }

    #[test]
    fn one_commit_scores_high() {
        let calc = CompletionSignalsCalculator;
        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 5,
            file_count: 2,
            events: vec![
                make_event(TimelineEventType::Edit),
                make_event(TimelineEventType::Commit),
            ],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 0.7);
        assert!(factor.reason.contains("1 commit"));
    }

    #[test]
    fn multiple_commits_scores_perfect() {
        let calc = CompletionSignalsCalculator;
        let context = SessionContext {
            workspace_id: Uuid::new_v4(),
            duration_seconds: 3600,
            event_count: 5,
            file_count: 2,
            events: vec![
                make_event(TimelineEventType::Edit),
                make_event(TimelineEventType::Commit),
                make_event(TimelineEventType::Edit),
                make_event(TimelineEventType::Commit),
            ],
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 1.0);
        assert!(factor.reason.contains("commits"));
        assert!(factor.reason.contains("clear progress"));
    }
}
