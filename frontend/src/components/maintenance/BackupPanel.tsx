import { CheckCircle2, DatabaseBackup, RotateCcw, ShieldAlert, XCircle } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { BackupRun, RestoreResult } from "@/types/backup";
import { formatBytes } from "@/utils/format";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

const STATUS_BADGE: Record<BackupRun["status"], { label: string; variant: "success" | "warning" | "accent" | "neutral" }> = {
  success: { label: "success", variant: "success" },
  failed: { label: "failed", variant: "accent" },
  staged: { label: "staged", variant: "warning" },
};

const KIND_LABEL: Record<BackupRun["kind"], string> = {
  backup: "backup",
  restore: "restore",
  integrity: "integrity",
  maintenance: "maintenance",
};

/**
 * Backup & restore panel (RC-10 M3): create a snapshot, browse the audit
 * ledger, stage a restore from any successful backup, and cancel a staged
 * restore. Restores apply on the next launch — never to a live database.
 */
export function BackupPanel({
  runs,
  pending,
  loading,
  error,
  acting,
  onBackup,
  onRestore,
  onCancelRestore,
}: {
  runs: BackupRun[];
  pending: RestoreResult | null;
  loading: boolean;
  error: string | null;
  acting: boolean;
  onBackup: () => void;
  onRestore: (id: number) => void;
  onCancelRestore: () => void;
}) {
  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <DatabaseBackup className="h-4 w-4 text-(--color-accent)" />
              Back up now
            </CardTitle>
            <CardDescription>
              Creates a consistent snapshot of the database (SQLite VACUUM INTO) with a SHA-256
              checksum, stored in the app-data backups folder.
            </CardDescription>
          </div>
          <Button variant="secondary" disabled={acting} onClick={onBackup}>
            <DatabaseBackup className="h-4 w-4" />
            Back up now
          </Button>
        </CardHeader>
      </Card>

      {pending && (
        <Card>
          <CardContent className="flex flex-col gap-2">
            <div className="flex items-center gap-2">
              {pending.ok ? (
                <CheckCircle2 className="h-4 w-4 text-(--color-success)" />
              ) : (
                <ShieldAlert className="h-4 w-4 text-(--color-warning)" />
              )}
              <p className="text-sm font-medium text-(--color-foreground)">
                {pending.ok ? "Restore staged" : "Staged restore needs attention"}
              </p>
            </div>
            <p className="text-sm text-(--color-muted-foreground)">
              {pending.message} — the swap happens before the database opens, and your current
              database is preserved as a safety copy.
            </p>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="secondary" disabled={acting} onClick={onCancelRestore}>
                <XCircle className="h-4 w-4" />
                Cancel staged restore
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Backup & maintenance ledger</CardTitle>
          <CardDescription>
            Every backup, staged restore, integrity check and maintenance run is recorded here.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {loading ? (
            <p className="text-sm text-(--color-muted-foreground)">Loading ledger…</p>
          ) : error ? (
            <p className="text-sm text-(--color-danger)">{error}</p>
          ) : runs.length === 0 ? (
            <p className="text-sm text-(--color-muted-foreground)">No runs recorded yet.</p>
          ) : (
            runs.map((run) => {
              const badge = STATUS_BADGE[run.status];
              const restorable = run.kind === "backup" && run.status === "success";
              return (
                <div
                  key={run.id}
                  className="flex items-center justify-between gap-3 border-b border-(--color-border-subtle) pb-3 last:border-0 last:pb-0"
                >
                  <div className="flex min-w-0 flex-col gap-1">
                    <div className="flex items-center gap-2">
                      <Badge variant={badge.variant}>{badge.label}</Badge>
                      <span className="text-xs text-(--color-muted-foreground)">
                        {KIND_LABEL[run.kind]}
                      </span>
                      {run.path && (
                        <span className="truncate font-mono text-xs text-(--color-foreground)">
                          {run.path}
                        </span>
                      )}
                    </div>
                    {run.checksum && (
                      <p className="truncate font-mono text-xs text-(--color-muted-foreground)">
                        sha256 {run.checksum}
                      </p>
                    )}
                    {run.detail && (
                      <p className="truncate text-xs text-(--color-muted-foreground)">{run.detail}</p>
                    )}
                    <p className="text-xs text-(--color-muted-foreground)">
                      {formatRelativeTime(run.completedAt)}
                      {run.sizeBytes > 0 && ` · ${formatBytes(run.sizeBytes)}`}
                      {run.durationMs > 0 && ` · ${run.durationMs} ms`}
                    </p>
                  </div>
                  {restorable && (
                    <Button size="sm" variant="secondary" disabled={acting} onClick={() => onRestore(run.id)}>
                      <RotateCcw className="h-4 w-4" />
                      Restore
                    </Button>
                  )}
                </div>
              );
            })
          )}
        </CardContent>
      </Card>
    </div>
  );
}
