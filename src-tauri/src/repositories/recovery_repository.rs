//! Reliability & Recovery repository (RC-10 M2).
//!
//! Owns every SQL statement behind the fault-tolerance surfaces: the
//! append-only `recovery_journal` (checkpoints, heartbeats, crashes,
//! rollbacks, recovery runs, self-healing actions, health snapshots),
//! `crash_reports`, the per-worker `worker_health` table, and the
//! `recovery_history` audit ledger. All SQL stays here; detection,
//! validation, and policy logic live in [`crate::performance::recovery`].

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::recovery::{
    CrashReport, CrashType, JournalEntryType, RecoveryJournalEntry, RecoveryOutcome, RecoveryRun,
    RecoveryTrigger, WorkerHealth, WorkerStatus,
};

/// Raw `recovery_journal` row.
type JournalRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    DateTime<Utc>,
);
/// Raw `crash_reports` row.
type CrashRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    i32,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
);
/// Raw `worker_health` row.
type WorkerRow = (
    i64,
    String,
    String,
    DateTime<Utc>,
    i64,
    i64,
    i64,
    String,
    String,
    DateTime<Utc>,
);
/// Raw `recovery_history` row.
type HistoryRow = (
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// Repository for the RC-10 M2 reliability ledger.
#[derive(Debug, Clone)]
pub struct RecoveryRepository {
    pool: SqlitePool,
}

impl RecoveryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Journal
    // ------------------------------------------------------------------

    /// Appends one journal entry, returning its row id.
    pub async fn append_journal_entry(
        &self,
        entry_type: JournalEntryType,
        scope: &str,
        entity: &str,
        state: &str,
        payload: &serde_json::Value,
        checksum: &str,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO recovery_journal (entry_type, scope, entity, state, payload, checksum)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(entry_type.as_str())
        .bind(scope)
        .bind(entity)
        .bind(state)
        .bind(payload.to_string())
        .bind(checksum)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Most recent journal entries, newest-first.
    pub async fn recent_journal(
        &self,
        limit: u32,
    ) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        let rows: Vec<JournalRow> = sqlx::query_as(
            "SELECT id, entry_type, scope, entity, state, payload, checksum, created_at
             FROM recovery_journal ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(RecoveryJournalEntry::try_from)
            .collect()
    }

    /// Journal entries about one entity, newest-first.
    pub async fn journal_for_entity(
        &self,
        entity: &str,
        limit: u32,
    ) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        let rows: Vec<JournalRow> = sqlx::query_as(
            "SELECT id, entry_type, scope, entity, state, payload, checksum, created_at
             FROM recovery_journal WHERE entity = ?
             ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(entity)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(RecoveryJournalEntry::try_from)
            .collect()
    }

    /// Total number of journal entries (self-healing pruning trigger).
    pub async fn journal_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recovery_journal")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    /// Deletes all but the newest `keep` journal entries (bounded ledger).
    /// SQLite's `LIMIT -1 OFFSET ?` selects every row past the newest
    /// `keep`, oldest first — the rows this deletes.
    pub async fn prune_journal_excess(&self, keep: u64) -> Result<u64, DatabaseError> {
        let result = sqlx::query(
            "DELETE FROM recovery_journal
             WHERE id IN (
                 SELECT id FROM recovery_journal
                 ORDER BY created_at DESC, id DESC
                 LIMIT -1 OFFSET ?
             )",
        )
        .bind(keep as i64)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Checkpoints (persisted in the journal)
    // ------------------------------------------------------------------

    /// The most recent checkpoint entry (`entry_type = 'checkpoint'`).
    pub async fn latest_checkpoint(&self) -> Result<Option<RecoveryJournalEntry>, DatabaseError> {
        let row: Option<JournalRow> = sqlx::query_as(
            "SELECT id, entry_type, scope, entity, state, payload, checksum, created_at
             FROM recovery_journal WHERE entry_type = 'checkpoint'
             ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(RecoveryJournalEntry::try_from).transpose()
    }

    /// The most recent checkpoint entries, newest-first (rollback
    /// candidates — the oldest valid one is the safest fallback target).
    pub async fn recent_checkpoints(
        &self,
        limit: u32,
    ) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        let rows: Vec<JournalRow> = sqlx::query_as(
            "SELECT id, entry_type, scope, entity, state, payload, checksum, created_at
             FROM recovery_journal WHERE entry_type = 'checkpoint'
             ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(RecoveryJournalEntry::try_from)
            .collect()
    }

    // ------------------------------------------------------------------
    // Crash reports
    // ------------------------------------------------------------------

    /// Records one crash, returning its row id.
    #[allow(clippy::too_many_arguments)]
    pub async fn report_crash(
        &self,
        component: &str,
        crash_type: CrashType,
        severity: &str,
        message: &str,
        stack_trace: &str,
        metadata: &serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO crash_reports
               (component, crash_type, severity, message, stack_trace, metadata)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(component)
        .bind(crash_type.as_str())
        .bind(severity)
        .bind(message)
        .bind(stack_trace)
        .bind(metadata.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Most recent crash reports, newest-first.
    pub async fn recent_crash_reports(
        &self,
        limit: u32,
    ) -> Result<Vec<CrashReport>, DatabaseError> {
        let rows: Vec<CrashRow> = sqlx::query_as(
            "SELECT id, component, crash_type, severity, message, stack_trace, metadata,
                    was_recovered, recovered_at, reported_at
             FROM crash_reports ORDER BY reported_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(CrashReport::try_from).collect()
    }

    /// Marks a crash report as handled by automatic recovery.
    pub async fn mark_crash_recovered(&self, id: i64) -> Result<(), DatabaseError> {
        sqlx::query(
            "UPDATE crash_reports SET was_recovered = 1, recovered_at = ?
             WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Total number of crash reports (self-healing pruning trigger).
    pub async fn crash_report_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM crash_reports")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    /// Deletes crash reports older than `since`.
    pub async fn prune_crash_reports_before(
        &self,
        since: DateTime<Utc>,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query("DELETE FROM crash_reports WHERE reported_at < ?")
            .bind(since)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Worker health
    // ------------------------------------------------------------------

    /// Registers (or refreshes) a monitored worker, returning its row id.
    pub async fn register_worker(&self, worker: &str) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO worker_health (worker, status, last_heartbeat)
             VALUES (?, 'healthy', ?)
             ON CONFLICT(worker) DO UPDATE SET
                 status = 'healthy',
                 last_heartbeat = excluded.last_heartbeat,
                 consecutive_misses = 0
             RETURNING id",
        )
        .bind(worker)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Refreshes a worker's heartbeat; resets miss counting and status.
    pub async fn heartbeat_worker(&self, worker: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "UPDATE worker_health
             SET status = 'healthy',
                 last_heartbeat = ?,
                 consecutive_misses = 0,
                 updated_at = ?
             WHERE worker = ?",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(worker)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Records a missed heartbeat: marks the worker stalled and bumps the
    /// consecutive-miss counter the watchdog and self-healing use.
    pub async fn record_worker_miss(&self, worker: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "UPDATE worker_health
             SET status = 'stalled',
                 consecutive_misses = consecutive_misses + 1,
                 updated_at = ?
             WHERE worker = ?",
        )
        .bind(Utc::now())
        .bind(worker)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks a worker as failed (permanent, beyond the watchdog's grace).
    pub async fn mark_worker_failed(&self, worker: &str, error: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "UPDATE worker_health
             SET status = 'failed', last_error = ?, updated_at = ?
             WHERE worker = ?",
        )
        .bind(error)
        .bind(Utc::now())
        .bind(worker)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Restarts a worker's monitoring state: healthy, fresh heartbeat,
    /// zero misses/errors (the self-healing "restart worker" action).
    pub async fn mark_worker_healthy(&self, worker: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "UPDATE worker_health
             SET status = 'healthy',
                 last_heartbeat = ?,
                 consecutive_misses = 0,
                 last_error = '',
                 updated_at = ?
             WHERE worker = ?",
        )
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(worker)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// One worker's health row.
    pub async fn worker_health(&self, worker: &str) -> Result<Option<WorkerHealth>, DatabaseError> {
        let row: Option<WorkerRow> = sqlx::query_as(
            "SELECT id, worker, status, last_heartbeat, consecutive_misses,
                    execution_count, error_count, last_error, details, updated_at
             FROM worker_health WHERE worker = ?",
        )
        .bind(worker)
        .fetch_optional(&self.pool)
        .await?;
        row.map(WorkerHealth::try_from).transpose()
    }

    /// Every monitored worker's health row (watchdog/health-monitor input).
    pub async fn all_worker_health(&self) -> Result<Vec<WorkerHealth>, DatabaseError> {
        let rows: Vec<WorkerRow> = sqlx::query_as(
            "SELECT id, worker, status, last_heartbeat, consecutive_misses,
                    execution_count, error_count, last_error, details, updated_at
             FROM worker_health ORDER BY worker",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(WorkerHealth::try_from).collect()
    }

    /// Deletes worker rows that have not been seen since `since`
    /// (workers that no longer exist).
    pub async fn prune_workers_inactive_since(
        &self,
        since: DateTime<Utc>,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query("DELETE FROM worker_health WHERE last_heartbeat < ?")
            .bind(since)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Recovery history
    // ------------------------------------------------------------------

    /// Records one completed recovery run, returning its row id.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_recovery_run(&self, run: &RecoveryRun) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO recovery_history
               (run_id, trigger, outcome, status, actions, recovered_jobs,
                rolled_back_to, errors, duration_ms, started_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(run.run_id.to_string())
        .bind(run.trigger.as_str())
        .bind(run.outcome.as_str())
        .bind(&run.status)
        .bind(serde_json::to_string(&run.actions)?)
        .bind(serde_json::to_string(&run.recovered_jobs)?)
        .bind(
            run.rolled_back_to
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .bind(serde_json::to_string(&run.errors)?)
        .bind(run.duration_ms as i64)
        .bind(run.started_at)
        .bind(run.completed_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Most recent recovery runs, newest-first.
    pub async fn recent_recovery_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<RecoveryRun>, DatabaseError> {
        let rows: Vec<HistoryRow> = sqlx::query_as(
            "SELECT id, run_id, trigger, outcome, status, actions, recovered_jobs,
                    rolled_back_to, errors, duration_ms, started_at, completed_at
             FROM recovery_history ORDER BY completed_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(RecoveryRun::try_from).collect()
    }

    // ------------------------------------------------------------------
    // Health history
    // ------------------------------------------------------------------

    /// The most recent health snapshots (journal entries of type
    /// `health`), newest-first.
    pub async fn recent_health_snapshots(
        &self,
        limit: u32,
    ) -> Result<Vec<RecoveryJournalEntry>, DatabaseError> {
        let rows: Vec<JournalRow> = sqlx::query_as(
            "SELECT id, entry_type, scope, entity, state, payload, checksum, created_at
             FROM recovery_journal WHERE entry_type = 'health'
             ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(RecoveryJournalEntry::try_from)
            .collect()
    }
}

impl TryFrom<JournalRow> for RecoveryJournalEntry {
    type Error = DatabaseError;

    fn try_from(
        (id, entry_type, scope, entity, state, payload, checksum, created_at): JournalRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id,
            entry_type: JournalEntryType::from(entry_type.as_str()),
            scope,
            entity,
            state,
            payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
            checksum,
            created_at,
        })
    }
}

impl TryFrom<CrashRow> for CrashReport {
    type Error = DatabaseError;

    fn try_from(
        (
            id,
            component,
            crash_type,
            severity,
            message,
            stack_trace,
            metadata,
            was_recovered,
            recovered_at,
            reported_at,
        ): CrashRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id,
            component,
            crash_type: CrashType::from(crash_type.as_str()),
            severity,
            message,
            stack_trace,
            metadata: serde_json::from_str(&metadata).unwrap_or(serde_json::Value::Null),
            was_recovered: was_recovered != 0,
            recovered_at,
            reported_at,
        })
    }
}

impl TryFrom<WorkerRow> for WorkerHealth {
    type Error = DatabaseError;

    fn try_from(
        (
            id,
            worker,
            status,
            last_heartbeat,
            consecutive_misses,
            execution_count,
            error_count,
            last_error,
            details,
            updated_at,
        ): WorkerRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id,
            worker,
            status: WorkerStatus::from(status.as_str()),
            last_heartbeat,
            consecutive_misses: consecutive_misses.max(0) as u64,
            execution_count: execution_count.max(0) as u64,
            error_count: error_count.max(0) as u64,
            last_error,
            details: serde_json::from_str(&details).unwrap_or(serde_json::Value::Null),
            updated_at,
        })
    }
}

impl TryFrom<HistoryRow> for RecoveryRun {
    type Error = DatabaseError;

    fn try_from(
        (
            id,
            run_id,
            trigger,
            outcome,
            status,
            actions,
            recovered_jobs,
            rolled_back_to,
            errors,
            duration_ms,
            started_at,
            completed_at,
        ): HistoryRow,
    ) -> Result<Self, Self::Error> {
        let run_uuid = Uuid::parse_str(&run_id).unwrap_or_else(|_| Uuid::nil());
        Ok(Self {
            id,
            run_id: run_uuid,
            trigger: RecoveryTrigger::from(trigger.as_str()),
            outcome: RecoveryOutcome::from(outcome.as_str()),
            status,
            actions: serde_json::from_str(&actions).unwrap_or_default(),
            recovered_jobs: serde_json::from_str(&recovered_jobs).unwrap_or_default(),
            rolled_back_to: rolled_back_to.parse::<i64>().ok().filter(|id| *id > 0),
            errors: serde_json::from_str(&errors).unwrap_or_default(),
            duration_ms: duration_ms.max(0) as u64,
            started_at,
            completed_at,
        })
    }
}

#[cfg(test)]
#[path = "recovery_repository_tests.rs"]
mod tests;
