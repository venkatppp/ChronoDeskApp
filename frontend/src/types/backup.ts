// ----------------------------------------------------------------------
// RC-10 M3: Data Integrity & Backup
// Mirrors the camelCase DTOs from `models/backup.rs`.
// ----------------------------------------------------------------------

/** What kind of intervention a `backup_runs` ledger row records. */
export type BackupRunKind = "backup" | "restore" | "integrity" | "maintenance";

/** Terminal state of a `backup_runs` ledger row. */
export type BackupRunStatus = "success" | "failed" | "staged";

/** One row of the `backup_runs` audit ledger. */
export interface BackupRun {
  id: number;
  kind: BackupRunKind;
  status: BackupRunStatus;
  /** Produced backup filename / staged restore target / empty for in-place ops. */
  path: string;
  sizeBytes: number;
  /** SHA-256 hex of the backup file (empty for in-place ops). */
  checksum: string;
  detail: string;
  durationMs: number;
  startedAt: string;
  completedAt: string;
}

/** The parsed result of a single `PRAGMA integrity_check` / `quick_check`. */
export interface IntegrityLines {
  ok: boolean;
  lines: string[];
}

/** The full `PRAGMA` battery for the main database file. */
export interface IntegrityChecks {
  databaseSizeBytes: number;
  pageCount: number;
  pageSize: number;
  freelistCount: number;
  journalMode: string;
  integrity: IntegrityLines;
  quickCheck: IntegrityLines;
  foreignKeyCheck: string[];
}

/** Complete integrity report for a database file. */
export interface IntegrityReport {
  checkedAt: string;
  dbPath: string;
  main: IntegrityChecks;
  ok: boolean;
}

/** Result of a staged restore (applied on next launch). */
export interface RestoreResult {
  ok: boolean;
  message: string;
  backupPath: string;
  stagedPath: string;
  appliesOnNextLaunch: boolean;
  validated: IntegrityChecks;
}

/** Result of a maintenance run (discard-page free, size delta). */
export interface MaintenanceReport {
  checkedAt: string;
  freelistBefore: number;
  freelistAfter: number;
  freedPages: number;
  sizeBeforeBytes: number;
  sizeAfterBytes: number;
  recoveredBytes: number;
  vacuumRan: boolean;
  checkpointedFrames: number;
}