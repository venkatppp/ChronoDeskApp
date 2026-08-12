import { Activity, ScanSearch, ShieldCheck, ShieldAlert } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { IntegrityReport } from "@/types/backup";
import { formatBytes } from "@/utils/format";

/**
 * Integrity panel (RC-10 M3): the `PRAGMA` battery over the live database
 * — full integrity check, quick check, foreign-key check, journal mode
 * and page statistics — each run audited in the ledger.
 */
export function IntegrityPanel({
  report,
  loading,
  error,
  acting,
  onRun,
}: {
  report: IntegrityReport | null;
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
              <ShieldCheck className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
              Database integrity
            </CardTitle>
            <CardDescription>
              Scans every page of the database file, verifies foreign-key referential integrity, and
              reports file statistics. Each run is recorded in the ledger.
            </CardDescription>
          </div>
          <Button variant="secondary" disabled={acting} onClick={onRun}>
            <ScanSearch className="h-4 w-4" strokeWidth={1.75} />
            Run integrity check
          </Button>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-sm text-(--color-muted-foreground)">Running checks…</p>
          ) : error ? (
            <p className="text-sm text-(--color-danger)">{error}</p>
          ) : report ? (
            <div className="flex flex-col gap-3">
              <div className="flex items-center gap-2">
                {report.ok ? (
                  <>
                    <ShieldCheck className="h-4 w-4 text-(--color-success)" strokeWidth={1.75} />
                    <Badge variant="success">healthy</Badge>
                  </>
                ) : (
                  <>
                    <ShieldAlert className="h-4 w-4 text-(--color-danger)" strokeWidth={1.75} />
                    <Badge variant="warning">issues found</Badge>
                  </>
                )}
                <span className="text-xs text-(--color-muted-foreground)">
                  {formatBytes(report.main.databaseSizeBytes)} · {report.main.pageCount} pages ·{" "}
                  {report.main.pageSize} B/page · {report.main.freelistCount} free ·{" "}
                  journal {report.main.journalMode || "n/a"}
                </span>
              </div>
              {!report.main.integrity.ok && (
                <div className="flex flex-col gap-1">
                  <p className="text-xs font-medium text-(--color-danger)">integrity_check failures:</p>
                  {report.main.integrity.lines.map((line) => (
                    <p key={line} className="font-(family-name:--font-mono) text-xs text-(--color-danger)">
                      {line}
                    </p>
                  ))}
                </div>
              )}
              {!report.main.quickCheck.ok && (
                <div className="flex flex-col gap-1">
                  <p className="text-xs font-medium text-(--color-danger)">quick_check failures:</p>
                  {report.main.quickCheck.lines.map((line) => (
                    <p key={line} className="font-(family-name:--font-mono) text-xs text-(--color-danger)">
                      {line}
                    </p>
                  ))}
                </div>
              )}
              {report.main.foreignKeyCheck.length > 0 && (
                <div className="flex flex-col gap-1">
                  <p className="text-xs font-medium text-(--color-danger)">foreign-key violations:</p>
                  {report.main.foreignKeyCheck.map((line) => (
                    <p key={line} className="font-(family-name:--font-mono) text-xs text-(--color-danger)">
                      {line}
                    </p>
                  ))}
                </div>
              )}
              <p className="flex items-center gap-2 text-xs text-(--color-muted-foreground)">
                <Activity className="h-3 w-3" strokeWidth={1.75} />
                checked {report.checkedAt}
              </p>
            </div>
          ) : (
            <p className="text-sm text-(--color-muted-foreground)">
              No check run yet in this session.
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
