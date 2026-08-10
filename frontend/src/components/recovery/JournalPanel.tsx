import { ScrollText } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import type { JournalEntryType, RecoveryJournalEntry } from "@/types/recovery";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

const TYPE_BADGE: Record<JournalEntryType, { label: string; variant: "success" | "warning" | "accent" | "neutral" }> = {
  checkpoint: { label: "checkpoint", variant: "accent" },
  heartbeat: { label: "heartbeat", variant: "neutral" },
  crash: { label: "crash", variant: "accent" },
  rollback: { label: "rollback", variant: "warning" },
  recovery: { label: "recovery", variant: "success" },
  self_healing: { label: "self-healing", variant: "success" },
  health: { label: "health", variant: "neutral" },
};

/**
 * Journal panel (RC-10 M2): the append-only reliability ledger — every
 * checkpoint, heartbeat, crash, rollback, recovery run, self-healing
 * action and health snapshot, newest-first.
 */
export function JournalPanel({ entries, loading, error }: { entries: RecoveryJournalEntry[]; loading: boolean; error: string | null }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ScrollText className="h-4 w-4 text-(--color-muted-foreground)" />
          Reliability Journal
        </CardTitle>
        <CardDescription>
          The append-only ledger of every reliability event, newest first.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {loading ? (
          <p className="text-sm text-(--color-muted-foreground)">Loading journal…</p>
        ) : error ? (
          <p className="text-sm text-(--color-danger)">{error}</p>
        ) : entries.length === 0 ? (
          <p className="text-sm text-(--color-muted-foreground)">No journal entries yet.</p>
        ) : (
          entries.map((entry) => {
            const badge = TYPE_BADGE[entry.entryType];
            return (
              <div key={entry.id} className="flex items-center justify-between gap-3 border-b border-(--color-border-subtle) pb-2 last:border-0 last:pb-0">
                <div className="flex min-w-0 items-center gap-2">
                  <Badge variant={badge.variant}>{badge.label}</Badge>
                  <span className="truncate font-(family-name:--font-mono) text-sm text-(--color-foreground)">{entry.entity}</span>
                  <span className="truncate text-sm text-(--color-muted-foreground)">{entry.state}</span>
                </div>
                <div className="flex shrink-0 items-center gap-3">
                  <span className="text-xs text-(--color-muted-foreground)">
                    {entry.scope} · {formatRelativeTime(entry.createdAt)}
                  </span>
                  {entry.checksum !== "" && (
                    <span className="hidden font-(family-name:--font-mono) text-[10px] text-(--color-muted-foreground) md:inline">
                      {entry.checksum.slice(0, 8)}
                    </span>
                  )}
                </div>
              </div>
            );
          })
        )}
      </CardContent>
    </Card>
  );
}