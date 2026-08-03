import { useCallback, useEffect, useState } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Database,
  Gauge,
  GitBranch,
  HeartPulse,
  ListChecks,
  Loader2,
  MemoryStick,
  Network,
  RefreshCw,
  ShieldCheck,
  Timer,
  Trash2,
  XCircle,
} from "lucide-react";
import { VirtualizedNodeList } from "@/features/graph/components/VirtualizedNodeList";
import { getGraphOptimizationRepository } from "@/services/graphOptimizationRepository";
import type {
  BenchmarkSuiteResult,
  ConsistencyReport,
  GraphDiagnostics,
  IntegrityCheckResult,
  OrphanCleanupResult,
  OrphanSummary,
  RepairResult,
} from "@/types/graphOptimization";
import type { GraphNodeType, KgNode, TypeCount } from "@/types/graph";

const TYPE_COLORS: Record<GraphNodeType, string> = {
  workspace: "var(--color-accent)",
  file: "var(--color-success)",
  planner_report: "var(--color-warning)",
  execution: "var(--color-danger)",
  memory_record: "var(--color-accent-muted)",
  autonomous_session: "var(--color-warning-foreground)",
};

const ALL_TYPES: GraphNodeType[] = [
  "workspace",
  "file",
  "planner_report",
  "execution",
  "memory_record",
  "autonomous_session",
];

