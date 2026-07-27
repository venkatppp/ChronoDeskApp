import type { SearchEntityType } from "./search";

export type GraphEdgeType =
  | "co_occurrence"
  | "semantic_similarity"
  | "explicit_reference"
  | "derivation";

export interface GraphNode {
  entityType: SearchEntityType;
  entityId: string;
  title: string;
  workspaceId: string;
}

export interface GraphEdge {
  id: string;
  sourceEntityType: SearchEntityType;
  sourceEntityId: string;
  targetEntityType: SearchEntityType;
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
