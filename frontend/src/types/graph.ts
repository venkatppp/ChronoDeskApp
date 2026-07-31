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
