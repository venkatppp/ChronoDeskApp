import { useState } from "react";
import { CheckCircle2, FlaskConical, Loader2, XCircle } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { BenchmarkCategory, BenchmarkResult, BenchmarkSuiteResult } from "@/types/performance";
import { BarList } from "@/components/performance/PerformanceCharts";
import { formatMs } from "@/utils/format";

const CATEGORIES: { value: BenchmarkCategory | "all"; label: string }[] = [
  { value: "all", label: "All suites" },
  { value: "planner", label: "Planner" },
  { value: "execution", label: "Execution" },
  { value: "memory", label: "Memory" },
  { value: "graph", label: "Graph" },
  { value: "vector", label: "Vector" },
];

const CATEGORY_COLORS: Record<BenchmarkCategory, string> = {
  planner: "var(--color-accent-soft)",
  execution: "var(--color-success)",
  memory: "var(--color-warning)",
  graph: "var(--color-violet)",
  vector: "var(--color-cyan)",
};

function benchmarkColor(category: BenchmarkCategory): string {
  return CATEGORY_COLORS[category] ?? "var(--color-accent-soft)";
}

/**
 * Benchmark panel: runs one micro-benchmark suite (or all of them)
 * through `performance_benchmark` and renders the per-operation mean
 * latencies as proportional bars with throughput, iterations, and
 * success state per row.
 */
export function BenchmarkPanel({
  run,
  running,
  error,
  result,
}: {
  run: (category?: BenchmarkCategory) => void;
  running: boolean;
  error: string | null;
  result: BenchmarkSuiteResult | null;
}) {
  const [selected, setSelected] = useState<BenchmarkCategory | "all">("all");

  const benchmarks: BenchmarkResult[] = result?.benchmarks ?? [];

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-4">
        <div>
          <CardTitle className="flex items-center gap-2">
            <FlaskConical className="h-4 w-4 text-(--color-accent)" />
            Benchmark Engine
          </CardTitle>
          <CardDescription>
            {result
              ? `Suite "${result.suiteName}" — ${result.totalDurationMs.toLocaleString()} ms total (${benchmarks.length} benchmarks)`
              : "Run read-only micro-benchmarks of the planner, execution, memory, graph, and vector pipelines."}
          </CardDescription>
        </div>
        <div className="flex shrink-0 flex-wrap gap-1.5">
          {CATEGORIES.map((category) => (
            <button
              key={category.value}
              type="button"
              disabled={running}
              onClick={() => {
                setSelected(category.value);
                run(category.value === "all" ? undefined : category.value);
              }}
              className={
                selected === category.value
                  ? "glass-accent rounded-[var(--radius-control)] px-2.5 py-1 text-xs font-medium text-(--color-accent-foreground)"
                  : "rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2.5 py-1 text-xs text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
              }
            >
              {category.label}
            </button>
          ))}
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {running && (
          <p className="flex items-center gap-2 text-sm text-(--color-muted-foreground)">
            <Loader2 className="h-4 w-4 animate-spin text-(--color-accent)" />
            Running benchmark suite…
          </p>
        )}
        {error && <p className="text-sm text-(--color-danger)">{error}</p>}
        {!running && !error && benchmarks.length === 0 && (
          <p className="text-sm text-(--color-muted-foreground)">
            Nothing measured yet — pick a suite above.
          </p>
        )}
        {!running && benchmarks.length > 0 && (
          <div className="flex flex-col gap-3">
            <BarList
              items={benchmarks.map((benchmark) => ({
                label: benchmark.name,
                value: benchmark.durationMs,
                sublabel: benchmark.ok ? `${benchmark.operation}` : "skipped",
              }))}
              valueFormatter={formatMs}
              color="var(--color-accent-soft)"
            />
            <div className="flex flex-col divide-y divide-(--color-border-subtle) border-t border-(--color-border-subtle)">
              {benchmarks.map((benchmark) => (
                <div key={benchmark.name} className="flex items-center gap-2 py-2 text-xs">
                  {benchmark.ok ? (
                    <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-(--color-success)" />
                  ) : (
                    <XCircle className="h-3.5 w-3.5 shrink-0 text-(--color-danger)" />
                  )}
                  <span className="w-44 truncate font-medium" style={{ color: benchmarkColor(benchmark.category) }}>
                    {benchmark.name}
                  </span>
                  <span className="flex-1 truncate text-(--color-muted-foreground)">
                    {benchmark.ok
                      ? `${benchmark.iterations} iterations · ${benchmark.operation}`
                      : String(benchmark.payload ?? "skipped")}
                  </span>
                  <span className="w-24 text-right tabular-nums">{formatMs(benchmark.durationMs)}</span>
                  <span className="w-20 text-right tabular-nums text-(--color-muted-foreground)">
                    {benchmark.throughputPerSec != null
                      ? `${benchmark.throughputPerSec.toFixed(1)}/s`
                      : "—"}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}