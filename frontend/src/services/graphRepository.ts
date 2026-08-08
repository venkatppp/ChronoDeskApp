import { invoke } from "@tauri-apps/api/core";
import type {
  KgNode,
  KgSubgraph,
  GraphPath,
  ContextDiscovery,
  GraphSyncSummary,
  KgStats,
  GraphNodeType,
  SemanticEdgeResult,
  EdgeDecaySummary,
  GraphAnalytics,
  RelationshipDetails,
} from "@/types/graph";
import type {
  ContextInference,
  ContextIntelSnapshot,
  GoalCluster,
  KnowledgeSummary,
  WorkspaceSimilarityResult,
} from "@/types/contextIntel";

export interface GraphRepository {
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
  graphRebuildSemanticEdges(maxNodes?: number): Promise<SemanticEdgeResult>;
  graphApplyEdgeDecay(): Promise<EdgeDecaySummary>;
  graphAnalytics(workspaceId?: string, cached?: boolean): Promise<GraphAnalytics>;
  graphRelationshipDetails(nodeType: GraphNodeType, entityId: string): Promise<RelationshipDetails>;

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
}

export class TauriGraphRepository implements GraphRepository {
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

  async graphRebuildSemanticEdges(maxNodes?: number): Promise<SemanticEdgeResult> {
    return invoke<SemanticEdgeResult>("graph_rebuild_semantic_edges", { maxNodes });
  }

  async graphApplyEdgeDecay(): Promise<EdgeDecaySummary> {
    return invoke<EdgeDecaySummary>("graph_apply_edge_decay");
  }

  async graphAnalytics(workspaceId?: string, cached?: boolean): Promise<GraphAnalytics> {
    return invoke<GraphAnalytics>("graph_analytics", { workspaceId, cached });
  }

  async graphRelationshipDetails(nodeType: GraphNodeType, entityId: string): Promise<RelationshipDetails> {
    return invoke<RelationshipDetails>("graph_relationship_details", { nodeType, entityId });
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
}

let repositoryInstance: GraphRepository | null = null;

export function getGraphRepository(): GraphRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphRepository();
  }
  return repositoryInstance;
}
