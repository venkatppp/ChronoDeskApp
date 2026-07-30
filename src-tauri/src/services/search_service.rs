use crate::errors::DatabaseError;
use crate::models::search::{SavedSearch, SearchEntityType, SearchResult, SearchStats};
use crate::repositories::SearchRepository;
use uuid::Uuid;

/// Service for coordinating search-related operations.
#[derive(Debug, Clone)]
pub struct SearchService {
    search_repository: SearchRepository,
}

impl SearchService {
    pub fn new(search_repository: SearchRepository) -> Self {
        Self { search_repository }
    }

    pub async fn search(
        &self,
        query: &str,
        entity_types: &[SearchEntityType],
        workspace_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DatabaseError> {
        self.search_repository
            .search(query, entity_types, workspace_id, limit)
            .await
    }

    pub async fn get_search_history(&self, limit: i64) -> Result<Vec<String>, DatabaseError> {
        self.search_repository.get_search_history(limit).await
    }

    pub async fn save_search_query(&self, query: &str) -> Result<(), DatabaseError> {
        self.search_repository.save_search_query(query).await
    }

    pub async fn clear_search_history(&self) -> Result<(), DatabaseError> {
        self.search_repository.delete_search_history().await
    }

    pub async fn save_search(&self, query: &str) -> Result<SavedSearch, DatabaseError> {
        self.search_repository.save_search(query).await
    }

    pub async fn list_saved_searches(&self) -> Result<Vec<SavedSearch>, DatabaseError> {
        self.search_repository.list_saved_searches().await
    }

    pub async fn delete_saved_search(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.search_repository.delete_saved_search(id).await
    }

    pub async fn get_recent_files(
        &self,
        workspace_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SearchResult>, DatabaseError> {
        self.search_repository
            .get_recent_files(workspace_id, limit)
            .await
    }

    pub async fn get_workspace_stats(
        &self,
        workspace_id: Uuid,
    ) -> Result<SearchStats, DatabaseError> {
        self.search_repository
            .get_workspace_stats(workspace_id)
            .await
    }
}
