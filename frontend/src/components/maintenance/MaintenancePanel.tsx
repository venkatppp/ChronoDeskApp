import { Gauge, Wrench } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { MaintenanceReport } from "@/types/backup";
import { formatBytes } from "@/utils/format";

/**
 * Maintenance panel (RC-10 M3): the safe maintenance pass — WAL
 * checkpoint, a full VACUUM only when the free-page ratio justifies a
 * rewrite, and `PRAGMA optimize` — with before/after measurements.
 */
export function MaintenancePanel({
  report,
  loading,
  error,
  acting,
  onRun,
}: {
  report: MaintenanceReport | null;
  loading: boolean;
  error: string | null;
  acting: boolean;
  onRun: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Wrench className="h-4 w-4 text-(--color-accent)" />
              Maintenance pass
            </CardTitle>
            <CardDescription>
              Checkpoints the WAL into the database file, reclaims free pages, and refreshes query
              statistics. Every run is measured and recorded in the ledger.
            </CardDescription>
          </div>
          <Button variant="secondary" disabled={acting} onClick={onRun}>
            <Gauge className="h-4 w-4" />
            Run maintenance
          </Button>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-sm text-(--color-muted-foreground)">Running maintenance…</p>
          ) : error ? (
            <p className="text-sm text-(--color-danger)">{error}</p>
          ) : report ? (
            <div className="flex flex-col gap-3">
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={report.vacuumRan ? "success" : "neutral"}>
                  {report.vacuumRan ? "vacuumed" : "no vacuum needed"}
                </Badge>
                <Badge variant="accent">{report.checkpointedFrames} frames checkpointed</Badge>
              </div>
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                <div className="flex flex-col gap-1">
                  <p className="text-xs text-(--color-muted-foreground)">File size</p>
                  <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">
                    {formatBytes(report.sizeBeforeBytes)} → {formatBytes(report.sizeAfterBytes)}
                  </p>
                </div>
                <div className="flex flex-col gap-1">
                  <p className="text-xs text-(--color-muted-foreground)">Free pages</p>
                  <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">
                    {report.freelistBefore} → {report.freelistAfter}
                  </p>
                </div>
                <div className="flex flex-col gap-1">
                  <p className="text-xs text-(--color-muted-foreground)">Recovered</p>
                  <p className="font-(family-name:--font-mono) text-sm text-(--color-success)">
                    {formatBytes(report.recoveredBytes)}
                  </p>
                </div>
              </div>
              <p className="text-xs text-(--color-muted-foreground)">
                {report.freedPages} pages freed · ran at {report.checkedAt}
              </p>
            </div>
          ) : (
            <p className="text-sm text-(--color-muted-foreground)">
              No maintenance run yet in this session.
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
