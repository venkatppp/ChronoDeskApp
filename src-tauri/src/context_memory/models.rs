//! Context Memory models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of context snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    /// Manually triggered by user.
    Manual,

    /// Automatically triggered at milestone (commit, session end, etc).
    Milestone,

    /// Automatically triggered periodically.
    Auto,
}

impl SnapshotType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotType::Manual => "manual",
            SnapshotType::Milestone => "milestone",
            SnapshotType::Auto => "auto",
        }
    }
}

/// A snapshot of workspace context at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    pub id: i64,
    pub workspace_id: String,
    pub snapshot_type: SnapshotType,
    pub captured_at: DateTime<Utc>,
    pub active_files: Vec<String>,
    pub session_summary: Option<serde_json::Value>,
    pub timeline_references: Option<Vec<i64>>,
    pub analytics_summary: Option<serde_json::Value>,
    pub health_score: Option<f64>,
    pub recommendations_summary: Option<Vec<String>>,
    pub metadata: serde_json::Value,
}

/// Request to create a context snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotRequest {
    pub workspace_id: String,
    pub snapshot_type: SnapshotType,
    pub active_files: Vec<String>,
    pub session_summary: Option<serde_json::Value>,
    pub timeline_references: Option<Vec<i64>>,
    pub analytics_summary: Option<serde_json::Value>,
    pub health_score: Option<f64>,
    pub recommendations_summary: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// Type of workspace relationship.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceRelationshipType {
    /// Workspaces share common files.
    SharedFiles,

    /// Workspaces share common folders.
    SharedFolders,

    /// Workspaces use same technologies/languages.
    SharedTech,

    /// Workspaces have similar editing patterns.
    SimilarPatterns,
}

impl WorkspaceRelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkspaceRelationshipType::SharedFiles => "shared_files",
            WorkspaceRelationshipType::SharedFolders => "shared_folders",
            WorkspaceRelationshipType::SharedTech => "shared_tech",
            WorkspaceRelationshipType::SimilarPatterns => "similar_patterns",
        }
    }
}

/// A relationship between two workspaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRelationship {
    pub id: i64,
    pub source_workspace_id: String,
    pub target_workspace_id: String,
    pub relationship_type: WorkspaceRelationshipType,
    pub strength: f64,
    pub evidence: serde_json::Value,
    pub detected_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Related workspace with enriched metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWorkspace {
    pub workspace_id: String,
    pub workspace_name: String,
    pub relationship_type: WorkspaceRelationshipType,
    pub strength: f64,
    pub evidence: serde_json::Value,
    pub last_active_at: DateTime<Utc>,
}

/// Knowledge search query types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum KnowledgeQuery {
    RelatedWorkspaces { workspace_id: String },
    RelatedFiles { file_path: String },
    RecentContext { workspace_id: String, limit: usize },
    PreviousSessions { workspace_id: String, limit: usize },
    SimilarProjects { workspace_id: String },
}

/// Knowledge search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResult {
    pub query_type: String,
    pub results: serde_json::Value,
    pub total_count: usize,
}
