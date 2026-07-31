//! Context Memory Service - enriched business logic layer.

use crate::context_memory::engine::ContextMemoryEngine;
use crate::context_memory::models::{
    ContextSnapshot, CreateSnapshotRequest, KnowledgeQuery, KnowledgeSearchResult,
    RelatedWorkspace, SnapshotType,
};
use crate::errors::DatabaseError;
use uuid::Uuid;

/// Service layer for context memory operations.
///
/// Provides higher-level business logic on top of the engine,
/// including automatic snapshot triggers and enrichment.
#[derive(Clone)]
pub struct ContextMemoryService {
    engine: ContextMemoryEngine,
}

impl ContextMemoryService {
    pub fn new(engine: ContextMemoryEngine) -> Self {
        Self { engine }
    }

    /// Creates a snapshot with automatic metadata enrichment.
    pub async fn create_snapshot(
        &self,
        request: CreateSnapshotRequest,
    ) -> Result<ContextSnapshot, DatabaseError> {
        self.engine.create_snapshot(request).await
    }

    /// Gets recent snapshots for a workspace.
    pub async fn get_workspace_snapshots(
        &self,
        workspace_id: &str,
        limit: usize,
    ) -> Result<Vec<ContextSnapshot>, DatabaseError> {
        self.engine.get_workspace_snapshots(workspace_id, limit).await
    }

    /// Gets the latest snapshot.
    pub async fn get_latest_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<Option<ContextSnapshot>, DatabaseError> {
        self.engine.get_latest_snapshot(workspace_id).await
    }

    /// Triggers relationship detection for a workspace.
    pub async fn detect_relationships(&self, workspace_id: &str) -> Result<(), DatabaseError> {
        self.engine
            .detect_workspace_relationships(workspace_id)
            .await
    }

    /// Gets related workspaces.
    pub async fn get_related_workspaces(
        &self,
        workspace_id: &str,
        min_strength: f64,
        limit: usize,
    ) -> Result<Vec<RelatedWorkspace>, DatabaseError> {
        self.engine
            .get_related_workspaces(workspace_id, min_strength, limit)
            .await
    }

    /// Executes a knowledge search.
    pub async fn search_knowledge(
        &self,
        query: KnowledgeQuery,
    ) -> Result<KnowledgeSearchResult, DatabaseError> {
        self.engine.search_knowledge(query).await
    }

    /// Creates a milestone snapshot.
    pub async fn snapshot_milestone(
        &self,
        workspace_id: &str,
        active_files: Vec<String>,
        metadata: serde_json::Value,
    ) -> Result<ContextSnapshot, DatabaseError> {
        self.engine
            .snapshot_milestone(workspace_id, active_files, metadata)
            .await
    }

    /// Auto-generates a snapshot on workspace switch or inactivity.
    pub async fn auto_snapshot(
        &self,
        workspace_id: Uuid,
        active_files: Vec<String>,
    ) -> Result<ContextSnapshot, DatabaseError> {
        let request = CreateSnapshotRequest {
            workspace_id: workspace_id.to_string(),
            snapshot_type: SnapshotType::Auto,
            active_files,
            session_summary: None,
            timeline_references: None,
            analytics_summary: None,
            health_score: None,
            recommendations_summary: None,
            metadata: Some(serde_json::json!({
                "trigger": "auto",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        };

        self.engine.create_snapshot(request).await
    }
}
