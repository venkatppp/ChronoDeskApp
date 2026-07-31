//! Recommendation generators - create candidate recommendations.

mod activity;
mod context;
mod organization;

pub use activity::ActivityRecommendationGenerator;
pub use context::ContextRecommendationGenerator;
pub use organization::OrganizationRecommendationGenerator;

use crate::errors::DatabaseError;
use crate::intelligence::recommendation::models::Recommendation;

/// Trait for recommendation generators.
#[async_trait::async_trait]
pub trait RecommendationGenerator: Send + Sync {
    /// Generates recommendations for a workspace.
    async fn generate(&self, workspace_id: i64) -> Result<Vec<Recommendation>, DatabaseError>;
}
