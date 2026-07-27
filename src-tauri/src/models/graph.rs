use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::search::SearchEntityType;

/// Type of relationship between two entities in the knowledge graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeType {
    CoOccurrence,
    SemanticSimilarity,
    ExplicitReference,
    Derivation,
}

impl GraphEdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GraphEdgeType::CoOccurrence => "co_occurrence",
            GraphEdgeType::SemanticSimilarity => "semantic_similarity",
            GraphEdgeType::ExplicitReference => "explicit_reference",
            GraphEdgeType::Derivation => "derivation",
        }
    }
}

impl fmt::Display for GraphEdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GraphEdgeType {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "co_occurrence" => Ok(GraphEdgeType::CoOccurrence),
            "semantic_similarity" => Ok(GraphEdgeType::SemanticSimilarity),
            "explicit_reference" => Ok(GraphEdgeType::ExplicitReference),
            "derivation" => Ok(GraphEdgeType::Derivation),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown graph edge type '{other}'"
            ))),
        }
    }
}

/// An edge in the knowledge graph representing a relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: Uuid,
    pub source_entity_type: SearchEntityType,
    pub source_entity_id: Uuid,
    pub target_entity_type: SearchEntityType,
    pub target_entity_id: Uuid,
    pub edge_type: GraphEdgeType,
    /// Strength of the relationship (0.0 to 1.0).
    pub weight: f64,
    pub workspace_id: Uuid,
    /// Optional JSON metadata (e.g., specific context for the relationship).
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A node in the knowledge graph, usually derived from a search result or file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub entity_type: SearchEntityType,
    pub entity_id: Uuid,
    pub title: String,
    pub workspace_id: Uuid,
}

#[derive(Debug, FromRow)]
pub(crate) struct GraphEdgeRow {
    pub id: Uuid,
    pub source_entity_type: String,
    pub source_entity_id: Uuid,
    pub target_entity_type: String,
    pub target_entity_id: Uuid,
    pub edge_type: String,
    pub weight: f64,
    pub workspace_id: Uuid,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<GraphEdgeRow> for GraphEdge {
    type Error = DatabaseError;

    fn try_from(row: GraphEdgeRow) -> Result<Self, Self::Error> {
        Ok(GraphEdge {
            id: row.id,
            source_entity_type: SearchEntityType::from_str(&row.source_entity_type)?,
            source_entity_id: row.source_entity_id,
            target_entity_type: SearchEntityType::from_str(&row.target_entity_type)?,
            target_entity_id: row.target_entity_id,
            edge_type: GraphEdgeType::from_str(&row.edge_type)?,
            weight: row.weight,
            workspace_id: row.workspace_id,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Summary statistics for a knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStats {
    pub node_count: i64,
    pub edge_count: i64,
    pub avg_weight: f64,
    pub max_weight: f64,
    pub density: f64,
}

/// View of a graph section (nodes + edges).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphView {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// Detailed information about a node including its neighborhood.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDetails {
    pub node: GraphNode,
    pub related_edges: Vec<GraphEdge>,
}
