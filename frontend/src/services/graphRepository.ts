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
  EntitySyncResult,
  SemanticEdgeResult,
  EdgeDecaySummary,
  GraphAnalytics,
  MultiHopContext,
  GraphRecommendation,
  RelationshipDetails,
  QueryCacheStats,
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

  // RC-8 M2: live knowledge graph.
  graphIncrementalSync(): Promise<GraphSyncSummary>;
  graphSyncEntity(nodeType: GraphNodeType, entityId: string): Promise<EntitySyncResult>;
  graphRebuildSemanticEdges(maxNodes?: number): Promise<SemanticEdgeResult>;
  graphApplyEdgeDecay(): Promise<EdgeDecaySummary>;
  graphAnalytics(workspaceId?: string, cached?: boolean): Promise<GraphAnalytics>;
  graphExpandContext(
    nodeType: GraphNodeType,
    entityId: string,
    hops?: number,
    limit?: number,
    cached?: boolean,
  ): Promise<MultiHopContext>;
  graphRecommendations(
    nodeType: GraphNodeType,
    entityId: string,
    limit?: number,
    cached?: boolean,
  ): Promise<GraphRecommendation[]>;
  graphRelationshipDetails(nodeType: GraphNodeType, entityId: string): Promise<RelationshipDetails>;
  graphCacheStats(): Promise<QueryCacheStats>;
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

  async graphIncrementalSync(): Promise<GraphSyncSummary> {
    return invoke<GraphSyncSummary>("graph_incremental_sync");
  }

  async graphSyncEntity(nodeType: GraphNodeType, entityId: string): Promise<EntitySyncResult> {
    return invoke<EntitySyncResult>("graph_sync_entity", { nodeType, entityId });
  }

  async graphRebuildSemanticEdges(maxNodes?: number): Promise<SemanticEdgeResult> {
    return invoke<SemanticEdgeResult>("graph_rebuild_semantic_edges", { maxNodes });
  }

  async graphApplyEdgeDecay(): Promise<EdgeDecaySummary> {
    return invoke<EdgeDecaySummary>("graph_apply_edge_decay");
  }

  async graphAnalytics(workspaceId?: string, cached?: boolean): Promise<GraphAnalytics> {
    return invoke<GraphAnalytics>("graph_analytics", { workspaceId, cached });
  }

  async graphExpandContext(
    nodeType: GraphNodeType,
    entityId: string,
    hops?: number,
    limit?: number,
    cached?: boolean,
  ): Promise<MultiHopContext> {
    return invoke<MultiHopContext>("graph_expand_context", {
      nodeType,
      entityId,
      hops,
      limit,
      cached,
    });
  }

  async graphRecommendations(
    nodeType: GraphNodeType,
    entityId: string,
    limit?: number,
    cached?: boolean,
  ): Promise<GraphRecommendation[]> {
    return invoke<GraphRecommendation[]>("graph_recommendations", {
      nodeType,
      entityId,
      limit,
      cached,
    });
  }

  async graphRelationshipDetails(nodeType: GraphNodeType, entityId: string): Promise<RelationshipDetails> {
    return invoke<RelationshipDetails>("graph_relationship_details", { nodeType, entityId });
  }

  async graphCacheStats(): Promise<QueryCacheStats> {
    return invoke<QueryCacheStats>("graph_cache_stats");
  }
}

let repositoryInstance: GraphRepository | null = null;

export function getGraphRepository(): GraphRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphRepository();
  }
  return repositoryInstance;
}
