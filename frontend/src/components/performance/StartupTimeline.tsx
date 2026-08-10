import { useState } from "react";
import { BarChart3 } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { StartupProfile, StartupStage } from "@/types/performance";
import { BarList } from "@/components/performance/PerformanceCharts";
import { formatMs } from "@/utils/format";

/**
 * Startup timeline: every persisted launch phase rendered as a
 * proportional horizontal bar, with the slowest stage highlighted.
 * Selecting a bar pins its detail line below.
 */
export function StartupTimeline({
  profiles,
  loading,
  error,
}: {
  profiles: StartupProfile[];
  loading: boolean;
  error: string | null;
}) {
  const [pinned, setPinned] = useState<StartupStage | null>(null);
  const latest = profiles[0] ?? null;

  const items = (latest?.stages ?? []).map((stage) => ({
    label: stage.label,
    value: stage.durationMs,
    sublabel: stage.name,
  }));

  const slowest = latest ? [...latest.stages].sort((a, b) => b.durationMs - a.durationMs)[0] : null;
  const total = latest?.totalMs ?? 0;

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between">
        <div>
          <CardTitle className="flex items-center gap-2">
            <BarChart3 className="h-4 w-4 text-(--color-muted-foreground)" />
            Startup Timeline
          </CardTitle>
          <CardDescription>
            {latest
              ? `Last launch: ${total.toLocaleString()} ms across ${latest.stages.length} stages`
              : "No startup profile recorded yet"}
          </CardDescription>
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {loading ? (
          <p className="text-sm text-(--color-muted-foreground)">Loading startup timeline…</p>
        ) : error ? (
          <p className="text-sm text-(--color-danger)">{error}</p>
        ) : items.length === 0 ? (
          <p className="text-sm text-(--color-muted-foreground)">
            Launch ChronoDesk once to populate the startup timeline.
          </p>
        ) : (
          <>
            <BarList
              items={items}
              valueFormatter={formatMs}
              color="var(--color-accent-soft)"
              onSelect={(index) => setPinned((latest?.stages[index] ?? null))}
            />
            {slowest && (
              <p className="text-xs text-(--color-muted-foreground)">
                Slowest stage: <span className="text-(--color-warning)">{slowest.label}</span> at{" "}
                {formatMs(slowest.durationMs)}
              </p>
            )}
            {pinned && (
              <p className="border-t border-(--color-border-subtle) pt-2 text-xs text-(--color-muted-foreground)">
                <span className="font-medium text-(--color-foreground)">{pinned.label}</span>{" "}
                ({pinned.name}) took {formatMs(pinned.durationMs)}.
              </p>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}