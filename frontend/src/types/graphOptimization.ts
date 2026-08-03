import type { KgNode, KgEdge, GraphNodeType, TypeCount } from "./graph";

// ----------------------------------------------------------------------
// RC-8 M4: Knowledge Graph Optimization & Scale
// ----------------------------------------------------------------------

/** One page of graph nodes (progressive / virtualized loading). */
export interface NodePage {
  nodes: KgNode[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
}

/** One page of graph edges. */
export interface EdgePage {
  edges: KgEdge[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
}

/** One neighbor of a node: the edge that connects them plus the node. */
export interface NeighborRow {
  edge: KgEdge;
  neighbor: KgNode;
}

/** One page of a node's neighbors. */
export interface NeighborPage {
  neighbors: NeighborRow[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
}

/** One ranked hit from the optimization search surfaces. */
export interface RankedSearchHit {
  node: KgNode;
  /** Normalized rank score in `[0, 1]`, sorted descending. */
  score: number;
  /** Which matcher produced the hit (`keyword` | `vector`). */
  method: string;
  /** Human-readable reason for the rank. */
  reason: string;
}

// ----------------------------------------------------------------------
// Integrity & health
// ----------------------------------------------------------------------

export type IssueType =
  | "orphan_edge"
  | "dangling_workspace"
  | "malformed_node"
  | "invalid_confidence";

export type IssueSeverity = "info" | "warning" | "critical";

/** One persisted integrity finding (`graph_integrity_issues` row). */
export interface GraphIntegrityIssue {
  id: number;
  issueType: IssueType;
  severity: IssueSeverity;
  /** Node type of the affected node, when applicable. */
  nodeType: GraphNodeType | null;
  /** The affected node/edge id. */
  entityId: string | null;
  detail: string;
  /** `open` | `resolved`. */
  status: string;
  createdAt: string;
  resolvedAt: string | null;
}

/** Accounting + findings of one integrity check pass. */
export interface IntegrityCheckResult {
  issues: GraphIntegrityIssue[];
  issueTypeCounts: TypeCount[];
  checkedAt: string;
}

/** Type-level resolution counts of one repair pass. */
export interface RepairResult {
  orphanEdgesRemoved: number;
  danglingWorkspacesRemoved: number;
  malformedNodesFixed: number;
  invalidConfidenceFixed: number;
  issuesResolved: number;
}

/** Live orphan bookkeeping without mutating anything. */
export interface OrphanSummary {
  orphanEdges: number;
  danglingWorkspaces: number;
}

/** Accounting of one orphan cleanup pass. */
export interface OrphanCleanupResult {
  orphanEdgesRemoved: number;
  danglingWorkspacesRemoved: number;
  issuesResolved: number;
}

/** One consistency verification question with a pass/fail verdict. */
export interface ConsistencyCheck {
  name: string;
  passed: boolean;
  detail: string;
}

/** Aggregate consistency report for the diagnostics panel. */
export interface ConsistencyReport {
  checks: ConsistencyCheck[];
  passed: boolean;
  checkedAt: string;
}

// ----------------------------------------------------------------------
// Performance observability
// ----------------------------------------------------------------------

/** One recorded operation metric (`graph_query_metrics` row). */
export interface QueryMetric {
  id: number;
  operation: string;
  scope: string | null;
  query: string | null;
  durationMs: number;
  rowsReturned: number;
  hitCache: boolean;
  occurredAt: string;
}

/** Cached + persisted graph memory bookkeeping for the dashboard. */
export interface GraphMemoryStats {
  nodeCount: number;
  edgeCount: number;
  cacheEntries: number;
  /** Total bytes of cached payloads. */
  cacheSizeBytes: number;
  /** Rough in-memory footprint estimate of the node/edge registry. */
  estimatedBytes: number;
}

/** Maintenance run record for the history panel. */
export interface MaintenanceRun {
  id: number;
  runType: string;
  status: string;
  issuesFound: number;
  issuesResolved: number;
  durationMs: number;
  summary: Record<string, unknown>;
  startedAt: string;
  finishedAt: string | null;
}

// ----------------------------------------------------------------------
// Benchmarks
// ----------------------------------------------------------------------

/** One micro-benchmark result within a suite run. */
export interface GraphBenchmarkResult {
  name: string;
  operation: string;
  nodeCount: number;
  edgeCount: number;
  /** Wall time of the benchmarked call. */
  durationMs: number;
  /** Throughput (rows/ops per second) where meaningful. */
  throughputPerSec: number | null;
  /** Suite name grouping this benchmark. */
  suiteName: string;
  createdAt: string;
}

/** Aggregate payload of one benchmark suite run. */
export interface BenchmarkSuiteResult {
  suiteName: string;
  benchmarks: GraphBenchmarkResult[];
  totalDurationMs: number;
  ranAt: string;
}

/** Accounting + payload of a parallel multi-root traversal pass. */
export interface ParallelWalkResult {
  roots: number;
  /** Unique nodes reached from all roots, deduplicated. */
  nodes: KgNode[];
  /** Edges whose both endpoints were reached. */
  edges: KgEdge[];
  nodeCount: number;
  edgeCount: number;
  /** Maximum BFS depth explored from any root. */
  maxDepth: number;
  durationMs: number;
}

// ----------------------------------------------------------------------
// Diagnostics bundle
// ----------------------------------------------------------------------

/** The full graph performance/health bundle the frontend page renders. */
export interface GraphDiagnostics {
  integrity: IntegrityCheckResult;
  consistency: ConsistencyReport;
  memory: GraphMemoryStats;
  recentMaintenance: MaintenanceRun[];
  recentBenchmarks: GraphBenchmarkResult[];
  recentMetrics: QueryMetric[];
}
