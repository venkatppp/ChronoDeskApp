import { History } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { RecoveryOutcome, RecoveryRun, RecoveryTrigger } from "@/types/recovery";
import { formatMs } from "@/utils/format";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

const OUTCOME_BADGE: Record<RecoveryOutcome, { label: string; variant: "success" | "warning" | "accent" | "neutral" }> = {
  recovered: { label: "recovered", variant: "success" },
  no_action: { label: "no action", variant: "neutral" },
  failed: { label: "failed", variant: "accent" },
  rolled_back: { label: "rolled back", variant: "warning" },
  partial: { label: "partial", variant: "warning" },
};

const TRIGGER_LABEL: Record<RecoveryTrigger, string> = {
  startup: "startup",
  crash: "crash",
  watchdog: "watchdog",
  rollback: "rollback",
  manual: "manual",
};

/**
 * Recovery history panel (RC-10 M2): every completed recovery run — its
 * trigger, outcome, the actions executed, and the jobs resumed or rolled
 * back — so every automatic intervention is auditable.
 */
export function HistoryPanel({ runs, loading, error }: { runs: RecoveryRun[]; loading: boolean; error: string | null }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <History className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          Recovery History
        </CardTitle>
        <CardDescription>
          Every completed recovery run — startup detection, watchdog passes, and manual interventions.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {loading ? (
          <p className="text-sm text-(--color-muted-foreground)">Loading recovery history…</p>
        ) : error ? (
          <p className="text-sm text-(--color-danger)">{error}</p>
        ) : runs.length === 0 ? (
          <p className="text-sm text-(--color-muted-foreground)">No recovery runs recorded yet.</p>
        ) : (
          runs.map((run) => {
            const badge = OUTCOME_BADGE[run.outcome];
            return (
              <div key={run.id} className="flex flex-col gap-1 border-b border-(--color-border-subtle) pb-3 last:border-0 last:pb-0">
                <div className="flex items-center justify-between gap-3">
                  <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">
                    {TRIGGER_LABEL[run.trigger]} <span className="text-(--color-muted-foreground)">#{run.id}</span>
                  </p>
                  <div className="flex items-center gap-2">
                    <Badge variant={badge.variant}>{badge.label}</Badge>
                    <span className="text-xs text-(--color-muted-foreground)">{formatMs(run.durationMs)}</span>
                  </div>
                </div>
                {run.actions.length > 0 && (
                  <p className="text-xs text-(--color-muted-foreground)">
                    actions: {run.actions.join(" → ")}
                  </p>
                )}
                {run.recoveredJobs.length > 0 && (
                  <p className="text-xs text-(--color-muted-foreground)">
                    jobs: <span className="font-(family-name:--font-mono)">{run.recoveredJobs.join(", ")}</span>
                  </p>
                )}
                {run.rolledBackTo !== null && (
                  <p className="text-xs text-(--color-warning)">rolled back to checkpoint #{run.rolledBackTo}</p>
                )}
                {run.errors.length > 0 && (
                  <p className="text-xs text-(--color-danger)">{run.errors.join("; ")}</p>
                )}
                <p className="text-xs text-(--color-muted-foreground)">{formatRelativeTime(run.completedAt)}</p>
              </div>
            );
          })
        )}
      </CardContent>
    </Card>
  );
}