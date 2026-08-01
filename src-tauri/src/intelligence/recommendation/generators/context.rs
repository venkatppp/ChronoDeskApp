//! Context-based recommendation generator.

use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::intelligence::recommendation::models::{
    Recommendation, RecommendationAction, RecommendationCategory,
};
use crate::services::ContextService;

use super::RecommendationGenerator;

/// Generates recommendations based on context quality and session patterns.
pub struct ContextRecommendationGenerator {
    context_service: ContextService,
}

impl ContextRecommendationGenerator {
    /// Creates a new context recommendation generator.
    pub fn new(context_service: ContextService) -> Self {
        Self { context_service }
    }
}

#[async_trait::async_trait]
impl RecommendationGenerator for ContextRecommendationGenerator {
    async fn generate(&self, _workspace_id: Uuid) -> Result<Vec<Recommendation>, DatabaseError> {
        let mut recommendations = Vec::new();

        // Get smart resume session (no workspace_id parameter)
        if let Some(session) = self.context_service.get_smart_resume_session().await? {
            // Short session detected
            if session.duration_seconds < 600 {
                recommendations.push(
                    Recommendation::new(
                        session.workspace_id.to_string(),
                        RecommendationCategory::Context,
                        "Short session detected",
                        "Your last session was brief. Use Smart Resume to quickly restore your context."
                    )
                    .with_confidence(0.75)
                    .with_impact(0.7)
                    .with_effort(0.1)
                    .with_action(RecommendationAction::ExecuteCommand {
                        command: "smart_resume".to_string(),
                        args: vec![],
                    })
                );
            }

            // Many files in session
            if session.file_count > 15 {
                recommendations.push(
                    Recommendation::new(
                        session.workspace_id.to_string(),
                        RecommendationCategory::Organization,
                        "Many files in recent session",
                        format!(
                            "You worked with {} files recently. Consider organizing into focused groups.",
                            session.file_count
                        )
                    )
                    .with_confidence(0.7)
                    .with_impact(0.5)
                    .with_effort(0.4)
                    .with_action(RecommendationAction::OpenView {
                        view: "files".to_string(),
                    })
                );
            }

            // Good focused session
            if session.duration_seconds > 3600 && session.file_count <= 10 {
                recommendations.push(
                    Recommendation::new(
                        session.workspace_id.to_string(),
                        RecommendationCategory::Productivity,
                        "Strong focus session detected",
                        "You had a well-focused session. Great work!",
                    )
                    .with_confidence(0.85)
                    .with_impact(0.3)
                    .with_effort(0.0),
                );
            }
        }

        Ok(recommendations)
    }
}
