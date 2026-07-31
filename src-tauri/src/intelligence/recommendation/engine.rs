//! Main recommendation engine.

use crate::errors::DatabaseError;
use crate::repositories::{FileRepository, WorkspaceRepository};
use crate::services::ContextService;

use super::generators::{
    ActivityRecommendationGenerator, ContextRecommendationGenerator,
    OrganizationRecommendationGenerator, RecommendationGenerator,
};
use super::models::Recommendation;
use super::scoring::RecommendationScoringEngine;

/// Main recommendation engine that coordinates generators and scoring.
#[derive(Clone)]
pub struct RecommendationEngine {
    workspace_repository: WorkspaceRepository,
    file_repository: FileRepository,
    context_service: ContextService,
    scoring_engine: RecommendationScoringEngine,
}

impl RecommendationEngine {
    /// Creates a new recommendation engine.
    pub fn new(
        workspace_repository: WorkspaceRepository,
        file_repository: FileRepository,
        context_service: ContextService,
    ) -> Self {
        Self {
            workspace_repository,
            file_repository,
            context_service,
            scoring_engine: RecommendationScoringEngine::new(),
        }
    }

    /// Generates all recommendations for a workspace.
    pub async fn generate_recommendations(
        &self,
        workspace_id: i64,
    ) -> Result<Vec<Recommendation>, DatabaseError> {
        // Create generators
        let activity_gen = ActivityRecommendationGenerator::new(self.workspace_repository.clone());
        let context_gen = ContextRecommendationGenerator::new(self.context_service.clone());
        let organization_gen = OrganizationRecommendationGenerator::new(
            self.workspace_repository.clone(),
            self.file_repository.clone(),
        );

        // Generate recommendations from all generators
        let mut all_recommendations = Vec::new();

        // Activity-based recommendations
        let activity_recs = activity_gen.generate(workspace_id).await?;
        all_recommendations.extend(activity_recs);

        // Context-based recommendations
        let context_recs = context_gen.generate(workspace_id).await?;
        all_recommendations.extend(context_recs);

        // Organization-based recommendations
        let org_recs = organization_gen.generate(workspace_id).await?;
        all_recommendations.extend(org_recs);

        // Score and rank all recommendations
        let scored = self.scoring_engine.score_and_rank(all_recommendations);

        // Filter expired recommendations
        let filtered = self.scoring_engine.filter_expired(scored);

        // Limit to top 10 recommendations
        let top_recommendations = self.scoring_engine.limit_top_n(filtered, 10);

        Ok(top_recommendations)
    }

    /// Generates recommendations for a specific category.
    pub async fn generate_category_recommendations(
        &self,
        workspace_id: i64,
        category: super::models::RecommendationCategory,
    ) -> Result<Vec<Recommendation>, DatabaseError> {
        // Generate all and filter by category
        let all_recommendations = self.generate_recommendations(workspace_id).await?;

        // Filter by category
        let filtered: Vec<_> = all_recommendations
            .into_iter()
            .filter(|rec| rec.category == category)
            .collect();

        Ok(filtered)
    }

    /// Generates top priority recommendations only.
    pub async fn generate_priority_recommendations(
        &self,
        workspace_id: i64,
        min_priority: super::models::RecommendationPriority,
    ) -> Result<Vec<Recommendation>, DatabaseError> {
        let all_recommendations = self.generate_recommendations(workspace_id).await?;

        let filtered: Vec<_> = all_recommendations
            .into_iter()
            .filter(|rec| rec.priority >= min_priority)
            .collect();

        Ok(filtered)
    }
}
