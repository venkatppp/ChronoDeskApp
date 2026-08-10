import { Activity, AlertTriangle, CheckCircle2, ShieldCheck } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { ProgressRing } from "@/components/ui/ProgressRing";
import type { HealthSnapshot, HealthStatus, WorkerHealth, WorkerStatus } from "@/types/recovery";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

const STATUS_BADGE: Record<WorkerStatus, { label: string; variant: "success" | "warning" | "accent" | "neutral" }> = {
  healthy: { label: "healthy", variant: "success" },
  stalled: { label: "stalled", variant: "warning" },
  failed: { label: "failed", variant: "accent" },
  idle: { label: "idle", variant: "neutral" },
};

const STATUS_META: Record<HealthStatus, { label: string; variant: "success" | "warning" | "accent" }> = {
  healthy: { label: "Healthy", variant: "success" },
  degraded: { label: "Degraded", variant: "warning" },
  critical: { label: "Critical", variant: "accent" },
};

/**
 * Health dashboard (RC-10 M2): the aggregate health score ring, the
 * runtime status badge, every monitored worker's liveness row, and the
 * issues the health monitor found.
 */
export function HealthDashboard({ snapshot, loading, error }: { snapshot: HealthSnapshot | null; loading: boolean; error: string | null }) {
  if (loading) {
    return <p className="text-sm text-(--color-muted-foreground)">Loading health snapshot…</p>;
  }
  if (error) {
    return <p className="text-sm text-(--color-danger)">{error}</p>;
  }
  if (!snapshot) {
    return <p className="text-sm text-(--color-muted-foreground)">No health snapshot yet.</p>;
  }

  const meta = STATUS_META[snapshot.status];
  return (
    <div className="grid items-start gap-4 xl:grid-cols-2">
      <div className="flex min-w-0 flex-col gap-4">
      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-4 w-4 text-(--color-muted-foreground)" />
              Runtime Health
            </CardTitle>
            <CardDescription>
              Watchdog-derived aggregate health of the ChronoDesk runtime.
            </CardDescription>
          </div>
          <Badge variant={meta.variant}>{meta.label}</Badge>
        </CardHeader>
        <CardContent className="flex items-center gap-5">
          <ProgressRing value={snapshot.overallScore} size={72} strokeWidth={6} />
          <div className="flex flex-col gap-1 text-sm">
            <p className="text-(--color-muted-foreground)">
              <Activity className="mr-1 inline h-3.5 w-3.5" />
              Score {snapshot.overallScore.toFixed(0)}/100 · {snapshot.workers.length} worker
              {snapshot.workers.length === 1 ? "" : "s"} monitored
            </p>
            <p className="text-xs text-(--color-muted-foreground)">
              Captured {formatRelativeTime(snapshot.capturedAt)}
            </p>
          </div>
        </CardContent>
      </Card>

      {snapshot.issues.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-(--color-warning)" />
              Open Issues
            </CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-2">
            {snapshot.issues.map((issue) => (
              <p key={issue} className="text-sm text-(--color-warning)">
                {issue}
              </p>
            ))}
          </CardContent>
        </Card>
      )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CheckCircle2 className="h-4 w-4 text-(--color-muted-foreground)" />
            Monitored Workers
          </CardTitle>
          <CardDescription>
            Background workers and the runtime's own liveness marker, as seen by the watchdog.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {snapshot.workers.length === 0 && (
            <p className="text-sm text-(--color-muted-foreground)">No workers registered yet.</p>
          )}
          {snapshot.workers.map((worker: WorkerHealth) => {
            const badge = STATUS_BADGE[worker.status];
            return (
              <div key={worker.worker} className="flex items-center justify-between gap-3">
                <div className="flex min-w-0 flex-col">
                  <p className="truncate font-(family-name:--font-mono) text-sm text-(--color-foreground)">{worker.worker}</p>
                  <p className="text-xs text-(--color-muted-foreground)">
                    heartbeat {formatRelativeTime(worker.lastHeartbeat)}
                    {worker.consecutiveMisses > 0 ? ` · ${worker.consecutiveMisses} missed` : ""}
                  </p>
                </div>
                <Badge variant={badge.variant}>{badge.label}</Badge>
              </div>
            );
          })}
        </CardContent>
      </Card>
    </div>
  );
}
