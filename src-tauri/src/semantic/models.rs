//! Semantic data models and types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of semantic document being indexed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDocumentType {
    Workspace,
    File,
    Session,
    ContextSnapshot,
    GraphNode,
    Recommendation,
    TimelineEvent,
    AnalyticsSummary,
}

impl SemanticDocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::File => "file",
            Self::Session => "session",
            Self::ContextSnapshot => "context_snapshot",
            Self::GraphNode => "graph_node",
            Self::Recommendation => "recommendation",
            Self::TimelineEvent => "timeline_event",
            Self::AnalyticsSummary => "analytics_summary",
        }
    }
}

/// A document in the semantic memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDocument {
    pub id: String,
    pub doc_type: SemanticDocumentType,
    pub workspace_id: Option<String>,
    pub title: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub indexed_at: DateTime<Utc>,
}

/// Result from semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchResult {
    pub document: SemanticDocument,
    pub score: f32,
    pub confidence: f32,
    pub explanation: Option<String>,
}

/// Explainable prediction with reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainablePrediction {
    pub prediction_type: String,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub explanation: String,
    pub supporting_evidence: Vec<Evidence>,
    pub source_engines: Vec<String>,
    pub related_documents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Evidence supporting a prediction or recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub source: String,
    pub description: String,
    pub confidence: f32,
    pub data: serde_json::Value,
}

/// Request to index a semantic document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexDocumentRequest {
    pub id: String,
    pub doc_type: SemanticDocumentType,
    pub workspace_id: Option<String>,
    pub title: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

/// Request to search semantic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchRequest {
    pub query: String,
    pub doc_types: Option<Vec<SemanticDocumentType>>,
    pub workspace_id: Option<String>,
    pub limit: usize,
    pub min_confidence: f32,
}

impl Default for SemanticSearchRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            doc_types: None,
            workspace_id: None,
            limit: 10,
            min_confidence: 0.5,
        }
    }
}
