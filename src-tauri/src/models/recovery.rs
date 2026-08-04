//! Reliability & Recovery models (RC-10 M2).
//!
//! DTOs for the fault-tolerance surfaces: the append-only journal of
//! reliability events (checkpoints, heartbeats, crashes, rollbacks,
//! recovery runs, self-healing actions, health snapshots), crash reports,
//! worker health, recovery runs, and the report types produced by the
//! validator, rollback service, health monitor, watchdog and self-healing
//! service. Everything here is a plain serializable DTO — the SQL lives
//! in [`crate::repositories::RecoveryRepository`] and the policy logic in
//! [`crate::performance::recovery`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ----------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------

/// The kind of reliability event a journal entry records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEntryType {
    /// A persisted recovery checkpoint (state + active jobs + checksum).
    Checkpoint,
    /// A liveness heartbeat (worker or runtime).
    Heartbeat,
    /// A detected crash.
    Crash,
    /// A rollback to a previously valid checkpoint.
    Rollback,
    /// A recovery run.
    Recovery,
    /// A self-healing action.
    SelfHealing,
    /// A health snapshot capture.
    Health,
}

impl JournalEntryType {
    pub fn as_str(self) -> &'static str {
        match self {
            JournalEntryType::Checkpoint => "checkpoint",
            JournalEntryType::Heartbeat => "heartbeat",
            JournalEntryType::Crash => "crash",
            JournalEntryType::Rollback => "rollback",
            JournalEntryType::Recovery => "recovery",
            JournalEntryType::SelfHealing => "self_healing",
            JournalEntryType::Health => "health",
        }
    }
}

impl From<&str> for JournalEntryType {
    fn from(value: &str) -> Self {
        match value {
            "heartbeat" => JournalEntryType::Heartbeat,
            "crash" => JournalEntryType::Crash,
            "rollback" => JournalEntryType::Rollback,
            "recovery" => JournalEntryType::Recovery,
            "self_healing" => JournalEntryType::SelfHealing,
            "health" => JournalEntryType::Health,
            _ => JournalEntryType::Checkpoint,
        }
    }
}

/// How a crash manifested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashType {
    /// A panic or unrecoverable error.
    Panic,
    /// The last checkpoint aged past the grace window without a clean
    /// shutdown record (the process died or hung silently).
    Timeout,
    /// A background worker failed.
    WorkerFailure,
    /// A database error took the subsystem down.
    Database,
    /// A persisted checkpoint failed validation.
    CheckpointCorrupt,
    /// Not enough evidence to classify.
    Unknown,
}

impl CrashType {
    pub fn as_str(self) -> &'static str {
        match self {
            CrashType::Panic => "panic",
            CrashType::Timeout => "timeout",
            CrashType::WorkerFailure => "worker_failure",
            CrashType::Database => "database",
            CrashType::CheckpointCorrupt => "checkpoint_corrupt",
            CrashType::Unknown => "unknown",
        }
    }
}

impl From<&str> for CrashType {
    fn from(value: &str) -> Self {
        match value {
            "panic" => CrashType::Panic,
            "timeout" => CrashType::Timeout,
            "worker_failure" => CrashType::WorkerFailure,
            "database" => CrashType::Database,
            "checkpoint_corrupt" => CrashType::CheckpointCorrupt,
            _ => CrashType::Unknown,
        }
    }
}

/// A monitored worker's liveness state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerStatus {
    /// Reporting heartbeats within the grace window.
    Healthy,
    /// Missed its heartbeats; the watchdog is watching.
    Stalled,
    /// Failed permanently; needs intervention.
    Failed,
    /// Registered but intentionally not running.
    Idle,
}

impl WorkerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkerStatus::Healthy => "healthy",
            WorkerStatus::Stalled => "stalled",
            WorkerStatus::Failed => "failed",
            WorkerStatus::Idle => "idle",
        }
    }
}

impl From<&str> for WorkerStatus {
    fn from(value: &str) -> Self {
        match value {
            "stalled" => WorkerStatus::Stalled,
            "failed" => WorkerStatus::Failed,
            "idle" => WorkerStatus::Idle,
            _ => WorkerStatus::Healthy,
        }
    }
}

/// Aggregate health of the application as seen by the health monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl HealthStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded => "degraded",
            HealthStatus::Critical => "critical",
        }
    }
}

/// What triggered a recovery run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryTrigger {
    Startup,
    Crash,
    Watchdog,
    Rollback,
    Manual,
}

impl RecoveryTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            RecoveryTrigger::Startup => "startup",
            RecoveryTrigger::Crash => "crash",
            RecoveryTrigger::Watchdog => "watchdog",
            RecoveryTrigger::Rollback => "rollback",
            RecoveryTrigger::Manual => "manual",
        }
    }
}

impl From<&str> for RecoveryTrigger {
    fn from(value: &str) -> Self {
        match value {
            "crash" => RecoveryTrigger::Crash,
            "watchdog" => RecoveryTrigger::Watchdog,
            "rollback" => RecoveryTrigger::Rollback,
            "manual" => RecoveryTrigger::Manual,
            _ => RecoveryTrigger::Startup,
        }
    }
}

/// How a recovery run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    Recovered,
    NoAction,
    Failed,
    RolledBack,
    Partial,
}

impl RecoveryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            RecoveryOutcome::Recovered => "recovered",
            RecoveryOutcome::NoAction => "no_action",
            RecoveryOutcome::Failed => "failed",
            RecoveryOutcome::RolledBack => "rolled_back",
            RecoveryOutcome::Partial => "partial",
        }
    }
}

