//! Organization-based recommendation generator.

use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::intelligence::recommendation::models::{
    Recommendation, RecommendationAction, RecommendationCategory,
};
use crate::repositories::{FileRepository, WorkspaceRepository};

use super::RecommendationGenerator;

/// Generates recommendations based on workspace organization.
pub struct OrganizationRecommendationGenerator {
    workspace_repository: WorkspaceRepository,
    #[allow(dead_code)]
    file_repository: FileRepository,
}

impl OrganizationRecommendationGenerator {
    /// Creates a new organization recommendation generator.
    pub fn new(workspace_repository: WorkspaceRepository, file_repository: FileRepository) -> Self {
        Self {
            workspace_repository,
            file_repository,
        }
    }

    /// Converts i64 workspace_id to Uuid (temporary helper for Phase 5C)
    fn id_to_uuid(&self, workspace_id: i64) -> Uuid {
        let bytes = workspace_id.to_le_bytes();
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes[..8].copy_from_slice(&bytes);
        Uuid::from_bytes(uuid_bytes)
    }
}

#[async_trait::async_trait]
impl RecommendationGenerator for OrganizationRecommendationGenerator {
    async fn generate(&self, workspace_id: i64) -> Result<Vec<Recommendation>, DatabaseError> {
        let mut recommendations = Vec::new();

        // Convert to UUID for repository access
        let workspace_uuid = self.id_to_uuid(workspace_id);

        // Get workspace stats
        let stats = match self
            .workspace_repository
            .get_workspace_stats(workspace_uuid)
            .await
        {
            Ok(stats) => stats,
            Err(_) => return Ok(recommendations), // Workspace not found, return empty
        };

        let file_count = stats.file_count;

        // Check for large workspaces
        if file_count > 100 {
            recommendations.push(
                Recommendation::new(
                    workspace_id,
                    RecommendationCategory::Organization,
                    "Large workspace detected",
                    format!(
                        "This workspace has {} files. Consider organizing into subdirectories.",
                        file_count
                    ),
                )
                .with_confidence(0.75)
                .with_impact(0.6)
                .with_effort(0.7),
            );
        }

        // Check for medium workspaces that might benefit from organization
        if file_count > 50 && file_count <= 100 {
            recommendations.push(
                Recommendation::new(
                    workspace_id,
                    RecommendationCategory::Organization,
                    "Growing workspace",
                    format!(
                        "{} files in workspace. Good time to organize before it gets too large.",
                        file_count
                    ),
                )
                .with_confidence(0.65)
                .with_impact(0.5)
                .with_effort(0.4),
            );
        }

        // Recommend duplicate scan for larger workspaces
        if file_count > 50 {
            recommendations.push(
                Recommendation::new(
                    workspace_id,
                    RecommendationCategory::Files,
                    "Scan for duplicate files",
                    "Run a duplicate scan to identify and clean up redundant files.",
                )
                .with_confidence(0.6)
                .with_impact(0.4)
                .with_effort(0.2)
                .with_action(RecommendationAction::ExecuteCommand {
                    command: "scan_duplicates".to_string(),
                    args: vec![workspace_uuid.to_string()],
                }),
            );
        }

        // Check for very small workspaces that might need consolidation
        if file_count > 0 && file_count < 5 && stats.timeline_event_count < 10 {
            recommendations.push(
                Recommendation::new(
                    workspace_id,
                    RecommendationCategory::Organization,
                    "Small workspace with low activity",
                    format!(
                        "Only {} files. Consider merging with related workspaces.",
                        file_count
                    ),
                )
                .with_confidence(0.7)
                .with_impact(0.4)
                .with_effort(0.3),
            );
        }

        Ok(recommendations)
    }
}
