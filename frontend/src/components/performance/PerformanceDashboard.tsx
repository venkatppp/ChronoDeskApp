import { useCallback, useEffect, useState } from "react";
import { Activity, History, RefreshCw } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { PerformanceHistory, ProfileSnapshot } from "@/types/performance";
import { BarList } from "@/components/performance/PerformanceCharts";
import { formatMs } from "@/utils/format";
import { StartupTimeline } from "@/components/performance/StartupTimeline";
import { getPerformanceRepository } from "@/services/performanceRepository";

const CATEGORY_COLORS: Record<string, string> = {
  command: "var(--color-accent-soft)",
  service: "var(--color-success)",
  repository: "var(--color-warning)",
  worker: "var(--color-danger)",
  engine: "var(--color-violet)",
};

function categoryColor(category: string): string {
  return CATEGORY_COLORS[category] ?? "var(--color-accent-soft)";
}

const REFRESH_INTERVAL_MS = 10_000;

/**
 * Live profiling dashboard: aggregates and recent samples for every
 * recorded command/service/repository/worker/engine operation,
 * auto-refreshing while the tab is visible, plus a history roll-up of
 * benchmarks and startup runs.
 */
export function PerformanceDashboard() {
  const repo = getPerformanceRepository();
  const [profile, setProfile] = useState<ProfileSnapshot | null>(null);
  const [history, setHistory] = useState<PerformanceHistory | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [live, setLive] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [nextProfile, nextHistory] = await Promise.all([
        repo.performanceProfile(),
        repo.performanceHistory(25),
      ]);
      setProfile(nextProfile);
      setHistory(nextHistory);
      setError(null);
    } catch (err) {
      console.error("Failed to load performance profile:", err);
      setError("Failed to load the performance profile. Please try again.");
    } finally {
      setLoading(false);
    }
  }, [repo]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (!live) return;
    const interval = setInterval(refresh, REFRESH_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [live, refresh]);

  const aggregates = profile?.aggregates ?? [];
  const slowest = profile?.slowest ?? [];

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
              Live Profiling
            </CardTitle>
            <CardDescription>
              {profile
                ? `${aggregates.length} operations measured · ${slowest.length} slowest below`
                : "Command, service, repository, and worker timings as they happen"}
            </CardDescription>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <label className="flex cursor-pointer items-center gap-2 text-xs text-(--color-muted-foreground)">
              <input
                type="checkbox"
                checked={live}
                onChange={(event) => setLive(event.target.checked)}
                className="h-3.5 w-3.5 accent-(--color-accent)"
              />
              Live refresh
            </label>
            <button
              type="button"
              onClick={refresh}
              disabled={loading}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2.5 py-1 text-xs text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin" : ""}`} strokeWidth={1.75} />
              Refresh
            </button>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          {error && <p className="text-sm text-(--color-danger)">{error}</p>}
          {!profile && !error && (
            <p className="text-sm text-(--color-muted-foreground)">
              {loading ? "Loading live profile…" : "No profile data yet."}
            </p>
          )}
          {profile && aggregates.length === 0 && (
            <p className="text-sm text-(--color-muted-foreground)">
              Nothing measured yet — performance commands and background work will appear here.
            </p>
          )}
          {profile && aggregates.length > 0 && (
            <div className="flex flex-col gap-3">
              {aggregates.slice(0, 12).map((aggregate) => (
                <div key={`${aggregate.category}:${aggregate.name}`} className="flex items-center gap-3">
                  <span
                    className="h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: categoryColor(aggregate.category) }}
                  />
                  <span className="w-44 shrink-0 truncate text-xs">
                    <span className="font-medium text-(--color-foreground)">{aggregate.name}</span>
                    <span className="ml-1.5 text-[10px] uppercase text-(--color-faint-foreground)">
                      {aggregate.category}
                    </span>
                  </span>
                  <span className="w-16 shrink-0 text-right text-xs tabular-nums text-(--color-muted-foreground)">
                    {aggregate.count}×
                  </span>
                  <span className="flex-1 text-xs tabular-nums text-(--color-muted-foreground)">
                    avg <span className="text-(--color-foreground)">{formatMs(aggregate.avgMs)}</span> · p95{" "}
                    <span className="text-(--color-warning)">{formatMs(aggregate.p95Ms)}</span> · max{" "}
                    {formatMs(aggregate.maxMs)}
                  </span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-(--color-danger)" strokeWidth={1.75} />
              Slowest Operations
            </CardTitle>
            <CardDescription>Top latency outliers from the live window</CardDescription>
          </CardHeader>
          <CardContent>
            {slowest.length === 0 ? (
              <p className="text-sm text-(--color-muted-foreground)">No slow operations recorded.</p>
            ) : (
              <BarList
                items={slowest.map((sample) => ({
                  label: sample.name,
                  value: sample.durationMs,
                  sublabel: sample.category,
                }))}
                valueFormatter={formatMs}
                color="var(--color-danger)"
              />
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <History className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
              Recent History
            </CardTitle>
            <CardDescription>Persisted samples, benchmark runs, and startup launches</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Benchmark runs ({history?.benchmarks.length ?? 0})
            </p>
            {history && history.benchmarks.length === 0 ? (
              <p className="text-xs text-(--color-muted-foreground)">No benchmark runs yet.</p>
            ) : (
              <div className="flex flex-col gap-1">
                {history?.benchmarks.slice(0, 6).map((benchmark) => (
                  <div key={`${benchmark.id}-${benchmark.name}`} className="flex items-center gap-2 text-xs">
                    <span
                      className={
                        benchmark.ok
                          ? "h-1.5 w-1.5 rounded-full bg-(--color-success)"
                          : "h-1.5 w-1.5 rounded-full bg-(--color-danger)"
                      }
                    />
                    <span className="w-40 truncate">{benchmark.name}</span>
                    <span className="text-[10px] uppercase text-(--color-faint-foreground)">
                      {benchmark.category}
                    </span>
                    <span className="ml-auto tabular-nums text-(--color-muted-foreground)">
                      {formatMs(benchmark.durationMs)}
                    </span>
                  </div>
                ))}
              </div>
            )}
            <p className="mt-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Profile samples ({history?.profiles.length ?? 0})
            </p>
            {history && history.profiles.length === 0 ? (
              <p className="text-xs text-(--color-muted-foreground)">No profile samples yet.</p>
            ) : (
              <div className="flex flex-col gap-1">
                {history?.profiles.slice(0, 6).map((sample) => (
                  <div key={`${sample.id}-${sample.name}`} className="flex items-center gap-2 text-xs">
                    <span className="w-40 truncate">{sample.name}</span>
                    <span className="text-[10px] uppercase text-(--color-faint-foreground)">
                      {sample.category}
                    </span>
                    <span className="ml-auto tabular-nums text-(--color-muted-foreground)">
                      {formatMs(sample.durationMs)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <StartupTimeline
        profiles={history?.startups ?? []}
        loading={loading}
        error={error}
      />
    </div>
  );
}