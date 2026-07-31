//! Workspace Consistency Calculator
//!
//! Measures whether all activity stayed within a single workspace.
//! Single-workspace sessions indicate focused work; multiple workspaces
//! suggest context switching.

use crate::session::scoring::ScoreCalculator;
use crate::session::types::{ScoreFactor, SessionContext};
use std::collections::HashSet;
use uuid::Uuid;

/// Calculates score based on workspace consistency.
///
/// Scoring logic:
/// - Single workspace: 1.0 (focused)
/// - Multiple workspaces: 0.3 (scattered - should not happen in practice
///   since sessions are already grouped by workspace)
#[derive(Debug)]
pub struct WorkspaceConsistencyCalculator;

impl ScoreCalculator for WorkspaceConsistencyCalculator {
    fn name(&self) -> &str {
        "Workspace Consistency"
    }

    fn weight(&self) -> f64 {
        0.15
    }

    fn calculate(&self, context: &SessionContext) -> ScoreFactor {
        // In practice, sessions are already grouped by workspace,
        // so this will almost always score 1.0. However, this calculator
        // is included for completeness and future extensibility.
        let workspaces: HashSet<Uuid> = context.events.iter().map(|e| e.workspace_id).collect();

        let (value, reason) = if workspaces.len() <= 1 {
            (1.0, "Single workspace focus".to_string())
        } else {
            (
                0.3,
                format!("Scattered across {} workspaces", workspaces.len()),
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
    use crate::models::TimelineEventType;
    use chrono::Utc;

    #[test]
    fn single_workspace_scores_perfect() {
        let calc = WorkspaceConsistencyCalculator;
        let workspace_id = Uuid::new_v4();

        let events = vec![
            TimelineEvent {
                id: Uuid::new_v4(),
                workspace_id,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: None,
                created_at: Utc::now(),
            },
            TimelineEvent {
                id: Uuid::new_v4(),
                workspace_id,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: None,
                created_at: Utc::now(),
            },
        ];

        let context = SessionContext {
            workspace_id,
            duration_seconds: 3600,
            event_count: events.len(),
            file_count: 2,
            events,
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 1.0);
        assert!(factor.reason.contains("Single workspace"));
    }

    #[test]
    fn multiple_workspaces_scores_low() {
        let calc = WorkspaceConsistencyCalculator;
        let workspace1 = Uuid::new_v4();
        let workspace2 = Uuid::new_v4();

        let events = vec![
            TimelineEvent {
                id: Uuid::new_v4(),
                workspace_id: workspace1,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: None,
                created_at: Utc::now(),
            },
            TimelineEvent {
                id: Uuid::new_v4(),
                workspace_id: workspace2,
                file_id: Some(Uuid::new_v4()),
                event_type: TimelineEventType::Edit,
                occurred_at: Utc::now(),
                metadata: None,
                created_at: Utc::now(),
            },
        ];

        let context = SessionContext {
            workspace_id: workspace1,
            duration_seconds: 3600,
            event_count: events.len(),
            file_count: 2,
            events,
        };

        let factor = calc.calculate(&context);
        assert_eq!(factor.value, 0.3);
        assert!(factor.reason.contains("Scattered"));
    }
}
