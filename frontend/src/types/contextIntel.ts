// RC-8 M3 Context Intelligence types.
// Mirror of the backend `models::kg_context` DTOs (camelCase JSON).
// Kept separate from `types/graph.ts` because the M3 `ContextHit` shape
// differs from the M1 context-discovery hit of the same name.

import type { GraphNodeType, GraphRelationshipType, KgNode } from "./graph";

export type ContextSignalType =
  | "structural"
  | "semantic"
  | "temporal"
  | "goalOverlap"
  | "memory";

/** Per-signal confidence contributions plus the weighted total. */
export interface ConfidenceBreakdown {
  structural: number;
  semantic: number;
  temporal: number;
  memory: number;
  /** Weighted total in [0, 1]. */
  total: number;
}

/** One ranked context hit in an inference or fused payload (M3 shape). */
export interface ContextHit {
  node: KgNode;
  /** Human-readable reason ("Direct file connection", ...). */
  reason: string;
  /** Rank score in [0, 1]. */
  score: number;
  signal: ContextSignalType;
}

/** Ranked context inferred around one entity. */
export interface ContextInference {
  source: KgNode;
  related: ContextHit[];
  confidence: ConfidenceBreakdown;
  inferredAt: string;
}

/** One piece of evidence contributing to a workspace similarity. */
export interface SignalEvidence {
  signal: ContextSignalType;
  /** Normalized contribution in [0, 1]. */
  score: number;
  detail: string;
}

/** One cross-workspace relationship discovered from the graph. */
export interface WorkspaceSimilarity {
  sourceWorkspaceId: string;
  targetWorkspaceId: string;
  targetName: string;
  /** Combined similarity in [0, 1]. */
  similarity: number;
  /** Confidence in the relationship. */
  confidence: number;
  /** Evidence signals for the "why related?" panel. */
  signals: SignalEvidence[];
  /** Whether the relationship is persisted for cross-session reuse. */
  persisted: boolean;
}

/** The workspace similarity explorer payload for one workspace. */
export interface WorkspaceSimilarityResult {
  sourceWorkspaceId: string;
  sourceName: string;
  related: WorkspaceSimilarity[];
  /** True when served from the query cache. */
  cached: boolean;
  computedAt: string;
}

/** One member of a goal-similarity cluster. */
export interface ClusterMember {
  nodeType: GraphNodeType;
  entityId: string;
  title: string;
  workspaceId: string | null;
  /** Membership score in [0, 1]. */
  score: number;
}

/** One persisted goal-similarity cluster. */
export interface GoalCluster {
  id: number;
  workspaceId: string | null;
  name: string;
  memberCount: number;
  members: ClusterMember[];
  centroidTerms: string[];
  /** Cluster cohesion (mean pairwise similarity). */
  confidence: number;
}

/** One knowledge summary point for an entity. */
export interface SummaryPoint {
  label: string;
  value: string;
  detail: string | null;
}

/** Knowledge summary of one graph entity. */
export interface KnowledgeSummary {
  node: KgNode;
  points: SummaryPoint[];
  confidence: number;
  generatedAt: string;
}

/** One persisted graph context snapshot. */
export interface ContextIntelSnapshot {
  id: number;
  workspaceId: string;
  snapshotType: string;
  nodeCount: number;
  edgeCount: number;
  confidence: number;
  summary: SummaryPoint[];
  createdAt: string;
}

/** One entry of the context timeline: snapshot plus deltas vs. prior. */
export interface ContextTimelineEntry {
  snapshot: ContextIntelSnapshot;
  nodesDelta: number;
  edgesDelta: number;
  confidenceDelta: number;
}

export type FusedHitSource = "knowledgeGraph" | "memory";

/** One fused hit — memory + knowledge graph context combined. */
export interface FusedHit {
  node: KgNode;
  source: FusedHitSource;
  reason: string;
  score: number;
  /** Combined confidence across both channels. */
  confidence: number;
}

/** Memory + knowledge-graph context fused for one entity. */
export interface FusedContext {
  source: KgNode;
  kgHits: ContextHit[];
  memoryHits: ContextHit[];
  fused: FusedHit[];
  confidence: ConfidenceBreakdown;
  fusedAt: string;
}

/** Graph-assisted planner context retrieval around a goal anchor. */
export interface PlannerContext {
  goal: string;
  /** Best graph anchor for the goal, if any. */
  anchor: KgNode | null;
  /** Fused context around the anchor; null when no anchor matched. */
  context: FusedContext | null;
  /** One-line retrieval summary shown to the planner. */
  summary: string;
  retrievedAt: string;
}

/** One step of an explanation chain between two nodes. */
export interface ExplanationLink {
  from: KgNode;
  to: KgNode;
  relationshipType: GraphRelationshipType;
  reason: string;
  score: number;
  confidence: number;
}

/** Why-nodes-are-related explanation payload. */
export interface ContextExplanation {
  source: KgNode;
  target: KgNode;
  /** The traversal chain; empty when only heuristic overlap explains it. */
  chain: ExplanationLink[];
  /** One-line human summary. */
  summary: string;
  confidence: number;
}
