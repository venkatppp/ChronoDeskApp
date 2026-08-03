//! Live Knowledge Graph models (RC-8 M2).
//!
//! Additive to the M1 `models::kg` types: analytics payloads (degree
//! distribution, centrality, components, workspace importance),
//! incremental-sync and semantic-edge accounting, multi-hop context
//! expansion, recommendation results, and the relationship inspector
//! payload. Everything here is a plain serializable DTO — the SQL that
//! backs them lives in `repositories::KgLiveRepository`, the scoring in
//! `services::KgLiveService`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::kg::{GraphNodeType, GraphRelationshipType, KgEdge, KgNode};

/// One structural link for a single entity, extracted by the repository
/// during incremental/entity sync (`RC-8 M2`). The endpoint pair always
/// refers to graph node keys; metadata is attached by the caller from
/// [`crate::models::kg::structural_edge_metadata`].
#[derive(Debug, Clone)]
pub struct StructuralLink {
    pub source_type: GraphNodeType,
    pub source_id: Uuid,
    pub target_type: GraphNodeType,
    pub target_id: Uuid,
    pub relationship_type: GraphRelationshipType,
}

/// Accounting for one entity sync (`sync_entity`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySyncResult {
    pub node_created: bool,
    pub node_updated: bool,
    pub edges_created: u64,
    pub edges_updated: u64,
}

/// Accounting for one semantic edge build pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticEdgeResult {
    /// Node pairs whose similarity cleared the cosine threshold.
    pub candidate_pairs: usize,
    /// `related_to` edges newly persisted.
    pub created: usize,
    /// `related_to` edges refreshed (similarity updated).
    pub updated: usize,
    /// Stale `related_to` edges pruned (below the threshold).
    pub pruned: usize,
    /// Cosine threshold used for the pass.
    pub threshold: f64,
}

/// Accounting for one edge-decay maintenance pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDecaySummary {
    /// Edges whose confidence was aged down.
    pub decayed: u64,
    /// Edges pruned after decaying below the minimum confidence.
    pub pruned: u64,
    /// Confidence floor under which edges are removed.
    pub min_confidence: f64,
}

/// One semantic edge awaiting confidence decay: its stored id, its
/// current confidence, and its age in days. The repository reports the
/// SQL-expressible facts; the service applies the exponential policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayCandidate {
    pub id: Uuid,
    pub confidence: f64,
    pub age_days: f64,
}

/// One bucket of the degree distribution histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DegreeBucket {
    pub degree: u64,
    pub count: u64,
}

/// Centrality of one graph node (degree + eigenvector-style ranking).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCentrality {
    pub node_type: GraphNodeType,
    pub entity_id: Uuid,
    pub title: String,
    pub in_degree: u64,
    pub out_degree: u64,
    /// Normalized degree centrality (degree / (n - 1)).
    pub degree_centrality: f64,
    /// Power-iteration eigenvector score (ranking importance).
    pub eigenvector: f64,
}

/// One connected component of the (undirected) graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphComponent {
    /// Stable per-computation index, largest components first.
    pub index: usize,
    pub size: u64,
    /// Count of nodes per node type inside the component.
    pub node_types: Vec<TypeCount>,
    /// Sample of member titles (up to 5) for the dashboard.
    pub member_titles: Vec<String>,
}

pub use crate::models::kg::TypeCount;

/// Importance of one workspace in the whole graph (global scope).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceImportance {
    pub workspace_id: Uuid,
    pub name: String,
    /// Rank score: eigenvector mass plus weighted edge strength.
    pub importance: f64,
    pub node_count: u64,
    pub edge_count: u64,
    /// Sum of confidence-weighted edge weights touching the workspace.
    pub weight_sum: f64,
}

/// Full analytics payload for the dashboard (cached per scope).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphAnalytics {
    /// Cache scope key: `all` or a workspace id.
    pub scope: String,
    pub node_count: u64,
    pub edge_count: u64,
    pub average_degree: f64,
    pub density: f64,
    pub degree_distribution: Vec<DegreeBucket>,
    /// Top nodes by eigenvector centrality (capped at 10).
    pub top_central_nodes: Vec<NodeCentrality>,
    pub components: Vec<GraphComponent>,
    /// Workspace importance (global scope only; empty otherwise).
    pub workspace_importance: Vec<WorkspaceImportance>,
    /// True when served from the query cache.
    pub cached: bool,
    pub computed_at: DateTime<Utc>,
}

/// One multi-hop context hit: a node reached at `hop` edges from the
/// source, carrying the strongest accumulated path score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiHopHit {
    pub node: KgNode,
    pub relationship_type: Option<GraphRelationshipType>,
    pub reason: String,
    /// Accumulated path score (edge weight × confidence × hop decay).
    pub weight: f64,
    pub hop: usize,
}

/// Multi-hop context expansion around one entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiHopContext {
    pub source: KgNode,
    pub related: Vec<MultiHopHit>,
}

/// One recommendation for related work around a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRecommendation {
    pub node: KgNode,
    /// Combined score: best path score plus semantic similarity.
    pub score: f64,
    pub reason: String,
    /// Number of edges from the source to this node.
    pub hop: usize,
    /// The intermediate node on the best path (2-hop hits), if any.
    pub via: Option<KgNode>,
}

/// One relationship in the inspector: the edge plus its resolved neighbor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDetail {
    pub edge: KgEdge,
    pub neighbor: KgNode,
}

/// The relationship inspector payload for one node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDetails {
    pub node: KgNode,
    pub relationships: Vec<RelationshipDetail>,
}

/// Cache bookkeeping for the graph dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryCacheStats {
    pub cached_queries: u64,
}

/// Minimal embedding surface the graph needs from the memory vector
/// system. [`crate::copilot::memory::vector::MemoryVectorSystem`]
/// implements it; tests use a deterministic fake.
#[async_trait::async_trait]
pub trait GraphEmbedder: Send + Sync {
    async fn embed(&self, text: &str) -> Option<Vec<f32>>;
}
