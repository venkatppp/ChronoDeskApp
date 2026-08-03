import { useCallback, useEffect, useState } from "react";
import { Activity, FlaskConical, Gauge } from "lucide-react";
import { BenchmarkPanel } from "@/components/performance/BenchmarkPanel";
import { DiagnosticsPanel } from "@/components/performance/DiagnosticsPanel";
import { PerformanceDashboard } from "@/components/performance/PerformanceDashboard";
import { getPerformanceRepository } from "@/services/performanceRepository";
import type {
  BenchmarkCategory,
  BenchmarkSuiteResult,
  DiagnosticsSnapshot,
  OptimizeResult,
} from "@/types/performance";

type Tab = "dashboard" | "benchmarks" | "diagnostics";

const TABS: { value: Tab; label: string; icon: typeof Gauge }[] = [
  { value: "dashboard", label: "Dashboard", icon: Activity },
  { value: "benchmarks", label: "Benchmarks", icon: FlaskConical },
  { value: "diagnostics", label: "Diagnostics", icon: Gauge },
];

/**
 * Performance page (RC-10 M1): live profiling, benchmark suites,
 * startup timeline, system diagnostics, and optimization
 * recommendations behind the six `performance_*` IPC commands.
 */
export function PerformancePage() {
  const repo = getPerformanceRepository();
  const [tab, setTab] = useState<Tab>("dashboard");

  // Benchmarks
  const [benchmarkRunning, setBenchmarkRunning] = useState(false);
  const [benchmarkError, setBenchmarkError] = useState<string | null>(null);
  const [benchmarkResult, setBenchmarkResult] = useState<BenchmarkSuiteResult | null>(null);

  // Diagnostics
  const [diagnostics, setDiagnostics] = useState<DiagnosticsSnapshot | null>(null);
  const [diagnosticsLoading, setDiagnosticsLoading] = useState(false);
  const [diagnosticsError, setDiagnosticsError] = useState<string | null>(null);
  const [optimizer, setOptimizer] = useState<OptimizeResult | null>(null);
  const [optimizing, setOptimizing] = useState(false);

  const runBenchmark = useCallback(
    async (category?: BenchmarkCategory) => {
      setBenchmarkRunning(true);
      setBenchmarkError(null);
      try {
        const result = await repo.performanceBenchmark(category);
        setBenchmarkResult(result);
      } catch (err) {
        console.error("Failed to run benchmark:", err);
        setBenchmarkError("Failed to run the benchmark suite. Please try again.");
      } finally {
        setBenchmarkRunning(false);
      }
    },
    [repo],
  );

  const refreshDiagnostics = useCallback(async () => {
    setDiagnosticsLoading(true);
    setDiagnosticsError(null);
    try {
      setDiagnostics(await repo.performanceDiagnostics());
    } catch (err) {
      console.error("Failed to load diagnostics:", err);
      setDiagnosticsError("Failed to load diagnostics. Please try again.");
    } finally {
      setDiagnosticsLoading(false);
    }
  }, [repo]);

  const runOptimizer = useCallback(
    async (apply: boolean) => {
      setOptimizing(true);
      try {
        setOptimizer(await repo.performanceOptimize(apply));
      } catch (err) {
        console.error("Failed to run optimizer:", err);
        setDiagnosticsError("Failed to run the optimizer analysis. Please try again.");
      } finally {
        setOptimizing(false);
      }
    },
    [repo],
  );

  // Diagnostics load on demand: once when the tab first opens and on
  // every manual refresh.
  useEffect(() => {
    if (tab === "diagnostics" && !diagnostics && !diagnosticsLoading) {
      refreshDiagnostics();
    }
  }, [tab, diagnostics, diagnosticsLoading, refreshDiagnostics]);

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="font-(family-name:--font-display) text-xl font-bold tracking-tight">
          Performance
        </h1>
        <p className="text-sm text-(--color-muted-foreground)">
          Live profiling, benchmarks, startup timeline, diagnostics, and optimization for the ChronoDesk runtime.
        </p>
      </div>

      <div className="flex gap-1 border-b border-(--color-border-subtle)">
        {TABS.map((item) => (
          <button
            key={item.value}
            type="button"
            onClick={() => setTab(item.value)}
            className={
              tab === item.value
                ? "flex items-center gap-2 border-b-2 border-(--color-accent) px-3 pb-2 text-sm font-medium text-(--color-foreground)"
                : "flex items-center gap-2 border-b-2 border-transparent px-3 pb-2 text-sm text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </button>
        ))}
      </div>

      {tab === "dashboard" && <PerformanceDashboard />}
      {tab === "benchmarks" && (
        <BenchmarkPanel
          run={runBenchmark}
          running={benchmarkRunning}
          error={benchmarkError}
          result={benchmarkResult}
        />
      )}
      {tab === "diagnostics" && (
        <DiagnosticsPanel
          diagnostics={diagnostics}
          loading={diagnosticsLoading}
          error={diagnosticsError}
          onRefresh={refreshDiagnostics}
          optimizer={optimizer}
          optimizing={optimizing}
          onOptimize={runOptimizer}
        />
      )}
    </div>
  );
}