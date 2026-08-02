//! Knowledge Graph Foundation models (RC-8 M1).
//!
//! The RC-8 knowledge graph is a typed node registry
//! ([`KgNode`]) plus a relationship table ([`KgEdge`]) built
//! automatically from six source aggregates — workspaces, files, planner
//! reports, executions, memory records, and autonomous sessions. These
//! types are additive to the Phase 4 `models::graph` types (which keep
//! serving the legacy `graph_edges` adjacency commands); the two graphs
//! coexist.
//!
//! Every enum-bearing model follows the codebase's `*Row` + `TryFrom`
//! decoding pattern (SQLite has no enums — raw `String` columns are
//! decoded first, then fallibly converted).

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::errors::DatabaseError;

/// The six aggregate kinds a knowledge graph node can represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeType {
    Workspace,
    File,
    PlannerReport,
    Execution,
    MemoryRecord,
    AutonomousSession,
}

impl GraphNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GraphNodeType::Workspace => "workspace",
            GraphNodeType::File => "file",
            GraphNodeType::PlannerReport => "planner_report",
            GraphNodeType::Execution => "execution",
            GraphNodeType::MemoryRecord => "memory_record",
            GraphNodeType::AutonomousSession => "autonomous_session",
        }
    }
}

impl fmt::Display for GraphNodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GraphNodeType {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "workspace" => Ok(GraphNodeType::Workspace),
            "file" => Ok(GraphNodeType::File),
            "planner_report" => Ok(GraphNodeType::PlannerReport),
            "execution" => Ok(GraphNodeType::Execution),
            "memory_record" => Ok(GraphNodeType::MemoryRecord),
            "autonomous_session" => Ok(GraphNodeType::AutonomousSession),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown graph node type '{other}'"
            ))),
        }
    }
}

/// The relationship vocabulary of the RC-8 knowledge graph. Intentionally
/// small and structural — computed/co-occurrence edges are the legacy
/// `graph_edges` table's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphRelationshipType {
    /// `workspace -> file`: a file belongs to a workspace.
    Contains,
    /// `execution | memory_record | autonomous_session -> workspace`.
    RunsIn,
    /// `planner_report -> execution`: the report summarizes that run.
    ReportsOn,
    /// `memory_record -> execution`: the record was learned from that run.
    DerivedFrom,
    /// Computed tie recorded during context discovery.
    RelatedTo,
}

impl GraphRelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GraphRelationshipType::Contains => "contains",
            GraphRelationshipType::RunsIn => "runs_in",
            GraphRelationshipType::ReportsOn => "reports_on",
            GraphRelationshipType::DerivedFrom => "derived_from",
            GraphRelationshipType::RelatedTo => "related_to",
        }
    }
}

impl fmt::Display for GraphRelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GraphRelationshipType {
    type Err = DatabaseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "contains" => Ok(GraphRelationshipType::Contains),
            "runs_in" => Ok(GraphRelationshipType::RunsIn),
            "reports_on" => Ok(GraphRelationshipType::ReportsOn),
            "derived_from" => Ok(GraphRelationshipType::DerivedFrom),
            "related_to" => Ok(GraphRelationshipType::RelatedTo),
            other => Err(DatabaseError::InvalidInput(format!(
                "unknown graph relationship type '{other}'"
            ))),
        }
    }
}

