import { invoke } from "@tauri-apps/api/core";
import type { GraphView, GraphEdgeType, NodeDetails, GraphStats } from "@/types/graph";
import type { SearchEntityType } from "@/types/search";

export interface GraphRepository {
  getGraph(workspaceId?: string, edgeTypes?: GraphEdgeType[]): Promise<GraphView>;
  getNodeDetails(entityId: string, entityType: SearchEntityType): Promise<NodeDetails>;
  getGraphStats(workspaceId?: string): Promise<GraphStats>;
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
}

let repositoryInstance: GraphRepository | null = null;

export function getGraphRepository(): GraphRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphRepository();
  }
  return repositoryInstance;
}
