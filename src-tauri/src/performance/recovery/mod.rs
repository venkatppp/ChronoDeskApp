//! Reliability & Recovery (RC-10 M2 — Production Hardening).
//!
//! The fault-tolerance subsystem: automatic crash detection and startup
//! recovery, checkpoint validation (checksum + ordering), rollback to the
//! last valid checkpoint, a background watchdog over worker heartbeats, a
//! health monitor producing snapshots, and a self-healing service that
//! executes the safe remediation for whatever the monitor sees.
//!
//! Layout mirrors `intelligence::health` and the rest of the codebase:
//! the pure policy pieces (`CheckpointValidator`, `Watchdog::evaluate`,
//! `HealthMonitor::assess`, `SelfHealingService::plan`) are functions of
//! their inputs and carry no SQL; the stateful components
//! (`Journal`, `CrashRecoveryService`, `RollbackService`,
//! `HealthMonitor::capture`, `SelfHealingService::run`,
//! `WatchdogService::scan`) compose [`crate::repositories::RecoveryRepository`].
//! [`RecoveryManager`] is the facade `lib.rs` wires as managed state.

mod checkpoint_validator;
mod crash_recovery;
mod health_monitor;
mod journal;
mod recovery_manager;
mod rollback;
mod self_healing;
mod watchdog;

pub use checkpoint_validator::CheckpointValidator;
pub use crash_recovery::CrashRecoveryService;
pub use health_monitor::HealthMonitor;
pub use journal::Journal;
pub use recovery_manager::RecoveryManager;
pub use rollback::RollbackService;
pub use self_healing::SelfHealingService;
pub use watchdog::WatchdogService;

/// How long the watchdog lets a worker go without a heartbeat before
/// declaring it stalled (seconds).
pub const WATCHDOG_HEARTBEAT_GRACE_SECS: i64 = 120;
/// Interval between watchdog passes (seconds).
pub const WATCHDOG_INTERVAL_SECS: u64 = 30;
/// Consecutive misses before self-healing restarts a worker's
/// monitoring state.
pub const WORKER_RESTART_AFTER_MISSES: u64 = 3;
/// Journal entries past which self-healing prunes history (keeps the
/// ledger bounded like the profiler ledger in RC-10 M1).
pub const JOURNAL_PRUNE_THRESHOLD: u64 = 10_000;
/// How much history self-healing keeps when pruning (days).
pub const JOURNAL_PRUNE_KEEP_DAYS: i64 = 30;