/// A node in the RC-8 knowledge graph — one row in `graph_nodes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KgNode {
    pub node_type: GraphNodeType,
    pub entity_id: Uuid,
    pub title: String,
    pub workspace_id: Option<Uuid>,
    pub summary: Option<String>,
    /// Free-form JSON metadata (status, kind, evidence, ...).
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(crate) struct KgNodeRow {
    pub node_type: String,
    pub entity_id: Uuid,
    pub title: String,
    pub workspace_id: Option<Uuid>,
    pub summary: Option<String>,
    pub metadata: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<KgNodeRow> for KgNode {
    type Error = DatabaseError;

    fn try_from(row: KgNodeRow) -> Result<Self, Self::Error> {
        Ok(KgNode {
            node_type: GraphNodeType::from_str(&row.node_type)?,
            entity_id: row.entity_id,
            title: row.title,
            workspace_id: row.workspace_id,
            summary: row.summary,
            metadata: serde_json::from_str(&row.metadata)
                .unwrap_or(serde_json::Value::Object(Default::default())),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// An edge between two knowledge graph nodes — one row in
/// `graph_relationships`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KgEdge {
    pub id: Uuid,
    pub source_node_type: GraphNodeType,
    pub source_entity_id: Uuid,
    pub target_node_type: GraphNodeType,
    pub target_entity_id: Uuid,
    pub relationship_type: GraphRelationshipType,
    /// Strength of the relationship (0.0 to 1.0).
    pub weight: f64,
    /// Free-form JSON evidence attached at construction time.
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub(crate) struct KgEdgeRow {
    pub id: Uuid,
    pub source_node_type: String,
    pub source_entity_id: Uuid,
    pub target_node_type: String,
    pub target_entity_id: Uuid,
    pub relationship_type: String,
    pub weight: f64,
    pub metadata: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<KgEdgeRow> for KgEdge {
    type Error = DatabaseError;

    fn try_from(row: KgEdgeRow) -> Result<Self, Self::Error> {
        Ok(KgEdge {
            id: row.id,
            source_node_type: GraphNodeType::from_str(&row.source_node_type)?,
            source_entity_id: row.source_entity_id,
            target_node_type: GraphNodeType::from_str(&row.target_node_type)?,
            target_entity_id: row.target_entity_id,
            relationship_type: GraphRelationshipType::from_str(&row.relationship_type)?,
            weight: row.weight,
            metadata: serde_json::from_str(&row.metadata)
                .unwrap_or(serde_json::Value::Object(Default::default())),
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// A section of the knowledge graph (BFS subgraph): every node reached
/// from a root plus every relationship connecting the returned nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KgSubgraph {
    pub root: KgNode,
    pub nodes: Vec<KgNode>,
    pub edges: Vec<KgEdge>,
}

/// A shortest path between two nodes: alternating node/edge sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPath {
    pub nodes: Vec<KgNode>,
    pub edges: Vec<KgEdge>,
}

/// One related entity returned by context relationship discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextHit {
    pub node: KgNode,
    /// The persisted relationship linking it to the source, when one
    /// exists (computed hits have `None`).
    pub relationship_type: Option<GraphRelationshipType>,
    /// Human-readable explanation of the relationship.
    pub reason: String,
    /// Ranking strength (0.0 to 1.0), sorted descending.
    pub weight: f64,
}

/// Result of context relationship discovery for one entity: the source
/// node plus its ranked neighborhood of related context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDiscovery {
    pub source: KgNode,
    pub related: Vec<ContextHit>,
}

/// Accounting for one full graph construction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSyncSummary {
    pub created_nodes: u64,
    pub updated_nodes: u64,
    pub created_edges: u64,
    pub updated_edges: u64,
    pub total_nodes: u64,
    pub total_edges: u64,
}

/// Per-type counts for the graph statistics panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeCount {
    pub name: String,
    pub count: i64,
}

/// Aggregate statistics for the RC-8 knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KgStats {
    pub node_count: i64,
    pub edge_count: i64,
    pub nodes_by_type: Vec<TypeCount>,
    pub edges_by_type: Vec<TypeCount>,
}

/// Raw material for one graph node, extracted by the repository from its
/// source aggregate and handed to the engine's construction pass.
#[derive(Debug, Clone)]
pub struct GraphSource {
    pub entity_id: Uuid,
    pub title: String,
    pub workspace_id: Option<Uuid>,
    pub summary: Option<String>,
    pub metadata: serde_json::Value,
}
