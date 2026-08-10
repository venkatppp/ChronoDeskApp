// RetentionManagerCard - RC-6 M4: manage the retention policy of
// remembered runs (permanent / temporary / archived / expired) and run
// the cleanup pass, showing what the last pass removed.

import { useCallback, useEffect, useState } from "react";
import { Archive, Clock, Shield, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { memoryRepository } from "@/services/memoryRepository";
import type {
  CleanupReport,
  ExecutionMemoryRecord,
  MemoryHit,
  RetentionPolicy,
} from "@/types/memory";

const POLICY_TONE: Record<RetentionPolicy, "neutral" | "accent" | "warning" | "success"> = {
  permanent: "success",
  temporary: "accent",
  archived: "warning",
  expired: "neutral",
};

const POLICY_LABEL: Record<RetentionPolicy, string> = {
  permanent: "Permanent",
  temporary: "Temporary",
  archived: "Archived",
  expired: "Expired",
};

export function RetentionManagerCard() {
  const [memories, setMemories] = useState<ExecutionMemoryRecord[]>([]);
  const [report, setReport] = useState<CleanupReport | null>(null);
  const [cleaning, setCleaning] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const hits = (await memoryRepository.search("", { limit: 15 })) as MemoryHit[];
      setMemories(hits.map((hit) => hit.record));
    } catch (err) {
      console.error("Failed to load retention list:", err);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const setPolicy = async (record: ExecutionMemoryRecord, policy: RetentionPolicy) => {
    setBusyId(record.id);
    try {
      await memoryRepository.setRetention(record.id, policy);
      await load();
    } catch (err) {
      console.error("Retention change failed:", err);
    } finally {
      setBusyId(null);
    }
  };

  const runCleanup = async () => {
    if (cleaning) return;
    setCleaning(true);
    try {
      setReport(await memoryRepository.cleanupNow());
      await load();
    } catch (err) {
      console.error("Cleanup failed:", err);
    } finally {
      setCleaning(false);
    }
  };

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Shield className="h-4 w-4 text-(--color-muted-foreground)" />
          <h2 className="text-sm font-medium text-(--color-foreground)">Retention</h2>
        </div>
        <Button variant="secondary" size="sm" onClick={() => void runCleanup()} disabled={cleaning}>
          <Sparkles className={cleaning ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />
          {cleaning ? "Cleaning…" : "Clean up now"}
        </Button>
      </div>

      {report && (
        <p className="mt-2 rounded-[var(--radius-control)] bg-(--color-surface) px-3 py-2 text-[11px] text-(--color-muted-foreground)">
          Last pass: {report.expired_marked} expired · {report.removed_expired} deleted ·{" "}
          {report.removed_duplicate_archives} archive dupes · {report.compressed} compressed
        </p>
      )}

      <div className="mt-3 space-y-1.5">
        {memories.length === 0 && (
          <p className="text-xs text-(--color-faint-foreground)">
            No memories yet — runs will appear here with their retention policy.
          </p>
        )}
        {memories.map((record) => (
          <div
            key={record.id}
            className="flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] bg-(--color-surface) px-3 py-2"
          >
            <p className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">
              {record.goal}
            </p>
            <Badge variant={POLICY_TONE[record.retention]}>{POLICY_LABEL[record.retention]}</Badge>
            <span className="text-[10px] text-(--color-faint-foreground)">v{record.version}</span>
            {record.retention_until && (
              <span className="flex items-center gap-1 text-[10px] text-(--color-faint-foreground)">
                <Clock className="h-3 w-3" />
                {new Date(record.retention_until).toLocaleDateString()}
              </span>
            )}
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                disabled={busyId === record.id}
                onClick={() => void setPolicy(record, "permanent")}
                title="Keep forever"
              >
                Permanent
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busyId === record.id}
                onClick={() => void setPolicy(record, "archived")}
                title="Keep but out of active circulation"
              >
                <Archive className="h-3 w-3" /> Archive
              </Button>
              <Button
                variant="ghost"
                size="sm"
                disabled={busyId === record.id}
                onClick={() => void setPolicy(record, "expired")}
                title="Remove on the next cleanup pass"
              >
                Expire
              </Button>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
