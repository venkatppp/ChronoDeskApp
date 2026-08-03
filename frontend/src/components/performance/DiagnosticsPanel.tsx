import { useState } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Cpu,
  Database,
  Lightbulb,
  Loader2,
  MemoryStick,
  RefreshCw,
  Sparkles,
  Users,
  Zap,
} from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type {
  DiagnosticsSnapshot,
  OptimizationRecommendation,
  OptimizeResult,
} from "@/types/performance";
import { ProgressBar, StatCard } from "@/components/performance/PerformanceCharts";
import { formatBytes, formatMs } from "@/utils/format";

const CATEGORY_LABELS: Record<string, string> = {
  query: "Query optimization",
  lazy_init: "Lazy initialization",
  worker: "Background worker",
  cache: "Cache",
  memory: "Memory",
};

/**
 * Diagnostics panel: machine + app snapshot (CPU, RAM, DB size, cache,
 * workers, threads) plus the optimizer's recommendations with
 * one-click remediation for safe actions.
 */
export function DiagnosticsPanel({
  diagnostics,
  loading,
  error,
  onRefresh,
  optimizer,
  optimizing,
  onOptimize,
}: {
  diagnostics: DiagnosticsSnapshot | null;
  loading: boolean;
  error: string | null;
  onRefresh: () => void;
  optimizer: OptimizeResult | null;
  optimizing: boolean;
  onOptimize: (apply: boolean) => void;
}) {
  const [appliedIds, setAppliedIds] = useState<Set<string>>(new Set());

  const handleApply = (recommendation: OptimizationRecommendation) => {
    onOptimize(true);
    setAppliedIds((prev) => new Set(prev).add(recommendation.id));
  };

  const memoryTone = diagnostics && diagnostics.memory.percent >= 85 ? "danger" : diagnostics && diagnostics.memory.percent >= 70 ? "warning" : "success";
  const cpuTone = diagnostics && diagnostics.cpu.usagePercent >= 85 ? "danger" : diagnostics && diagnostics.cpu.usagePercent >= 60 ? "warning" : "success";

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-(--color-accent)" />
              System Diagnostics
            </CardTitle>
            <CardDescription>
              {diagnostics
                ? `Captured ${new Date(diagnostics.capturedAt).toLocaleTimeString()} — machine + application snapshot`
                : "Live CPU, RAM, database, cache, worker, and thread status"}
            </CardDescription>
          </div>
          <button
            type="button"
            onClick={onRefresh}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2.5 py-1 text-xs text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} />
            Refresh
          </button>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          {error && <p className="text-sm text-(--color-danger)">{error}</p>}
          {!diagnostics && !error && (
            <p className="text-sm text-(--color-muted-foreground)">
              {loading ? "Gathering diagnostics…" : "No diagnostics captured yet."}
            </p>
          )}
          {diagnostics && (
            <>
              <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
                <StatCard
                  label="CPU"
                  value={`${diagnostics.cpu.usagePercent.toFixed(0)}%`}
                  sublabel={`${diagnostics.cpu.cores} cores / ${diagnostics.cpu.cpuParallelism} physical`}
                  tone={cpuTone}
                />
                <StatCard
                  label="Memory"
                  value={`${diagnostics.memory.percent.toFixed(0)}%`}
                  sublabel={`${formatBytes(diagnostics.memory.usedBytes)} / ${formatBytes(diagnostics.memory.totalBytes)}`}
                  tone={memoryTone}
                />
                <StatCard
                  label="Database"
                  value={formatBytes(diagnostics.db.sizeBytes)}
                  sublabel={diagnostics.db.path.split("/").pop()}
                />
                <StatCard
                  label="Cache"
                  value={`${diagnostics.cache.graphCacheEntries + diagnostics.cache.runtimeEntries} entries`}
                  sublabel={`hit rate ${(diagnostics.cache.runtimeHitRate * 100).toFixed(0)}% · ${formatBytes(diagnostics.cache.graphCacheSizeBytes)} graph payload`}
                />
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                <div className="flex flex-col gap-3 rounded-[var(--radius-card)] border border-(--color-border-subtle) p-4">
                  <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                    <Cpu className="h-3.5 w-3.5" /> CPU utilization
                    <span className="ml-auto tabular-nums text-(--color-muted-foreground)">
                      {diagnostics.cpu.usagePercent.toFixed(1)}%
                    </span>
                  </p>
                  <ProgressBar percent={diagnostics.cpu.usagePercent} tone={cpuTone} />
                  <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                    <MemoryStick className="h-3.5 w-3.5" /> Memory utilization
                    <span className="ml-auto tabular-nums text-(--color-muted-foreground)">
                      {diagnostics.memory.percent.toFixed(1)}%
                    </span>
                  </p>
                  <ProgressBar percent={diagnostics.memory.percent} tone={memoryTone} />
                  <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                    <Database className="h-3.5 w-3.5" /> Graph query cache
                    <span className="ml-auto tabular-nums text-(--color-muted-foreground)">
                      {formatBytes(diagnostics.cache.graphCacheSizeBytes)}
                    </span>
                  </p>
                  <ProgressBar
                    percent={
                      diagnostics.cache.graphCacheEntries > 0
                        ? Math.min(100, diagnostics.cache.graphCacheEntries / 2000)
                        : 0
                    }
                  />
                </div>

                <div className="flex flex-col gap-2 rounded-[var(--radius-card)] border border-(--color-border-subtle) p-4">
                  <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                    <Users className="h-3.5 w-3.5" /> Processes & threads
                  </p>
                  <p className="text-sm text-(--color-muted-foreground)">
                    <span className="tabular-nums text-(--color-foreground)">{diagnostics.threads.processCount}</span>{" "}
                    processes ·{" "}
                    <span className="tabular-nums text-(--color-foreground)">{diagnostics.threads.totalThreads}</span>{" "}
                    threads (0 on macOS — platform limitation)
                  </p>
                  <p className="mt-2 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                    <Zap className="h-3.5 w-3.5" /> Background workers
                  </p>
                  {diagnostics.workers.length === 0 ? (
                    <p className="text-sm text-(--color-muted-foreground)">No worker telemetry yet.</p>
                  ) : (
                    <div className="flex flex-col gap-1.5">
                      {diagnostics.workers.map((worker) => (
                        <div key={worker.name} className="flex items-center gap-2 text-xs">
                          <span
                            className={
                              worker.errorCount > 0
                                ? "h-1.5 w-1.5 rounded-full bg-(--color-danger)"
                                : "h-1.5 w-1.5 rounded-full bg-(--color-success)"
                            }
                          />
                          <span className="flex-1 truncate text-(--color-foreground)">{worker.name}</span>
                          <span className="tabular-nums text-(--color-muted-foreground)">
                            {worker.executionCount} runs
                          </span>
                          <span className="tabular-nums text-(--color-muted-foreground)">
                            {formatMs(worker.avgExecutionTimeMs)} avg
                          </span>
                          {worker.errorCount > 0 && (
                            <span className="text-(--color-danger)">{worker.errorCount} errors</span>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-start justify-between gap-4">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Lightbulb className="h-4 w-4 text-(--color-warning)" />
              Optimization Recommendations
            </CardTitle>
            <CardDescription>
              The optimizer analyzes the profile, startup, cache, and system figures for actionable findings.
            </CardDescription>
          </div>
          <div className="flex shrink-0 gap-1.5">
            <button
              type="button"
              disabled={optimizing}
              onClick={() => {
                setAppliedIds(new Set());
                onOptimize(false);
              }}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2.5 py-1 text-xs text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Analyze
            </button>
            <button
              type="button"
              disabled={optimizing}
              onClick={() => {
                setAppliedIds(new Set());
                onOptimize(true);
              }}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] bg-(--color-accent) px-2.5 py-1 text-xs font-medium text-(--color-accent-foreground)"
            >
              {optimizing ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Zap className="h-3.5 w-3.5" />}
              Analyze & apply
            </button>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {optimizing && (
            <p className="text-sm text-(--color-muted-foreground)">Running optimizer analysis…</p>
          )}
          {!optimizer && !optimizing && (
            <p className="text-sm text-(--color-muted-foreground)">
              Run an analysis to see findings across query, startup, worker, cache, and memory surfaces.
            </p>
          )}
          {optimizer && optimizer.recommendations.length === 0 && (
            <p className="flex items-center gap-2 text-sm text-(--color-success)">
              <CheckCircle2 className="h-4 w-4" /> No optimization opportunities detected.
            </p>
          )}
          {optimizer?.recommendations.map((recommendation) => {
            const applied = optimizer.applied.includes(recommendation.id) || appliedIds.has(recommendation.id);
            return (
              <div
                key={recommendation.id}
                className="flex items-start gap-3 rounded-[var(--radius-card)] border border-(--color-border-subtle) p-3"
              >
                <AlertTriangle
                  className={
                    recommendation.severity === "critical"
                      ? "mt-0.5 h-4 w-4 shrink-0 text-(--color-danger)"
                      : recommendation.severity === "warning"
                        ? "mt-0.5 h-4 w-4 shrink-0 text-(--color-warning)"
                        : "mt-0.5 h-4 w-4 shrink-0 text-(--color-accent)"
                  }
                />
                <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-sm font-medium">{recommendation.title}</span>
                    <span
                      className={`rounded-full px-1.5 py-0.5 text-[10px] font-medium ${
                        recommendation.severity === "critical"
                          ? "bg-(--color-danger) text-(--color-accent-foreground)"
                          : recommendation.severity === "warning"
                            ? "bg-(--color-warning) text-(--color-warning-foreground)"
                            : "bg-(--color-border-subtle) text-(--color-muted-foreground)"
                      }`}
                    >
                      {CATEGORY_LABELS[recommendation.category] ?? recommendation.category}
                    </span>
                    {applied && (
                      <span className="rounded-full bg-(--color-success) px-1.5 py-0.5 text-[10px] font-medium text-(--color-accent-foreground)">
                        applied
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-(--color-muted-foreground)">{recommendation.detail}</p>
                </div>
                {recommendation.action && !applied && (
                  <button
                    type="button"
                    disabled={optimizing}
                    onClick={() => handleApply(recommendation)}
                    className="shrink-0 rounded-[var(--radius-control)] bg-(--color-accent) px-2.5 py-1 text-xs font-medium text-(--color-accent-foreground)"
                  >
                    Apply
                  </button>
                )}
              </div>
            );
          })}
        </CardContent>
      </Card>
    </div>
  );
}