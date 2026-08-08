import { invoke } from "@tauri-apps/api/core";
import type { GraphNodeType } from "@/types/graph";
import type {
  BenchmarkSuiteResult,
  ConsistencyReport,
  EdgePage,
  GraphDiagnostics,
  IntegrityCheckResult,
  NodePage,
  OrphanCleanupResult,
  OrphanSummary,
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

  graphEdgesPage(offset?: number, limit?: number): Promise<EdgePage>;

  // Cache & memory.
  graphCacheTrim(n: number): Promise<number>;
  graphClearExpiredCache(): Promise<number>;

  // Health.
  graphIntegrityCheck(): Promise<IntegrityCheckResult>;
  graphRepair(): Promise<RepairResult>;
  graphOrphanSummary(): Promise<OrphanSummary>;
  graphOrphanCleanup(): Promise<OrphanCleanupResult>;
  graphConsistencyReport(): Promise<ConsistencyReport>;
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

  async graphEdgesPage(offset?: number, limit?: number): Promise<EdgePage> {
    return invoke<EdgePage>("graph_edges_page", { offset, limit });
  }

  async graphCacheTrim(n: number): Promise<number> {
    return invoke<number>("graph_cache_trim", { n });
  }

  async graphClearExpiredCache(): Promise<number> {
    return invoke<number>("graph_clear_expired_cache");
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