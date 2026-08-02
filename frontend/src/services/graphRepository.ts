import { invoke } from "@tauri-apps/api/core";
import type {
  GraphView,
  GraphEdgeType,
  NodeDetails,
  GraphStats,
  KgNode,
  KgSubgraph,
  GraphPath,
  ContextDiscovery,
  GraphSyncSummary,
  KgStats,
  GraphNodeType,
} from "@/types/graph";
import type { SearchEntityType } from "@/types/search";

export interface GraphRepository {
  // Legacy (Phase 4) graph_edges view.
  getGraph(workspaceId?: string, edgeTypes?: GraphEdgeType[]): Promise<GraphView>;
  getNodeDetails(entityId: string, entityType: SearchEntityType): Promise<NodeDetails>;
  getGraphStats(workspaceId?: string): Promise<GraphStats>;

  // RC-8 knowledge graph.
  syncGraph(): Promise<GraphSyncSummary>;
  searchGraphNodes(query: string, nodeTypes?: GraphNodeType[], limit?: number): Promise<KgNode[]>;
  graphSubgraph(nodeType: GraphNodeType, entityId: string, depth?: number): Promise<KgSubgraph>;
  graphPath(
    sourceNodeType: GraphNodeType,
    sourceEntityId: string,
    targetNodeType: GraphNodeType,
    targetEntityId: string,
    maxDepth?: number,
  ): Promise<GraphPath | null>;
  graphContext(nodeType: GraphNodeType, entityId: string, limit?: number): Promise<ContextDiscovery>;
  graphKgStats(): Promise<KgStats>;
  graphNodes(
    nodeTypes: GraphNodeType[],
    workspaceId?: string,
    limit?: number,
  ): Promise<KgNode[]>;
}

export class TauriGraphRepository implements GraphRepository {
  async getGraph(workspaceId?: string, edgeTypes?: GraphEdgeType[]): Promise<GraphView> {
    return invoke<GraphView>("get_graph", { workspaceId, edgeTypes });
  }

  async getNodeDetails(entityId: string, entityType: SearchEntityType): Promise<NodeDetails> {
    return invoke<NodeDetails>("get_node_details", { entityId, entityType });
  }

  async getGraphStats(workspaceId?: string): Promise<GraphStats> {
    return invoke<GraphStats>("get_graph_stats", { workspaceId });
  }

  async syncGraph(): Promise<GraphSyncSummary> {
    return invoke<GraphSyncSummary>("graph_sync");
  }

  async searchGraphNodes(query: string, nodeTypes?: GraphNodeType[], limit?: number): Promise<KgNode[]> {
    return invoke<KgNode[]>("graph_search", { query, nodeTypes, limit });
  }

  async graphSubgraph(nodeType: GraphNodeType, entityId: string, depth?: number): Promise<KgSubgraph> {
    return invoke<KgSubgraph>("graph_subgraph", { nodeType, entityId, depth });
  }

  async graphPath(
    sourceNodeType: GraphNodeType,
    sourceEntityId: string,
    targetNodeType: GraphNodeType,
    targetEntityId: string,
    maxDepth?: number,
  ): Promise<GraphPath | null> {
    return invoke<GraphPath | null>("graph_path", {
      sourceNodeType,
      sourceEntityId,
      targetNodeType,
      targetEntityId,
      maxDepth,
    });
  }

  async graphContext(nodeType: GraphNodeType, entityId: string, limit?: number): Promise<ContextDiscovery> {
    return invoke<ContextDiscovery>("graph_context", { nodeType, entityId, limit });
  }

  async graphKgStats(): Promise<KgStats> {
    return invoke<KgStats>("graph_kg_stats");
  }

  async graphNodes(nodeTypes: GraphNodeType[], workspaceId?: string, limit?: number): Promise<KgNode[]> {
    return invoke<KgNode[]>("graph_nodes", { nodeTypes, workspaceId, limit });
  }
}

let repositoryInstance: GraphRepository | null = null;

export function getGraphRepository(): GraphRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphRepository();
  }
  return repositoryInstance;
}
