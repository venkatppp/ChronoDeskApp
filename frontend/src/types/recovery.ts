// ----------------------------------------------------------------------
// RC-10 M2: Reliability & Recovery
// Mirrors the camelCase DTOs from `models/recovery.rs`.
// ----------------------------------------------------------------------

/** The kind of reliability event a journal entry records. */
export type JournalEntryType =
  | "checkpoint"
  | "heartbeat"
  | "crash"
  | "rollback"
  | "recovery"
  | "self_healing"
  | "health";

/** How a crash manifested. */
export type CrashType =
  | "panic"
  | "timeout"
  | "worker_failure"
  | "database"
  | "checkpoint_corrupt"
  | "unknown";

/** A monitored worker's liveness state. */
export type WorkerStatus = "healthy" | "stalled" | "failed" | "idle";

/** Aggregate health of the application as seen by the health monitor. */
export type HealthStatus = "healthy" | "degraded" | "critical";

/** What triggered a recovery run. */
export type RecoveryTrigger = "startup" | "crash" | "watchdog" | "rollback" | "manual";

/** How a recovery run ended. */
export type RecoveryOutcome =
  | "recovered"
  | "no_action"
  | "failed"
  | "rolled_back"
  | "partial";

/** One append-only reliability event. */
export interface RecoveryJournalEntry {
  id: number;
  entryType: JournalEntryType;
  /** Where the event happened (`startup`, `watchdog`, `runtime`, ...). */
  scope: string;
  /** The entity the entry is about (worker name, run id, `app`). */
  entity: string;
  /** Caller-provided state label (`running`, `clean`, ...). */
  state: string;
  payload: Record<string, unknown> | string | null;
  /** SHA-256 over `(entity, state, payload)` — detects half-written checkpoints. */
  checksum: string;
  createdAt: string;
}

/** One detected crash. */
export interface CrashReport {
  id: number;
  /** The subsystem that crashed (`runtime`, `worker:<name>`, ...). */
  component: string;
  crashType: CrashType;
  /** `error` | `critical`. */
  severity: string;
  message: string;
  stackTrace: string;
  metadata: Record<string, unknown> | string | null;
  /** Whether automatic recovery already handled this crash. */
  wasRecovered: boolean;
  recoveredAt: string | null;
  reportedAt: string;
}

/** One monitored worker's persisted health row. */
export interface WorkerHealth {
  id: number;
  worker: string;
  status: WorkerStatus;
  lastHeartbeat: string;
  consecutiveMisses: number;
  executionCount: number;
  errorCount: number;
  lastError: string;
  details: Record<string, unknown> | string | null;
  updatedAt: string;
}

/** What the watchdog noticed about one worker on one pass. */
export interface WatchdogEvent {
  worker: string;
  /** `stalled` | `recovered` | `failed`. */
  kind: string;
  detail: string;
  occurredAt: string;
}

/** A point-in-time aggregate health view of the application. */
export interface HealthSnapshot {
  capturedAt: string;
  status: HealthStatus;
  /** `0..=100`, derived from worker liveness. */
  overallScore: number;
  workers: WorkerHealth[];
  /** Human-readable problems found (stalled workers, invalid checkpoints, ...). */
  issues: string[];
  details: Record<string, unknown> | string | null;
}

/** One completed recovery run (audit row). */
export interface RecoveryRun {
  id: number;
  runId: string;
  trigger: RecoveryTrigger;
  outcome: RecoveryOutcome;
  /** `success` | `partial` | `failed`. */
  status: string;
  /** Recovery action names executed, in order. */
  actions: string[];
  /** Jobs resumed or rolled back, in order. */
  recoveredJobs: string[];
  /** Journal id the state rolled back to, when a rollback happened. */
  rolledBackTo: number | null;
  errors: string[];
  durationMs: number;
  startedAt: string;
  completedAt: string;
}

/** Result of a rollback operation. */
export interface RollbackResult {
  /** Journal id the state rolled back to. */
  rolledBackTo: number | null;
  /** Jobs restored from the target checkpoint. */
  restored: string[];
  ok: boolean;
  message: string;
}

/** Combined history for the recovery surface. */
export interface RecoveryHistory {
  runs: RecoveryRun[];
  crashes: CrashReport[];
  journal: RecoveryJournalEntry[];
}

/** What the self-healing service did on one pass. */
export interface SelfHealingReport {
  /** Planned actions that were actually executed. */
  executed: string[];
  /** Planned actions that could not be executed. */
  failed: string[];
  /** Workers whose monitoring state was restarted. */
  healedWorkers: string[];
  ranAt: string;
}
