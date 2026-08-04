import { useCallback, useEffect, useState } from "react";
import { DatabaseBackup, Gauge, ShieldCheck } from "lucide-react";
import { BackupPanel } from "@/components/maintenance/BackupPanel";
import { IntegrityPanel } from "@/components/maintenance/IntegrityPanel";
import { MaintenancePanel } from "@/components/maintenance/MaintenancePanel";
import { getMaintenanceRepository } from "@/services/maintenanceRepository";
import type { BackupRun, IntegrityReport, MaintenanceReport, RestoreResult } from "@/types/backup";

type Tab = "backups" | "integrity" | "maintenance";

const TABS: { value: Tab; label: string; icon: typeof DatabaseBackup }[] = [
  { value: "backups", label: "Backups", icon: DatabaseBackup },
  { value: "integrity", label: "Integrity", icon: ShieldCheck },
  { value: "maintenance", label: "Maintenance", icon: Gauge },
];

/**
 * Maintenance page (RC-10 M3): the data integrity & backup surface —
 * snapshots, the audit ledger, staged restores (applied on next launch),
 * the `PRAGMA` integrity battery, and the maintenance pass — behind the
 * seven `maintenance_*` IPC commands.
 */
export function MaintenancePage() {
  const repo = getMaintenanceRepository();
  const [tab, setTab] = useState<Tab>("backups");

  const [runs, setRuns] = useState<BackupRun[]>([]);
  const [pending, setPending] = useState<RestoreResult | null>(null);
  const [ledgerLoading, setLedgerLoading] = useState(false);
  const [ledgerError, setLedgerError] = useState<string | null>(null);

  const [integrity, setIntegrity] = useState<IntegrityReport | null>(null);
  const [integrityLoading, setIntegrityLoading] = useState(false);
  const [integrityError, setIntegrityError] = useState<string | null>(null);

  const [maintenance, setMaintenance] = useState<MaintenanceReport | null>(null);
  const [maintenanceLoading, setMaintenanceLoading] = useState(false);
  const [maintenanceError, setMaintenanceError] = useState<string | null>(null);

  const [acting, setActing] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);

  const refreshLedger = useCallback(async () => {
    setLedgerLoading(true);
    setLedgerError(null);
    try {
      setRuns(await repo.maintenanceBackups(100));
      setPending(await repo.maintenancePendingRestore());
    } catch (err) {
      console.error("Failed to load the backup ledger:", err);
      setLedgerError("Failed to load the backup ledger. Please try again.");
    } finally {
      setLedgerLoading(false);
    }
  }, [repo]);

  const runAction = useCallback(
    async (action: "backup" | "restore" | "cancel_restore" | "integrity" | "maintenance", backupId?: number) => {
      setActing(true);
      setActionError(null);
      setActionMessage(null);
      try {
        if (action === "backup") {
          const run = await repo.maintenanceBackup();
          setActionMessage(`Backup created: ${run.path} (${run.checksum.slice(0, 12)}…)`);
        } else if (action === "restore" && backupId !== undefined) {
          const result = await repo.maintenanceRestore(backupId);
          setActionMessage(result.ok ? result.message : `Restore rejected: ${result.message}`);
        } else if (action === "cancel_restore") {
          await repo.maintenanceCancelRestore();
          setActionMessage("Staged restore cancelled.");
        } else if (action === "integrity") {
          setIntegrityLoading(true);
          setIntegrityError(null);
          try {
            setIntegrity(await repo.maintenanceIntegrity());
          } finally {
            setIntegrityLoading(false);
          }
        } else {
          setMaintenanceLoading(true);
          setMaintenanceError(null);
          try {
            setMaintenance(await repo.maintenanceOptimize());
          } finally {
            setMaintenanceLoading(false);
          }
        }
        await refreshLedger();
      } catch (err) {
        console.error("Maintenance action failed:", err);
        setActionError("The maintenance action failed. Please try again.");
      } finally {
        setActing(false);
      }
    },
    [repo, refreshLedger],
  );

  useEffect(() => {
    refreshLedger();
  }, [refreshLedger]);

  return (
    <div className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="font-(family-name:--font-display) text-xl font-bold tracking-tight">Maintenance</h1>
        <p className="text-sm text-(--color-muted-foreground)">
          Data integrity & backup: snapshots, staged restores, integrity checks, and the maintenance pass.
        </p>
      </div>

      <div className="flex gap-1 border-b border-(--color-border-subtle)">
        {TABS.map((item) => (
          <button
            key={item.value}
            type="button"
            onClick={() => setTab(item.value)}
            className={
              tab === item.value
                ? "flex items-center gap-2 border-b-2 border-(--color-accent) px-3 pb-2 text-sm font-medium text-(--color-foreground)"
                : "flex items-center gap-2 border-b-2 border-transparent px-3 pb-2 text-sm text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            }
          >
            <item.icon className="h-4 w-4" />
            {item.label}
          </button>
        ))}
      </div>

      {(actionMessage || actionError) && (
        <p className={actionError ? "text-sm text-(--color-danger)" : "text-sm text-(--color-success)"}>
          {actionError ?? actionMessage}
        </p>
      )}

      {tab === "backups" && (
        <BackupPanel
          runs={runs}
          pending={pending}
          loading={ledgerLoading}
          error={ledgerError}
          acting={acting}
          onBackup={() => runAction("backup")}
          onRestore={(id) => runAction("restore", id)}
          onCancelRestore={() => runAction("cancel_restore")}
        />
      )}
      {tab === "integrity" && (
        <IntegrityPanel
          report={integrity}
          loading={integrityLoading}
          error={integrityError ?? actionError}
          acting={acting}
          onRun={() => runAction("integrity")}
        />
      )}
      {tab === "maintenance" && (
        <MaintenancePanel
          report={maintenance}
          loading={maintenanceLoading}
          error={maintenanceError ?? actionError}
          acting={acting}
          onRun={() => runAction("maintenance")}
        />
      )}
    </div>
  );
}
