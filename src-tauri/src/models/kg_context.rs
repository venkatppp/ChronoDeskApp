//! Context Intelligence models (RC-8 M3).
//!
//! Additive to the M1 `models::kg` types and M2 `models::kg_live` types:
//! context inference around an entity, graph-based workspace similarity
//! and cross-workspace relationships, goal-similarity clusters,
//! knowledge summaries, graph context snapshots + timeline, memory + KG
//! context fusion, graph-assisted planner context retrieval, context
//! explanations, and per-signal confidence breakdowns. Every type here
//! is a plain serializable DTO — the SQL lives in
//! [`crate::repositories::ContextIntelRepository`], the scoring in
//! [`crate::services::ContextIntelService`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::kg::GraphNodeType;

/// The signal class behind one context hit / similarity evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextSignalType {
    /// Structural graph reachability (`contains`, `runs_in`, ...).
    Structural,
    /// Semantic `related_to` cosine similarity.
    Semantic,
    /// Recency / activity recency of the hit.
    Temporal,
    /// Shared goal vocabulary between workspaces or nodes.
    GoalOverlap,
    /// Memory-record provenance (a memory node in the graph).
    Memory,
}

/// Confidence breakdown for one inference — the per-signal contributions
/// plus the combined total the frontend visualizes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceBreakdown {
    pub structural: f64,
    pub semantic: f64,
    pub temporal: f64,
    pub memory: f64,
    /// Weighted total in `[0, 1]`.
    pub total: f64,
}

/// One context hit in an inference or fused payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextHit {
    pub node: crate::models::kg::KgNode,
    /// Human-readable reason ("Direct file connection", "Semantically
    /// similar", ...).
    pub reason: String,
    /// Rank score in `[0, 1]`.
    pub score: f64,
    pub signal: ContextSignalType,
}

/// Ranked context inferred around one entity (context inference engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextInference {
    pub source: crate::models::kg::KgNode,
    pub related: Vec<ContextHit>,
    pub confidence: ConfidenceBreakdown,
    pub inferred_at: DateTime<Utc>,
}

/// One piece of evidence contributing to a workspace similarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalEvidence {
    pub signal: ContextSignalType,
    /// Normalized contribution in `[0, 1]`.
    pub score: f64,
    /// Detail shown in the explanation panel.
    pub detail: String,
}

/// One cross-workspace relationship discovered from the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSimilarity {
    pub source_workspace_id: Uuid,
    pub target_workspace_id: Uuid,
    pub target_name: String,
    /// Combined similarity in `[0, 1]`.
    pub similarity: f64,
    /// Confidence in the relationship.
    pub confidence: f64,
    /// Explanation + evidence signals for the "why related?" panel.
    pub signals: Vec<SignalEvidence>,
    /// Whether the relationship is persisted for cross-session reuse.
    pub persisted: bool,
}

/// The workspace similarity explorer payload for one workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSimilarityResult {
    pub source_workspace_id: Uuid,
    pub source_name: String,
    pub related: Vec<WorkspaceSimilarity>,
    /// True when served from the query cache.
    pub cached: bool,
    pub computed_at: DateTime<Utc>,
}

/// One member of a goal-similarity cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterMember {
    pub node_type: GraphNodeType,
    pub entity_id: Uuid,
    pub title: String,
    pub workspace_id: Option<Uuid>,
    /// Membership score in `[0, 1]`.
    pub score: f64,
}

/// One persisted goal-similarity cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalCluster {
    pub id: i64,
    pub workspace_id: Option<Uuid>,
    pub name: String,
    pub member_count: u64,
    pub members: Vec<ClusterMember>,
    pub centroid_terms: Vec<String>,
    /// Cluster cohesion (mean pairwise similarity).
    pub confidence: f64,
}

/// One knowledge summary point for an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryPoint {
    /// Short label shown on the card ("Connections", "Related work", ...).
    pub label: String,
    /// The value text.
    pub value: String,
    /// Optional detail line.
    pub detail: Option<String>,
}

/// Knowledge summary of one graph entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSummary {
    pub node: crate::models::kg::KgNode,
    pub points: Vec<SummaryPoint>,
    pub confidence: f64,
    pub generated_at: DateTime<Utc>,
}

/// One persisted graph context snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextIntelSnapshot {
    pub id: i64,
    pub workspace_id: Uuid,
    pub snapshot_type: String,
    pub node_count: u64,
    pub edge_count: u64,
    pub confidence: f64,
    pub summary: Vec<SummaryPoint>,
    pub created_at: DateTime<Utc>,
}

/// One entry of the context timeline: a snapshot with the deltas against
/// the previous snapshot (what changed since then).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextTimelineEntry {
    pub snapshot: ContextIntelSnapshot,
    pub nodes_delta: i64,
    pub edges_delta: i64,
    pub confidence_delta: f64,
}

/// Where a fused hit came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FusedHitSource {
    /// A knowledge-graph structural/semantic hit.
    KnowledgeGraph,
    /// A memory-record hit (memory is a first-class node type in the KG).
    Memory,
}

/// One fused hit — memory + knowledge graph context combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FusedHit {
    pub node: crate::models::kg::KgNode,
    pub source: FusedHitSource,
    pub reason: String,
    pub score: f64,
    /// Combined confidence across both channels.
    pub confidence: f64,
}

/// Memory + knowledge-graph context fused for one entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FusedContext {
    pub source: crate::models::kg::KgNode,
    /// KG-derived hits (structural + semantic).
    pub kg_hits: Vec<ContextHit>,
    /// Memory-derived hits (MemoryRecord nodes reached via the graph).
    pub memory_hits: Vec<ContextHit>,
    /// The merged, deduplicated, ranked hit list.
    pub fused: Vec<FusedHit>,
    pub confidence: ConfidenceBreakdown,
    pub fused_at: DateTime<Utc>,
}

/// Graph-assisted planner context retrieval: the anchor node plus the
/// fused context the planner can base its decisions on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerContext {
    /// The goal text searched for.
    pub goal: String,
    /// Best graph anchor for the goal, if any.
    pub anchor: Option<crate::models::kg::KgNode>,
    /// Fused context around the anchor; `None` when no graph anchor
    /// matched the goal.
    pub context: Option<FusedContext>,
    /// One-line retrieval summary shown to the planner.
    pub summary: String,
    pub retrieved_at: DateTime<Utc>,
}

/// One step of an explanation chain between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplanationLink {
    pub from: crate::models::kg::KgNode,
    pub to: crate::models::kg::KgNode,
    pub relationship_type: crate::models::kg::GraphRelationshipType,
    pub reason: String,
    pub score: f64,
    pub confidence: f64,
}

/// Why-nodes-are-related explanation payload (context explanation engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextExplanation {
    pub source: crate::models::kg::KgNode,
    pub target: crate::models::kg::KgNode,
    /// The traversal chain; empty when only heuristic overlap explains it.
    pub chain: Vec<ExplanationLink>,
    /// One-line human summary.
    pub summary: String,
    pub confidence: f64,
}
