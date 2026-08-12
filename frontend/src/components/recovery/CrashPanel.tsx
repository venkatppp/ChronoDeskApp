import { Bug, CheckCircle2 } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { CrashReport } from "@/types/recovery";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

/**
 * Crash panel (RC-10 M2): every detected crash with its type, severity,
 * whether automatic recovery already handled it, and the recovery time.
 */
export function CrashPanel({ crashes, loading, error }: { crashes: CrashReport[]; loading: boolean; error: string | null }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Bug className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          Crash Reports
        </CardTitle>
        <CardDescription>
          Detected crash events from startup recovery and the watchdog. Automatic recovery marks handled
          reports.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {loading ? (
          <p className="text-sm text-(--color-muted-foreground)">Loading crash reports…</p>
        ) : error ? (
          <p className="text-sm text-(--color-danger)">{error}</p>
        ) : crashes.length === 0 ? (
          <p className="text-sm text-(--color-muted-foreground)">No crashes recorded. The runtime has stayed up.</p>
        ) : (
          crashes.map((crash) => (
            <div key={crash.id} className="flex flex-col gap-1 border-b border-(--color-border-subtle) pb-3 last:border-0 last:pb-0">
              <div className="flex items-center justify-between gap-3">
                <p className="font-(family-name:--font-mono) text-sm text-(--color-foreground)">
                  {crash.component} <span className="text-(--color-muted-foreground)">/ {crash.crashType}</span>
                </p>
                <div className="flex items-center gap-2">
                  {crash.wasRecovered ? (
                    <Badge variant="success">
                      <CheckCircle2 className="h-3 w-3" strokeWidth={1.75} /> recovered
                    </Badge>
                  ) : (
                    <Badge variant="warning">open</Badge>
                  )}
                  <Badge variant={crash.severity === "critical" ? "accent" : "neutral"}>{crash.severity}</Badge>
                </div>
              </div>
              <p className="text-sm text-(--color-muted-foreground)">{crash.message}</p>
              <p className="text-xs text-(--color-muted-foreground)">
                reported {formatRelativeTime(crash.reportedAt)}
                {crash.recoveredAt ? ` · recovered ${formatRelativeTime(crash.recoveredAt)}` : ""}
              </p>
            </div>
          ))
        )}
      </CardContent>
    </Card>
  );
}