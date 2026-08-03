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
import type {
  ContextExplanation,
  ContextInference,
  ContextIntelSnapshot,
  ContextTimelineEntry,
  FusedContext,
  GoalCluster,
  KnowledgeSummary,
  PlannerContext,
  WorkspaceSimilarityResult,
} from "@/types/contextIntel";
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

  // RC-8 M3: context intelligence.
  graphInferContext(
    nodeType: GraphNodeType,
    entityId: string,
    limit?: number,
    cached?: boolean,
  ): Promise<ContextInference>;
  graphWorkspaceSimilarity(workspaceId: string, cached?: boolean): Promise<WorkspaceSimilarityResult>;
  graphDiscoverCrossWorkspaceRelationships(workspaceId: string): Promise<WorkspaceSimilarityResult>;
  graphGoalClusters(workspaceId?: string, cached?: boolean): Promise<GoalCluster[]>;
  graphKnowledgeSummary(
    nodeType: GraphNodeType,
    entityId: string,
    cached?: boolean,
  ): Promise<KnowledgeSummary>;
  graphSnapshotCreate(workspaceId: string, snapshotType?: string): Promise<ContextIntelSnapshot>;
  graphSnapshotList(workspaceId: string, limit?: number): Promise<ContextIntelSnapshot[]>;
  graphContextTimeline(workspaceId: string, limit?: number): Promise<ContextTimelineEntry[]>;
  graphFusedContext(
    nodeType: GraphNodeType,
    entityId: string,
    cached?: boolean,
  ): Promise<FusedContext>;
  graphPlannerContext(goal: string, cached?: boolean): Promise<PlannerContext>;
  graphExplain(
    sourceNodeType: GraphNodeType,
    sourceEntityId: string,
    targetNodeType: GraphNodeType,
    targetEntityId: string,
  ): Promise<ContextExplanation>;
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

  async graphInferContext(
    nodeType: GraphNodeType,
    entityId: string,
    limit?: number,
    cached?: boolean,
  ): Promise<ContextInference> {
    return invoke<ContextInference>("graph_infer_context", { nodeType, entityId, limit, cached });
  }

  async graphWorkspaceSimilarity(workspaceId: string, cached?: boolean): Promise<WorkspaceSimilarityResult> {
    return invoke<WorkspaceSimilarityResult>("graph_workspace_similarity", { workspaceId, cached });
  }

  async graphDiscoverCrossWorkspaceRelationships(workspaceId: string): Promise<WorkspaceSimilarityResult> {
    return invoke<WorkspaceSimilarityResult>("graph_discover_cross_workspace_relationships", {
      workspaceId,
    });
  }

  async graphGoalClusters(workspaceId?: string, cached?: boolean): Promise<GoalCluster[]> {
    return invoke<GoalCluster[]>("graph_goal_clusters", { workspaceId, cached });
  }

  async graphKnowledgeSummary(
    nodeType: GraphNodeType,
    entityId: string,
    cached?: boolean,
  ): Promise<KnowledgeSummary> {
    return invoke<KnowledgeSummary>("graph_knowledge_summary", { nodeType, entityId, cached });
  }

  async graphSnapshotCreate(workspaceId: string, snapshotType?: string): Promise<ContextIntelSnapshot> {
    return invoke<ContextIntelSnapshot>("graph_snapshot_create", { workspaceId, snapshotType });
  }

  async graphSnapshotList(workspaceId: string, limit?: number): Promise<ContextIntelSnapshot[]> {
    return invoke<ContextIntelSnapshot[]>("graph_snapshot_list", { workspaceId, limit });
  }

  async graphContextTimeline(workspaceId: string, limit?: number): Promise<ContextTimelineEntry[]> {
    return invoke<ContextTimelineEntry[]>("graph_context_timeline", { workspaceId, limit });
  }

  async graphFusedContext(
    nodeType: GraphNodeType,
    entityId: string,
    cached?: boolean,
  ): Promise<FusedContext> {
    return invoke<FusedContext>("graph_fused_context", { nodeType, entityId, cached });
  }

  async graphPlannerContext(goal: string, cached?: boolean): Promise<PlannerContext> {
    return invoke<PlannerContext>("graph_planner_context", { goal, cached });
  }

  async graphExplain(
    sourceNodeType: GraphNodeType,
    sourceEntityId: string,
    targetNodeType: GraphNodeType,
    targetEntityId: string,
  ): Promise<ContextExplanation> {
    return invoke<ContextExplanation>("graph_explain", {
      sourceNodeType,
      sourceEntityId,
      targetNodeType,
      targetEntityId,
    });
  }
}

let repositoryInstance: GraphRepository | null = null;

export function getGraphRepository(): GraphRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphRepository();
  }
  return repositoryInstance;
}
