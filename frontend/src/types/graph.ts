import type { SearchEntityType } from "./search";

export type GraphEntityType = SearchEntityType | "folder" | "language" | "project";

export type GraphEdgeType =
  | "co_occurrence"
  | "semantic_similarity"
  | "explicit_reference"
  | "derivation";

export interface GraphNode {
  entityType: GraphEntityType;
  entityId: string;
  title: string;
  workspaceId: string;
  metadata?: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  sourceEntityType: GraphEntityType;
  sourceEntityId: string;
  targetEntityType: GraphEntityType;
  targetEntityId: string;
  edgeType: GraphEdgeType;
  weight: number;
  workspaceId: string;
  metadata: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface GraphView {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface GraphStats {
  nodeCount: number;
  edgeCount: number;
  avgWeight: number;
  maxWeight: number;
  density: number;
}

export interface NodeDetails {
  node: GraphNode;
  edges: GraphEdge[];
}

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
  weight: number;
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
