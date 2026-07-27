//! Search Engine (blueprint §4.2, §9).
//!
//! Provides a facade for full-text search and related services.
//! Wraps [`SearchService`] to coordinate search operations.

use uuid::Uuid;
use crate::errors::DatabaseError;
use crate::models::search::{SearchResult, SearchEntityType, SearchStats};
use crate::services::SearchService;

/// Facade for search operations.
#[derive(Debug, Clone)]
pub struct SearchEngine {
    search_service: SearchService,
}

impl SearchEngine {
    pub fn new(search_service: SearchService) -> Self {
        Self { search_service }
    }

    /// Performs a search across indexed entities.
    pub async fn search(
        &self,
        query: &str,
        entity_types: Option<Vec<SearchEntityType>>,
        workspace_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DatabaseError> {
        let types = entity_types.unwrap_or_default();
        self.search_service.search(query, &types, workspace_id, limit).await
    }

    /// Fetches recently updated files for a workspace.
    pub async fn get_recent_files(
        &self,
        workspace_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DatabaseError> {
        self.search_service.get_recent_files(workspace_id, limit).await
    }

    /// Returns search statistics for a workspace.
    pub async fn get_workspace_stats(
        &self,
        workspace_id: Uuid,
    ) -> Result<SearchStats, DatabaseError> {
        self.search_service.get_workspace_stats(workspace_id).await
    }

    /// Provides auto-complete suggestions based on search history, filtered
    /// to entries containing `query` (case-insensitive prefix/substring
    /// match) since [`crate::services::SearchService::get_search_history`]
    /// only returns the most recent entries unfiltered.
    pub async fn get_suggestions(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<String>, DatabaseError> {
        let history = self.search_service.get_search_history(limit).await?;
        let trimmed = query.trim().to_lowercase();
        if trimmed.is_empty() {
            return Ok(history);
        }
        Ok(history
            .into_iter()
            .filter(|h| h.to_lowercase().contains(&trimmed))
            .collect())
    }
}
