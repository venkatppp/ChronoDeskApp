import { useCallback, useEffect, useState } from "react";
import { Activity, History, RotateCcw, ScrollText, ShieldCheck, Stethoscope, TimerReset } from "lucide-react";
import { CrashPanel } from "@/components/recovery/CrashPanel";
import { HealthDashboard } from "@/components/recovery/HealthDashboard";
import { HistoryPanel } from "@/components/recovery/HistoryPanel";
import { JournalPanel } from "@/components/recovery/JournalPanel";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { getRecoveryRepository } from "@/services/recoveryRepository";
import type {
  CrashReport,
  HealthSnapshot,
  RecoveryHistory,
  RecoveryJournalEntry,
  RollbackResult,
  SelfHealingReport,
} from "@/types/recovery";

type Tab = "health" | "history" | "journal";

const TABS: { value: Tab; label: string; icon: typeof Activity }[] = [
  { value: "health", label: "Health", icon: Activity },
  { value: "history", label: "History", icon: History },
  { value: "journal", label: "Journal", icon: ScrollText },
];

/**
 * Recovery page (RC-10 M2): the reliability & recovery surface — runtime
 * health, monitored workers, crash reports, recovery history, the
 * append-only journal, plus manual self-healing and rollback, behind the
 * seven `recovery_*` IPC commands.
 */
export function RecoveryPage() {
  const repo = getRecoveryRepository();
  const [tab, setTab] = useState<Tab>("health");

  const [status, setStatus] = useState<HealthSnapshot | null>(null);
  const [statusLoading, setStatusLoading] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);

  const [history, setHistory] = useState<RecoveryHistory | null>(null);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [historyError, setHistoryError] = useState<string | null>(null);

  const [checkpoint, setCheckpoint] = useState<RecoveryJournalEntry | null>(null);
  const [tick, setTick] = useState<number>(0);

  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [acting, setActing] = useState(false);

  const refreshHealth = useCallback(async () => {
    setStatusLoading(true);
    setStatusError(null);
    try {
      setStatus(await repo.recoveryStatus());
      setTick(await repo.recoveryTick());
    } catch (err) {
      console.error("Failed to load health:", err);
      setStatusError("Failed to load the health snapshot. Please try again.");
    } finally {
      setStatusLoading(false);
    }
  }, [repo]);

  const refreshHistory = useCallback(async () => {
    setHistoryLoading(true);
    setHistoryError(null);
    try {
      const result = await repo.recoveryHistory(100);
      setHistory(result);
      setCheckpoint(await repo.recoveryLatestCheckpoint());
    } catch (err) {
      console.error("Failed to load recovery history:", err);
      setHistoryError("Failed to load recovery history. Please try again.");
    } finally {
      setHistoryLoading(false);
    }
  }, [repo]);

  const runAction = useCallback(
    async (action: "self_heal" | "rollback") => {
      setActing(true);
      setActionError(null);
      setActionMessage(null);
      try {
        if (action === "self_heal") {
          const report: SelfHealingReport = await repo.recoverySelfHeal();
          setActionMessage(
            `Self-healing pass done: ${report.executed.length} executed, ${report.failed.length} failed.`,
          );
        } else {
          const result: RollbackResult = await repo.recoveryRollback();
          setActionMessage(result.ok
            ? `Rolled back to checkpoint #${result.rolledBackTo}.`
            : `Rollback not applied: ${result.message}`);
        }
        await refreshHistory();
        await refreshHealth();
      } catch (err) {
        console.error("Recovery action failed:", err);
        setActionError("The recovery action failed. Please try again.");
      } finally {
        setActing(false);
      }
    },
    [repo, refreshHealth, refreshHistory],
  );

  useEffect(() => {
    refreshHealth();
    refreshHistory();
  }, [refreshHealth, refreshHistory]);

  return (
    <PageContainer>
      <PageHeader
        eyebrow="System"
        title="Recovery"
        description="Crash detection, checkpoint validation, watchdog monitoring, and self-healing for the ChronoDesk runtime."
        actions={
          <SegmentedControl
            ariaLabel="Recovery views"
            value={tab}
            onChange={setTab}
            options={TABS.map((t) => ({ value: t.value, label: t.label }))}
          />
        }
      />

      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
              Session Checkpoint
            </CardTitle>
            <CardDescription>
              The last persisted recovery checkpoint, used to distinguish a clean stop from a crash on
              the next launch.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="secondary" disabled={acting} onClick={() => runAction("self_heal")}>
              <Stethoscope className="h-4 w-4" strokeWidth={1.75} />
              Run self-healing
            </Button>
            <Button variant="secondary" disabled={acting} onClick={() => runAction("rollback")}>
              <RotateCcw className="h-4 w-4" strokeWidth={1.75} />
              Roll back
            </Button>
          </div>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          <div className="flex items-center gap-3">
            <TimerReset className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
            <span className="text-sm text-(--color-muted-foreground)">
              Watchdog tick <span className="font-mono text-(--color-foreground)">{tick}</span>
            </span>
          </div>
          {checkpoint ? (
            <p className="font-mono text-sm text-(--color-foreground)">
              #{checkpoint.id} <span className="text-(--color-muted-foreground)">{checkpoint.state}</span> · {checkpoint.scope} ·{" "}
              {checkpoint.entryType}
            </p>
          ) : (
            <p className="text-sm text-(--color-muted-foreground)">No checkpoint recorded yet.</p>
          )}
          {actionMessage && <p className="text-sm text-(--color-success)">{actionMessage}</p>}
          {actionError && <p className="text-sm text-(--color-danger)">{actionError}</p>}
        </CardContent>
      </Card>

      {tab === "health" && (
        <HealthDashboard snapshot={status} loading={statusLoading} error={statusError} />
      )}
      {tab === "history" && (
        <div className="flex flex-col gap-4">
          <HistoryPanel runs={history?.runs ?? []} loading={historyLoading} error={historyError} />
          <CrashPanel crashes={(history?.crashes ?? []) as CrashReport[]} loading={historyLoading} error={historyError} />
        </div>
      )}
      {tab === "journal" && (
        <JournalPanel entries={history?.journal ?? []} loading={historyLoading} error={historyError} />
      )}
    </PageContainer>
  );
}