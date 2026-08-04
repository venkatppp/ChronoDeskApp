import { invoke } from "@tauri-apps/api/core";
import type {
  CrashReport,
  HealthSnapshot,
  RecoveryHistory,
  RecoveryJournalEntry,
  RollbackResult,
  SelfHealingReport,
} from "@/types/recovery";

export interface RecoveryRepository {
  /** A fresh aggregate health snapshot (persisted as health history). */
  recoveryStatus(): Promise<HealthSnapshot>;
  /** Combined recovery history: runs, crashes, and the recent journal. */
  recoveryHistory(limit?: number): Promise<RecoveryHistory>;
  /** The most recent crash reports, newest-first. */
  recoveryCrashReports(limit?: number): Promise<CrashReport[]>;
  /** The latest recovery checkpoint, if any. */
  recoveryLatestCheckpoint(): Promise<RecoveryJournalEntry | null>;
  /** Runs a self-healing pass on demand. */
  recoverySelfHeal(): Promise<SelfHealingReport>;
  /** Rolls back to the newest valid ancestor checkpoint. */
  recoveryRollback(): Promise<RollbackResult>;
  /** The watchdog's current tick counter. */
  recoveryTick(): Promise<number>;
}

export class TauriRecoveryRepository implements RecoveryRepository {
  async recoveryStatus(): Promise<HealthSnapshot> {
    return invoke<HealthSnapshot>("recovery_status");
  }

  async recoveryHistory(limit?: number): Promise<RecoveryHistory> {
    return invoke<RecoveryHistory>("recovery_history", { limit });
  }

  async recoveryCrashReports(limit?: number): Promise<CrashReport[]> {
    return invoke<CrashReport[]>("recovery_crash_reports", { limit });
  }

  async recoveryLatestCheckpoint(): Promise<RecoveryJournalEntry | null> {
    return invoke<RecoveryJournalEntry | null>("recovery_latest_checkpoint");
  }

  async recoverySelfHeal(): Promise<SelfHealingReport> {
    return invoke<SelfHealingReport>("recovery_self_heal");
  }

  async recoveryRollback(): Promise<RollbackResult> {
    return invoke<RollbackResult>("recovery_rollback");
  }

  async recoveryTick(): Promise<number> {
    return invoke<number>("recovery_tick");
  }
}

let instance: RecoveryRepository | null = null;

/** Returns the shared recovery repository (Tauri-backed). */
export function getRecoveryRepository(): RecoveryRepository {
  if (!instance) {
    instance = new TauriRecoveryRepository();
  }
  return instance;
}
