import { invoke } from "@tauri-apps/api/core";
import type {
  BackupRun,
  IntegrityReport,
  MaintenanceReport,
  RestoreResult,
} from "@/types/backup";

export interface MaintenanceRepository {
  /** Runs the full `PRAGMA` battery over the live database. */
  maintenanceIntegrity(): Promise<IntegrityReport>;
  /** Creates a backup snapshot in the backup directory. */
  maintenanceBackup(): Promise<BackupRun>;
  /** The most recent backup/integrity/maintenance ledger rows. */
  maintenanceBackups(limit?: number): Promise<BackupRun[]>;
  /** Stages the backup for restore on the next launch. */
  maintenanceRestore(backupId: number): Promise<RestoreResult>;
  /** Whether a staged restore is waiting to be applied. */
  maintenancePendingRestore(): Promise<RestoreResult | null>;
  /** Discards a staged restore. */
  maintenanceCancelRestore(): Promise<void>;
  /** Runs the maintenance pass (checkpoint → maybe VACUUM → optimize). */
  maintenanceOptimize(): Promise<MaintenanceReport>;
}

export class TauriMaintenanceRepository implements MaintenanceRepository {
  async maintenanceIntegrity(): Promise<IntegrityReport> {
    return invoke<IntegrityReport>("maintenance_integrity");
  }

  async maintenanceBackup(): Promise<BackupRun> {
    return invoke<BackupRun>("maintenance_backup");
  }

  async maintenanceBackups(limit?: number): Promise<BackupRun[]> {
    return invoke<BackupRun[]>("maintenance_backups", { limit });
  }

  async maintenanceRestore(backupId: number): Promise<RestoreResult> {
    return invoke<RestoreResult>("maintenance_restore", { backupId });
  }

  async maintenancePendingRestore(): Promise<RestoreResult | null> {
    return invoke<RestoreResult | null>("maintenance_pending_restore");
  }

  async maintenanceCancelRestore(): Promise<void> {
    return invoke<void>("maintenance_cancel_restore");
  }

  async maintenanceOptimize(): Promise<MaintenanceReport> {
    return invoke<MaintenanceReport>("maintenance_optimize");
  }
}

let instance: MaintenanceRepository | null = null;

/** Returns the shared maintenance repository (Tauri-backed). */
export function getMaintenanceRepository(): MaintenanceRepository {
  if (!instance) {
    instance = new TauriMaintenanceRepository();
  }
  return instance;
}
