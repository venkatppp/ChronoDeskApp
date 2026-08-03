import { invoke } from "@tauri-apps/api/core";
import type { GraphNodeType } from "@/types/graph";
import type {
  BenchmarkSuiteResult,
  ConsistencyReport,
  EdgePage,
  GraphDiagnostics,
  GraphMemoryStats,
  IntegrityCheckResult,
  MaintenanceRun,
  NeighborPage,
  NodePage,
  OrphanCleanupResult,
  OrphanSummary,
  ParallelWalkResult,
  QueryMetric,
  RankedSearchHit,
  RepairResult,
} from "@/types/graphOptimization";

export interface GraphOptimizationRepository {
  // Pagination / progressive loading.
  graphNodesPage(
    nodeTypes?: GraphNodeType[],
    workspaceId?: string,
    offset?: number,
    limit?: number,
  ): Promise<NodePage>;
  graphNodesTotal(nodeTypes?: GraphNodeType[], workspaceId?: string): Promise<number>;
  graphEdgesPage(offset?: number, limit?: number): Promise<EdgePage>;
  graphNeighborsPage(
    nodeType: GraphNodeType,
    entityId: string,
    offset?: number,
    limit?: number,
  ): Promise<NeighborPage>;

  // Search & traversal.
  graphRankedSearch(
    query: string,
    nodeTypes?: GraphNodeType[],
    limit?: number,
  ): Promise<RankedSearchHit[]>;
  graphVectorSearch(
    query: string,
    nodeTypes?: GraphNodeType[],
    limit?: number,
  ): Promise<RankedSearchHit[]>;
  graphParallelTraverse(
    roots: [GraphNodeType, string][],
    maxDepth?: number,
    budget?: number,
  ): Promise<ParallelWalkResult>;

  // Cache & memory.
  graphCacheTrim(n: number): Promise<number>;
  graphClearExpiredCache(): Promise<number>;
  graphMemoryStats(): Promise<GraphMemoryStats>;
  graphRecentMetrics(limit?: number): Promise<QueryMetric[]>;

  // Health.
  graphIntegrityCheck(): Promise<IntegrityCheckResult>;
  graphRepair(): Promise<RepairResult>;
  graphOrphanSummary(): Promise<OrphanSummary>;
  graphOrphanCleanup(): Promise<OrphanCleanupResult>;
  graphConsistencyReport(): Promise<ConsistencyReport>;
  graphMaintenanceRuns(limit?: number): Promise<MaintenanceRun[]>;
  graphBenchmarkSuite(suiteName?: string): Promise<BenchmarkSuiteResult>;
  graphDiagnostics(): Promise<GraphDiagnostics>;
}

export class TauriGraphOptimizationRepository implements GraphOptimizationRepository {
  async graphNodesPage(
    nodeTypes?: GraphNodeType[],
    workspaceId?: string,
    offset?: number,
    limit?: number,
  ): Promise<NodePage> {
    return invoke<NodePage>("graph_nodes_page", { nodeTypes, workspaceId, offset, limit });
  }

  async graphNodesTotal(nodeTypes?: GraphNodeType[], workspaceId?: string): Promise<number> {
    return invoke<number>("graph_nodes_total", { nodeTypes, workspaceId });
  }

  async graphEdgesPage(offset?: number, limit?: number): Promise<EdgePage> {
    return invoke<EdgePage>("graph_edges_page", { offset, limit });
  }

  async graphNeighborsPage(
    nodeType: GraphNodeType,
    entityId: string,
    offset?: number,
    limit?: number,
  ): Promise<NeighborPage> {
    return invoke<NeighborPage>("graph_neighbors_page", { nodeType, entityId, offset, limit });
  }

  async graphRankedSearch(
    query: string,
    nodeTypes?: GraphNodeType[],
    limit?: number,
  ): Promise<RankedSearchHit[]> {
    return invoke<RankedSearchHit[]>("graph_ranked_search", { query, nodeTypes, limit });
  }

  async graphVectorSearch(
    query: string,
    nodeTypes?: GraphNodeType[],
    limit?: number,
  ): Promise<RankedSearchHit[]> {
    return invoke<RankedSearchHit[]>("graph_vector_search", { query, nodeTypes, limit });
  }

  async graphParallelTraverse(
    roots: [GraphNodeType, string][],
    maxDepth?: number,
    budget?: number,
  ): Promise<ParallelWalkResult> {
    return invoke<ParallelWalkResult>("graph_parallel_traverse", {
      roots,
      maxDepth,
      budget,
    });
  }

  async graphCacheTrim(n: number): Promise<number> {
    return invoke<number>("graph_cache_trim", { n });
  }

  async graphClearExpiredCache(): Promise<number> {
    return invoke<number>("graph_clear_expired_cache");
  }

  async graphMemoryStats(): Promise<GraphMemoryStats> {
    return invoke<GraphMemoryStats>("graph_memory_stats");
  }

  async graphRecentMetrics(limit?: number): Promise<QueryMetric[]> {
    return invoke<QueryMetric[]>("graph_recent_metrics", { limit });
  }

  async graphIntegrityCheck(): Promise<IntegrityCheckResult> {
    return invoke<IntegrityCheckResult>("graph_integrity_check");
  }

  async graphRepair(): Promise<RepairResult> {
    return invoke<RepairResult>("graph_repair");
  }

  async graphOrphanSummary(): Promise<OrphanSummary> {
    return invoke<OrphanSummary>("graph_orphan_summary");
  }

  async graphOrphanCleanup(): Promise<OrphanCleanupResult> {
    return invoke<OrphanCleanupResult>("graph_orphan_cleanup");
  }

  async graphConsistencyReport(): Promise<ConsistencyReport> {
    return invoke<ConsistencyReport>("graph_consistency_report");
  }

  async graphMaintenanceRuns(limit = 10): Promise<MaintenanceRun[]> {
    return invoke<MaintenanceRun[]>("graph_maintenance_runs", { limit });
  }

  async graphBenchmarkSuite(suiteName?: string): Promise<BenchmarkSuiteResult> {
    return invoke<BenchmarkSuiteResult>("graph_benchmark_suite", { suiteName });
  }

  async graphDiagnostics(): Promise<GraphDiagnostics> {
    return invoke<GraphDiagnostics>("graph_diagnostics");
  }
}

let repositoryInstance: GraphOptimizationRepository | null = null;

export function getGraphOptimizationRepository(): GraphOptimizationRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriGraphOptimizationRepository();
  }
  return repositoryInstance;
}