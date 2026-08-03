// ----------------------------------------------------------------------
// RC-10 M1: Performance & Profiling
// Mirrors the camelCase DTOs from `models/performance.rs`.
// ----------------------------------------------------------------------

/** What kind of operation a sample captured. */
export type ProfileCategory = "command" | "service" | "repository" | "worker" | "engine";

/** One measured operation. */
export interface ProfileSample {
  id: number;
  category: ProfileCategory;
  name: string;
  durationMs: number;
  metadata: Record<string, unknown>;
  occurredAt: string;
}

/** Per-operation aggregate over the live window. */
export interface ProfileAggregate {
  category: ProfileCategory;
  name: string;
  count: number;
  avgMs: number;
  minMs: number;
  maxMs: number;
  /** 95th percentile latency. */
  p95Ms: number;
}

/** A point-in-time view of the profiler. */
export interface ProfileSnapshot {
  capturedAt: string;
  aggregates: ProfileAggregate[];
  recent: ProfileSample[];
  slowest: ProfileSample[];
}

/** One timed startup phase. */
export interface StartupStage {
  name: string;
  label: string;
  durationMs: number;
  startedAt: string;
}

/** The full report of one application launch. */
export interface StartupProfile {
  runId: string;
  totalMs: number;
  stages: StartupStage[];
  recordedAt: string;
}

/** Which subsystem a benchmark suite exercises. */
export type BenchmarkCategory = "planner" | "execution" | "memory" | "graph" | "vector";

/** One measured micro-benchmark within a suite run. */
export interface BenchmarkResult {
  id: number;
  name: string;
  operation: string;
  category: BenchmarkCategory;
  /** Iterations run; `durationMs` is the mean per iteration. */
  iterations: number;
  durationMs: number;
  /** Operations per second, when measurable. */
  throughputPerSec: number | null;
  /** `false` when the benchmark could not complete (e.g. not wired). */
  ok: boolean;
  /** Operation-specific accounting; a plain string when the run was skipped. */
  payload: Record<string, unknown> | string;
  createdAt: string;
}

/** The result of running one or more benchmark suites. */
export interface BenchmarkSuiteResult {
  suiteName: string;
  benchmarks: BenchmarkResult[];
  totalDurationMs: number;
  ranAt: string;
}

/** CPU-side system facts. */
export interface CpuUsage {
  /** Whole-system utilization, in `[0, 100]`. */
  usagePercent: number;
  cores: number;
  cpuParallelism: number;
}

/** Physical memory usage. */
export interface MemoryUsage {
  totalBytes: number;
  usedBytes: number;
  percent: number;
}

/** On-disk database footprint. */
export interface DbUsage {
  sizeBytes: number;
  path: string;
}

/** In-process cache health. */
export interface CacheUsage {
  runtimeEntries: number;
  runtimeHitRate: number;
  graphCacheEntries: number;
  graphCacheSizeBytes: number;
}

/** One background worker's observable state. */
export interface WorkerInfo {
  name: string;
  status: string;
  executionCount: number;
  errorCount: number;
  avgExecutionTimeMs: number;
  lastExecution: string | null;
}

/** Concurrency/process facts. */
export interface ThreadUsage {
  /** 0 on platforms sysinfo cannot enumerate threads for (e.g. macOS). */
  totalThreads: number;
  processCount: number;
}

/** A point-in-time snapshot of the whole application + machine. */
export interface DiagnosticsSnapshot {
  capturedAt: string;
  cpu: CpuUsage;
  memory: MemoryUsage;
  db: DbUsage;
  cache: CacheUsage;
  workers: WorkerInfo[];
  threads: ThreadUsage;
}

/** The optimization surface a recommendation belongs to. */
export type OptimizationCategory = "query" | "lazy_init" | "worker" | "cache" | "memory";

/** A remediation the optimizer can perform on the user's behalf. */
export type OptimizationAction =
  | "clear_expired_graph_cache"
  | { trim_graph_cache: number }
  | { prune_profile_history: number };

/** One actionable finding from the optimizer analysis. */
export interface OptimizationRecommendation {
  id: string;
  category: OptimizationCategory;
  /** `info` | `warning` | `critical`. */
  severity: string;
  title: string;
  detail: string;
  action: OptimizationAction | null;
}

/** Output of an optimizer run. */
export interface OptimizeResult {
  recommendations: OptimizationRecommendation[];
  /** Recommendation ids whose action was applied. */
  applied: string[];
  analyzedAt: string;
}

/** Combined recent history for the performance dashboard. */
export interface PerformanceHistory {
  profiles: ProfileSample[];
  benchmarks: BenchmarkResult[];
  startups: StartupProfile[];
}