const NODE_FILTERS: { value: GraphNodeType[] | "all"; label: string }[] = [
  { value: "all", label: "All" },
  { value: ["file"], label: "Files" },
  { value: ["memory_record"], label: "Memory" },
  { value: ["execution"], label: "Executions" },
];

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function GraphPerformancePage() {
  const repo = getGraphOptimizationRepository();
  const [diagnostics, setDiagnostics] = useState<GraphDiagnostics | null>(null);
  const [loadingDiagnostics, setLoadingDiagnostics] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const [nodes, setNodes] = useState<KgNode[]>([]);
  const [nodesTotal, setNodesTotal] = useState(0);
  const [loadingNodes, setLoadingNodes] = useState(false);
  const [selectedNode, setSelectedNode] = useState<KgNode | null>(null);
  const [nodeFilter, setNodeFilter] = useState<GraphNodeType[] | "all">("all");

  const [lastIntegrity, setLastIntegrity] = useState<IntegrityCheckResult | null>(null);
  const [lastRepair, setLastRepair] = useState<RepairResult | null>(null);
  const [orphans, setOrphans] = useState<OrphanSummary | null>(null);
  const [lastCleanup, setLastCleanup] = useState<OrphanCleanupResult | null>(null);
  const [consistency, setConsistency] = useState<ConsistencyReport | null>(null);
  const [lastSuite, setLastSuite] = useState<BenchmarkSuiteResult | null>(null);

  const refreshDiagnostics = useCallback(async () => {
    setLoadingDiagnostics(true);
    setError(null);
    try {
      setDiagnostics(await repo.graphDiagnostics());
    } catch (err) {
      console.error("Failed to load graph diagnostics:", err);
      setError("Failed to load graph diagnostics. Please try again.");
    } finally {
      setLoadingDiagnostics(false);
    }
  }, [repo]);

  useEffect(() => {
    refreshDiagnostics();
  }, [refreshDiagnostics]);

  const loadNodesPage = useCallback(
    async (offset: number) => {
      setLoadingNodes(true);
      try {
        const types = nodeFilter === "all" ? ALL_TYPES : nodeFilter;
        const page = await repo.graphNodesPage(types, undefined, offset, 100);
        setNodes((prev) => (offset === 0 ? page.nodes : [...prev, ...page.nodes]));
        setNodesTotal(page.total);
      } catch (err) {
        console.error("Failed to load node page:", err);
      } finally {
        setLoadingNodes(false);
      }
    },
    [repo, nodeFilter],
  );

  useEffect(() => {
    setNodes([]);
    loadNodesPage(0);
  }, [nodeFilter, loadNodesPage]);

  const runAction = async (key: string, action: () => Promise<void>) => {
    setBusy(key);
    setError(null);
    try {
      await action();
    } catch (err) {
      console.error(`Action ${key} failed:`, err);
      setError("Operation failed. Please try again.");
    } finally {
      setBusy(null);
      await refreshDiagnostics();
    }
  };

  const handleIntegrity = () =>
    runAction("integrity", async () => setLastIntegrity(await repo.graphIntegrityCheck()));

  const handleRepair = () =>
    runAction("repair", async () => setLastRepair(await repo.graphRepair()));

  const handleOrphanSummary = () =>
    runAction("orphans", async () => setOrphans(await repo.graphOrphanSummary()));

  const handleOrphanCleanup = () =>
    runAction("cleanup", async () => setLastCleanup(await repo.graphOrphanCleanup()));

  const handleConsistency = () =>
    runAction("consistency", async () => setConsistency(await repo.graphConsistencyReport()));

  const handleBenchmark = () =>
    runAction("benchmark", async () => setLastSuite(await repo.graphBenchmarkSuite()));

  const handleTrimCache = () => runAction("trim", async () => { await repo.graphCacheTrim(50); });

  const handleClearExpired = () => runAction("sweep", async () => { await repo.graphClearExpiredCache(); });

  const memory = diagnostics?.memory;
  const integrity = lastIntegrity ?? diagnostics?.integrity;
  const metrics = diagnostics?.recentMetrics ?? [];

  return (
    <div className="mx-auto h-[calc(100vh-64px)] overflow-y-auto">
      <div className="mx-auto max-w-6xl px-6 py-5">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h1 className="font-(family-name:--font-display) text-xl font-bold">
              Graph Performance &amp; Scale
            </h1>
            <p className="text-sm text-(--color-muted-foreground)">
              Pagination, virtualization, integrity, repair, benchmarks, and operational health.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleBenchmark}
              disabled={busy !== null}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] bg-(--color-accent) px-3 py-1.5 text-xs font-medium text-(--color-accent-foreground) transition-opacity disabled:opacity-50"
            >
              <Timer className="h-3.5 w-3.5" strokeWidth={1.75} />
              Run benchmark suite
            </button>
            <button
              onClick={refreshDiagnostics}
              disabled={loadingDiagnostics}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1.5 text-xs font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loadingDiagnostics ? "animate-spin" : ""}`} strokeWidth={1.75} />
              Refresh
            </button>
          </div>
        </div>

        {error && (
          <p className="mt-3 rounded-[var(--radius-control)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-3 py-2 text-xs text-(--color-danger)">
            {error}
          </p>
        )}

        {loadingDiagnostics && !diagnostics ? (
          <div className="mt-16 flex flex-col items-center gap-3">
            <Loader2 className="h-6 w-6 animate-spin text-(--color-accent)" strokeWidth={1.75} />
            <p className="text-sm text-(--color-muted-foreground)">Running diagnostics…</p>
          </div>
        ) : (
          <div className="mt-5 flex flex-col gap-5">
            {/* Memory & cache statistics */}
            <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                  <MemoryStick className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                  Memory &amp; cache statistics
                </h2>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleTrimCache}
                    disabled={busy !== null}
                    className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                  >
                    Trim 50 oldest
                  </button>
                  <button
                    onClick={handleClearExpired}
                    disabled={busy !== null}
                    className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                  >
                    <Trash2 className="h-3 w-3" strokeWidth={1.75} />
                    Clear expired
                  </button>
                </div>
              </div>
              {memory && (
                <div className="grid grid-cols-2 gap-2 md:grid-cols-5">
                  {[
                    { label: "Nodes", value: memory.nodeCount, icon: Network },
                    { label: "Edges", value: memory.edgeCount, icon: GitBranch },
                    { label: "Cache entries", value: memory.cacheEntries, icon: Database },
                    { label: "Cache payload", value: formatBytes(memory.cacheSizeBytes), icon: Database },
                    { label: "Est. memory", value: formatBytes(memory.estimatedBytes), icon: MemoryStick },
                  ].map((stat) => (
                    <div key={stat.label} className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2">
                      <p className="text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">{stat.label}</p>
                      <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">{stat.value}</p>
                    </div>
                  ))}
                </div>
              )}
            </section>

            {/* Integrity panel */}
            <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                  <ShieldCheck className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                  Integrity panel
                </h2>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleIntegrity}
                    disabled={busy !== null}
                    className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                  >
                    <ListChecks className="h-3 w-3" strokeWidth={1.75} />
                    Run integrity check
                  </button>
                  <button
                    onClick={handleRepair}
                    disabled={busy !== null}
                    className="rounded-[var(--radius-control)] bg-(--color-warning) px-2.5 py-1 text-[10px] font-medium text-(--color-warning-foreground) transition-opacity disabled:opacity-50"
                  >
                    Repair issues
                  </button>
                </div>
              </div>
              {integrity && (
                <>
                  <div className="mb-2 flex flex-wrap gap-1.5">
                    {integrity.issueTypeCounts.length === 0 && (
                      <span className="inline-flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium text-(--color-success)">
                        <CheckCircle2 className="h-3 w-3" strokeWidth={1.75} />
                        No open issues
                      </span>
                    )}
                    {integrity.issueTypeCounts.map((count: TypeCount) => (
                      <span
                        key={count.name}
                        className="inline-flex items-center gap-1 rounded bg-(--color-danger)/10 px-2 py-1 text-[10px] font-medium text-(--color-danger)"
                      >
                        <AlertTriangle className="h-3 w-3" strokeWidth={1.75} />
                        {count.name.replace("_", " ")} · {count.count}
                      </span>
                    ))}
                  </div>
                  <div className="flex flex-col gap-1">
                    {integrity.issues.slice(0, 8).map((issue) => (
                      <div key={issue.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                        <span
                          className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                            issue.severity === "critical"
                              ? "bg-(--color-danger)"
                              : issue.severity === "warning"
                                ? "bg-(--color-warning)"
                                : "bg-(--color-accent-muted)"
                          }`}
                        />
                        <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{issue.detail}</span>
                        <span className="shrink-0 text-[9px] text-(--color-faint-foreground)">
                          {issue.issueType.replace("_", " ")}
                        </span>
                      </div>
                    ))}
                  </div>
                </>
              )}
              {lastRepair && (
                <p className="mt-2 rounded-[var(--radius-control)] bg-(--color-success)/10 px-3 py-2 text-[10px] text-(--color-success)">
                  Repair removed {lastRepair.orphanEdgesRemoved} orphan edges, {lastRepair.danglingWorkspacesRemoved}{" "}
                  dangling nodes, fixed {lastRepair.malformedNodesFixed} malformed, clamped{" "}
                  {lastRepair.invalidConfidenceFixed} edges · {lastRepair.issuesResolved} issues resolved
                </p>
              )}
            </section>

            {/* Orphans + consistency */}
            <div className="grid gap-5 lg:grid-cols-2">
              <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
                <div className="mb-3 flex items-center justify-between gap-2">
                  <h2 className="flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                    <Trash2 className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />
                    Orphans
                  </h2>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleOrphanSummary}
                      disabled={busy !== null}
                      className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                    >
                      Detect
                    </button>
                    <button
                      onClick={handleOrphanCleanup}
                      disabled={busy !== null}
                      className="rounded-[var(--radius-control)] bg-(--color-danger) px-2.5 py-1 text-[10px] font-medium text-(--color-danger-foreground) transition-opacity disabled:opacity-50"
                    >
                      Clean up
                    </button>
                  </div>
                </div>
                {orphans && (
                  <div className="grid grid-cols-2 gap-2">
                    <div className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2">
                      <p className="text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Orphan edges</p>
                      <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">{orphans?.orphanEdges ?? 0}</p>
                    </div>
                    <div className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2">
                      <p className="text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Dangling workspaces</p>
                      <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">{orphans?.danglingWorkspaces ?? 0}</p>
                    </div>
                  </div>
                )}
                {lastCleanup && (
                  <p className="mt-2 rounded-[var(--radius-control)] bg-(--color-success)/10 px-3 py-2 text-[10px] text-(--color-success)">
                    Cleanup removed {lastCleanup.orphanEdgesRemoved} edges and {lastCleanup.danglingWorkspacesRemoved} nodes ·{" "}
                    {lastCleanup.issuesResolved} issues resolved
                  </p>
                )}
              </section>

              <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
                <div className="mb-3 flex items-center justify-between gap-2">
                  <h2 className="flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                    <HeartPulse className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                    Consistency
                  </h2>
                  <button
                    onClick={handleConsistency}
                    disabled={busy !== null}
                    className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                  >
                    Verify
                  </button>
                </div>
                {(consistency ?? diagnostics?.consistency) && (
                  <div className="flex flex-col gap-1">
                    {(consistency ?? diagnostics!.consistency).checks.map((check) => (
                      <div key={check.name} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                        {check.passed ? (
                          <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-(--color-success)" strokeWidth={1.75} />
                        ) : (
                          <XCircle className="h-3.5 w-3.5 shrink-0 text-(--color-danger)" strokeWidth={1.75} />
                        )}
                        <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{check.name}</span>
                        <span className="shrink-0 truncate text-[9px] text-(--color-faint-foreground)" title={check.detail}>
                          {check.detail}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </section>
            </div>

            {/* Benchmark viewer */}
            <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <h2 className="mb-3 flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                <Gauge className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                Benchmark viewer
              </h2>
              {lastSuite && (
                <div className="mb-3 flex flex-col gap-1">
                  {lastSuite.benchmarks.map((benchmark) => (
                    <div key={benchmark.name} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                      <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{benchmark.name}</span>
                      <span className="shrink-0 font-(family-name:--font-mono) text-[10px] text-(--color-muted-foreground)">
                        {benchmark.durationMs} ms
                      </span>
                      {benchmark.throughputPerSec != null && (
                        <span className="shrink-0 font-(family-name:--font-mono) text-[10px] text-(--color-faint-foreground)">
                          {benchmark.throughputPerSec}/s
                        </span>
                      )}
                    </div>
                  ))}
                  <p className="rounded-[var(--radius-control)] bg-(--color-surface-hover) px-3 py-2 text-[10px] text-(--color-faint-foreground)">
                    Suite {lastSuite.suiteName} · {lastSuite.totalDurationMs} ms total · {lastSuite.benchmarks.length} benchmarks
                  </p>
                </div>
              )}
              {(diagnostics?.recentBenchmarks ?? []).length > 0 && (
                <div className="flex flex-col gap-1">
                  <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Recent runs</p>
                  {(diagnostics?.recentBenchmarks ?? []).slice(0, 8).map((benchmark) => (
                    <div key={`${benchmark.suiteName}-${benchmark.name}-${benchmark.createdAt}`} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                      <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{benchmark.name}</span>
                      <span className="shrink-0 font-(family-name:--font-mono) text-[10px] text-(--color-muted-foreground)">{benchmark.durationMs} ms</span>
                      <span className="shrink-0 text-[9px] text-(--color-faint-foreground)">{benchmark.suiteName}</span>
                    </div>
                  ))}
                </div>
              )}
            </section>

            {/* Query metrics */}
            <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <h2 className="mb-3 flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                <Activity className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                Query metrics
              </h2>
              <div className="flex flex-col gap-1">
                {metrics.length === 0 && (
                  <p className="px-2 py-1.5 text-xs text-(--color-faint-foreground)">
                    No operations tracked yet — run a search, page, or traversal.
                  </p>
                )}
                {metrics.slice(0, 10).map((metric) => (
                  <div key={metric.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                    <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">
                      {metric.operation}
                      {metric.query ? ` · "${metric.query}"` : ""}
                    </span>
                    {metric.hitCache && (
                      <span className="shrink-0 rounded bg-(--color-accent)/10 px-1.5 py-0.5 text-[9px] font-medium text-(--color-accent)">
                        cached
                      </span>
                    )}
                    <span className="shrink-0 font-(family-name:--font-mono) text-[10px] text-(--color-muted-foreground)">
                      {metric.durationMs} ms
                    </span>
                    <span className="shrink-0 font-(family-name:--font-mono) text-[10px] text-(--color-faint-foreground)">
                      {metric.rowsReturned} rows
                    </span>
                  </div>
                ))}
              </div>
            </section>

            {/* Maintenance history */}
            <section className="rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <h2 className="mb-3 flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                <Database className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                Maintenance history
              </h2>
              <div className="flex flex-col gap-1">
                {(diagnostics?.recentMaintenance ?? []).map((run) => (
                  <div key={run.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5">
                    <span
                      className={`shrink-0 rounded px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide ${
                        run.status === "completed"
                          ? "bg-(--color-success)/10 text-(--color-success)"
                          : "bg-(--color-danger)/10 text-(--color-danger)"
                      }`}
                    >
                      {run.runType.replace("_", " ")}
                    </span>
                    <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">
                      {new Date(run.startedAt).toLocaleString()}
                    </span>
                    <span className="shrink-0 text-[9px] text-(--color-faint-foreground)">
                      {`${run.issuesFound} found · ${run.issuesResolved} resolved · ${run.durationMs} ms`}
                    </span>
                  </div>
                ))}
              </div>
            </section>

            {/* Virtualized node browser */}
            <section className="flex min-h-96 flex-col rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-4">
              <div className="mb-3 flex items-center justify-between gap-2">
                <h2 className="flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground)">
                  <Network className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
                  Virtualized node browser
                </h2>
                <div className="flex items-center gap-1">
                  {NODE_FILTERS.map((filter) => (
                    <button
                      key={filter.label}
                      onClick={() => setNodeFilter(filter.value)}
                      className={`rounded-[var(--radius-control)] px-2.5 py-1 text-[10px] font-medium transition-colors ${
                        nodeFilter === filter.value
                          ? "bg-(--color-accent)/10 text-(--color-accent)"
                          : "text-(--color-muted-foreground) hover:bg-(--color-surface-hover)"
                      }`}
                    >
                      {filter.label}
                    </button>
                  ))}
                </div>
              </div>
              <div className="flex min-h-0 flex-1 gap-3">
                <div className="flex min-h-96 flex-1 flex-col">
                  <VirtualizedNodeList
                    nodes={nodes}
                    total={nodesTotal}
                    loading={loadingNodes}
                    onLoadMore={() => loadNodesPage(nodes.length)}
                    onSelect={setSelectedNode}
                    selectedId={selectedNode?.entityId}
                    typeColors={TYPE_COLORS}
                  />
                </div>
                {selectedNode && (
                  <div className="hidden w-56 shrink-0 flex-col gap-2 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) p-3 lg:flex">
                    <p className="truncate text-xs font-semibold text-(--color-foreground)">{selectedNode.title}</p>
                    <p className="text-[10px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                      {selectedNode.nodeType.replace("_", " ")}
                    </p>
                    {selectedNode.summary && (
                      <p className="rounded-[var(--radius-control)] bg-(--color-surface-hover) px-2 py-1.5 text-[10px] text-(--color-muted-foreground)">
                        {selectedNode.summary}
                      </p>
                    )}
                    <p className="text-[9px] text-(--color-faint-foreground)">
                      {selectedNode.entityId}
                    </p>
                  </div>
                )}
              </div>
            </section>
          </div>
        )}
      </div>
    </div>
  );
}