impl From<&str> for RecoveryOutcome {
    fn from(value: &str) -> Self {
        match value {
            "recovered" => RecoveryOutcome::Recovered,
            "failed" => RecoveryOutcome::Failed,
            "rolled_back" => RecoveryOutcome::RolledBack,
            "partial" => RecoveryOutcome::Partial,
            _ => RecoveryOutcome::NoAction,
        }
    }
}

/// A recovery action executed as part of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// A checkpoint was persisted.
    Checkpoint,
    /// The runtime heartbeat was refreshed.
    Heartbeat,
    /// Interrupted jobs were resumed from a checkpoint.
    Resume,
    /// The state was rolled back to a previous valid checkpoint.
    Rollback,
    /// A stalled worker's monitoring state was restarted.
    RestartWorker,
    /// The latest checkpoint was re-validated.
    Revalidate,
}

impl RecoveryAction {
    pub fn as_str(self) -> &'static str {
        match self {
            RecoveryAction::Checkpoint => "checkpoint",
            RecoveryAction::Heartbeat => "heartbeat",
            RecoveryAction::Resume => "resume",
            RecoveryAction::Rollback => "rollback",
            RecoveryAction::RestartWorker => "restart_worker",
            RecoveryAction::Revalidate => "revalidate",
        }
    }
}

// ----------------------------------------------------------------------
// Journal & checkpoints
// ----------------------------------------------------------------------

/// One append-only reliability event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryJournalEntry {
    pub id: i64,
    pub entry_type: JournalEntryType,
    /// Where the event happened (`startup`, `watchdog`, `runtime`, ...).
    pub scope: String,
    /// The entity the entry is about (worker name, run id, `app`).
    pub entity: String,
    /// Caller-provided state label (`running`, `clean`, ...).
    pub state: String,
    pub payload: serde_json::Value,
    /// SHA-256 over `(entity, state, payload)` — lets the validator
    /// detect a half-written checkpoint after a crash.
    pub checksum: String,
    pub created_at: DateTime<Utc>,
}

/// Result of validating one checkpoint entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointValidationResult {
    pub valid: bool,
    /// Human-readable issues; empty when `valid` is true.
    pub issues: Vec<String>,
    /// The journal id the validation ran against.
    pub entry_id: Option<i64>,
    pub checked_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Crashes
// ----------------------------------------------------------------------

/// One detected crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub id: i64,
    /// The subsystem that crashed (`runtime`, `worker:<name>`, ...).
    pub component: String,
    pub crash_type: CrashType,
    /// `error` | `critical`.
    pub severity: String,
    pub message: String,
    pub stack_trace: String,
    pub metadata: serde_json::Value,
    /// Whether automatic recovery already handled this crash.
    pub was_recovered: bool,
    pub recovered_at: Option<DateTime<Utc>>,
    pub reported_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Workers
// ----------------------------------------------------------------------

/// One monitored worker's persisted health row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHealth {
    pub id: i64,
    pub worker: String,
    pub status: WorkerStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub consecutive_misses: u64,
    pub execution_count: u64,
    pub error_count: u64,
    pub last_error: String,
    pub details: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// What the watchdog noticed about one worker on one pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchdogEvent {
    pub worker: String,
    /// `stalled` | `recovered` | `failed`.
    pub kind: String,
    pub detail: String,
    pub occurred_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Health
// ----------------------------------------------------------------------

/// A point-in-time aggregate health view of the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub captured_at: DateTime<Utc>,
    pub status: HealthStatus,
    /// `0..=100`, derived from worker liveness.
    pub overall_score: f64,
    pub workers: Vec<WorkerHealth>,
    /// Human-readable problems found (stalled workers, invalid
    /// checkpoints, ...).
    pub issues: Vec<String>,
    pub details: serde_json::Value,
}

// ----------------------------------------------------------------------
// Recovery runs & history
// ----------------------------------------------------------------------

/// One completed recovery run (audit row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryRun {
    pub id: i64,
    pub run_id: Uuid,
    pub trigger: RecoveryTrigger,
    pub outcome: RecoveryOutcome,
    /// `success` | `partial` | `failed`.
    pub status: String,
    /// Recovery action names executed, in order.
    pub actions: Vec<String>,
    /// Jobs resumed or rolled back, in order.
    pub recovered_jobs: Vec<String>,
    /// Journal id the state rolled back to, when a rollback happened.
    pub rolled_back_to: Option<i64>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

/// Result of a rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    /// Journal id the state rolled back to.
    pub rolled_back_to: Option<i64>,
    /// Jobs restored from the target checkpoint.
    pub restored: Vec<String>,
    pub ok: bool,
    pub message: String,
}

/// Combined history for the recovery surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryHistory {
    pub runs: Vec<RecoveryRun>,
    pub crashes: Vec<CrashReport>,
    pub journal: Vec<RecoveryJournalEntry>,
}

// ----------------------------------------------------------------------
// Self healing
// ----------------------------------------------------------------------

/// What the self-healing service did on one pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfHealingReport {
    /// Planned actions that were actually executed.
    pub executed: Vec<String>,
    /// Planned actions that could not be executed.
    pub failed: Vec<String>,
    /// Workers whose monitoring state was restarted.
    pub healed_workers: Vec<String>,
    pub ran_at: DateTime<Utc>,
}

// ----------------------------------------------------------------------
// Inputs
// ----------------------------------------------------------------------

/// Payload carried by a checkpoint journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointPayload {
    /// Jobs active at checkpoint time; resumed after a crash.
    pub active_jobs: Vec<String>,
    /// Free-form context (run id, phase, ...).
    pub metadata: serde_json::Value,
}
