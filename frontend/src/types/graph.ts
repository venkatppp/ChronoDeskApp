// ----------------------------------------------------------------------
// RC-8 Knowledge Graph (typed node registry + relationships)
// ----------------------------------------------------------------------

export type GraphNodeType =
  | "workspace"
  | "file"
  | "planner_report"
  | "execution"
  | "memory_record"
  | "autonomous_session";

export type GraphRelationshipType =
  | "contains"
  | "runs_in"
  | "reports_on"
  | "derived_from"
  | "related_to";

/** One node in the RC-8 knowledge graph (`graph_nodes` row). */
export interface KgNode {
  nodeType: GraphNodeType;
  entityId: string;
  title: string;
  workspaceId: string | null;
  summary: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

/** One relationship between two RC-8 knowledge graph nodes. */
export interface KgEdge {
  id: string;
  sourceNodeType: GraphNodeType;
  sourceEntityId: string;
  targetNodeType: GraphNodeType;
  targetEntityId: string;
  relationshipType: GraphRelationshipType;
  /** Strength of the relationship (0.0 to 1.0). */
  weight: number;
  /**
   * Confidence in the relationship (0.0 to 1.0, RC-8 M2). Structural
   * edges are constructed at 1.0 and never decay; semantic `related_to`
   * edges start at their similarity and decay over time until dropped
   * below the pruning threshold.
   */
  confidence: number;
  metadata: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

/** A BFS section of the knowledge graph around a root node. */
export interface KgSubgraph {
  root: KgNode;
  nodes: KgNode[];
  edges: KgEdge[];
}

/** Shortest path between two nodes. */
export interface GraphPath {
  nodes: KgNode[];
  edges: KgEdge[];
}

/** One related entity returned by context relationship discovery. */
export interface ContextHit {
  node: KgNode;
  relationshipType: GraphRelationshipType | null;
  reason: string;
  weight: number;
}

/** Ranked context surrounding one entity. */
export interface ContextDiscovery {
  source: KgNode;
  related: ContextHit[];
}

/** Accounting for one graph construction pass. */
export interface GraphSyncSummary {
  createdNodes: number;
  updatedNodes: number;
  createdEdges: number;
  updatedEdges: number;
  totalNodes: number;
  totalEdges: number;
}

export interface TypeCount {
  name: string;
  count: number;
}

/** Aggregate statistics for the RC-8 knowledge graph. */
export interface KgStats {
  nodeCount: number;
  edgeCount: number;
  nodesByType: TypeCount[];
  edgesByType: TypeCount[];
}

// ----------------------------------------------------------------------
// RC-8 M2: Live knowledge graph (incremental sync, semantic edges,
// analytics, relationship inspector)
// ----------------------------------------------------------------------

/** Accounting for one semantic edge build pass. */
export interface SemanticEdgeResult {
  /** Node pairs whose similarity cleared the cosine threshold. */
  candidatePairs: number;
  /** `related_to` edges newly persisted. */
  created: number;
  /** `related_to` edges refreshed (similarity updated). */
  updated: number;
  /** Stale `related_to` edges pruned (below the threshold). */
  pruned: number;
  /** Cosine threshold used for the pass. */
  threshold: number;
}

/** Accounting for one edge-decay maintenance pass. */
export interface EdgeDecaySummary {
  /** Edges whose confidence was aged down. */
  decayed: number;
  /** Edges pruned after decaying below the minimum confidence. */
  pruned: number;
  /** Confidence floor under which edges are removed. */
  minConfidence: number;
}

/** One bucket of the degree distribution histogram. */
export interface DegreeBucket {
  degree: number;
  count: number;
}

/** Centrality of one graph node (degree + eigenvector-style ranking). */
export interface NodeCentrality {
  nodeType: GraphNodeType;
  entityId: string;
  title: string;
  inDegree: number;
  outDegree: number;
  /** Normalized degree centrality (degree / (n - 1)). */
  degreeCentrality: number;
  /** Power-iteration eigenvector score (ranking importance). */
  eigenvector: number;
}

/** One connected component of the (undirected) graph. */
export interface GraphComponent {
  /** Stable per-computation index, largest components first. */
  index: number;
  size: number;
  /** Count of nodes per node type inside the component. */
  nodeTypes: TypeCount[];
  /** Sample of member titles (up to 5) for the dashboard. */
  memberTitles: string[];
}

/** Importance of one workspace in the whole graph (global scope). */
export interface WorkspaceImportance {
  workspaceId: string;
  name: string;
  /** Rank score: eigenvector mass plus weighted edge strength. */
  importance: number;
  nodeCount: number;
  edgeCount: number;
  /** Sum of confidence-weighted edge weights touching the workspace. */
  weightSum: number;
}

/** Full analytics payload for the dashboard (cached per scope). */
export interface GraphAnalytics {
  /** Cache scope key: `all` or a workspace id. */
  scope: string;
  nodeCount: number;
  edgeCount: number;
  averageDegree: number;
  density: number;
  degreeDistribution: DegreeBucket[];
  /** Top nodes by eigenvector centrality (capped at 10). */
  topCentralNodes: NodeCentrality[];
  components: GraphComponent[];
  /** Workspace importance (global scope only; empty otherwise). */
  workspaceImportance: WorkspaceImportance[];
  /** True when served from the query cache. */
  cached: boolean;
  computedAt: string;
}

/** The relationship inspector payload for one node. */
export interface RelationshipDetail {
  edge: KgEdge;
  neighbor: KgNode;
}

/** The relationship inspector payload for one node. */
export interface RelationshipDetails {
  node: KgNode;
  relationships: RelationshipDetail[];
}
