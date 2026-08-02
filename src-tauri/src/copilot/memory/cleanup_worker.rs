//! Memory Cleanup Worker (RC-6 M4) — the background lifecycle worker.
//!
//! On a periodic interval (and on `notify()` wake-ups) it runs one
//! cleanup pass over the memory store — expiring temporary memories past
//! their deadline, deleting expired memories, removing duplicate
//! archives and orphaned vectors, compressing oversized reasoning — and
//! periodically captures a full-store snapshot.
//!
//! The worker only consults the `MemoryEngine` facade (lifecycle
//! belongs to the engine); it plans and schedules nothing else.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify};

use crate::copilot::memory::engine::MemoryEngine;

/// How often a cleanup pass runs even without notifications.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// How often an automatic ("auto") snapshot is captured.
const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// The background memory lifecycle worker.
#[derive(Clone)]
pub struct MemoryCleanupWorker {
    engine: MemoryEngine,
    notify: Arc<Notify>,
    shutdown: Arc<AtomicBool>,
    cleanup_interval: Duration,
    snapshot_interval: Duration,
    last_snapshot: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl MemoryCleanupWorker {
    /// Creates a worker over a memory engine with the default intervals.
    pub fn new(engine: MemoryEngine) -> Self {
        Self {
            engine,
            notify: Arc::new(Notify::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
            cleanup_interval: CLEANUP_INTERVAL,
            snapshot_interval: SNAPSHOT_INTERVAL,
            last_snapshot: Arc::new(Mutex::new(None)),
        }
    }

    /// Overrides the cleanup interval (test hook).
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    /// Overrides the snapshot interval (test hook).
    pub fn with_snapshot_interval(mut self, interval: Duration) -> Self {
        self.snapshot_interval = interval;
        self
    }

    /// Wakes the worker so it runs a cleanup pass immediately.
    pub fn notify(&self) {
        self.notify.notify_one();
    }

    /// Requests the worker loop to stop (app-lifetime workers are simply
    /// dropped; this exists for tests).
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.notify.notify_one();
    }

    /// One cleanup pass (manual "clean up now" or worker tick).
    pub async fn run_once(
        &self,
    ) -> Result<crate::copilot::memory::models::CleanupReport, crate::errors::DatabaseError> {
        self.engine.run_cleanup().await
    }

    /// Captures an automatic snapshot when its interval has elapsed.
    pub async fn maybe_snapshot(&self) {
        let mut last = self.last_snapshot.lock().await;
        let due = match *last {
            Some(instant) => instant.elapsed() >= self.snapshot_interval,
            None => true,
        };
        if !due {
            return;
        }
        *last = Some(tokio::time::Instant::now());
        match self.engine.create_snapshot(Some("auto")).await {
            Ok(snapshot) => tracing::info!(
                snapshot = %snapshot.id,
                records = snapshot.record_count,
                "automatic memory snapshot captured"
            ),
            Err(error) => tracing::warn!(error = %error, "automatic memory snapshot failed"),
        }
    }

    /// The worker loop: cleanup passes on the interval and on
    /// notifications, snapshot captures on their own interval, until
    /// [`Self::shutdown`].
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.cleanup_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = self.notify.notified() => self.run_pass().await,
                _ = interval.tick() => {
                    self.run_pass().await;
                    self.maybe_snapshot().await;
                }
                _ = self.wait_shutdown() => break,
            }
        }
    }

    async fn run_pass(&self) {
        match self.run_once().await {
            Ok(report) => {
                if report.removed_expired
                    + report.removed_duplicate_archives
                    + report.expired_marked
                    > 0
                {
                    tracing::info!(?report, "memory cleanup pass complete");
                }
            }
            Err(error) => tracing::warn!(error = %error, "memory cleanup pass failed"),
        }
    }

    async fn wait_shutdown(&self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::engine::MemoryEngine;
    use crate::copilot::memory::models::{
        MemoryKind, MemoryOutcome, MemoryStatus, RetentionPolicy,
    };
    use crate::copilot::memory::repository::MemoryRepository;
    use crate::copilot::memory::vector::LocalVectorProvider;
    use crate::database::test_database;
    use chrono::Duration as ChronoDuration;
    use uuid::Uuid;

    fn record(goal: &str) -> crate::copilot::memory::models::ExecutionMemoryRecord {
        crate::copilot::memory::models::ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.to_string(),
            status: MemoryStatus::Success,
            plan: None,
            steps: vec![],
            reasoning: vec![],
            tools_used: vec![],
            failed_steps: vec![],
            error: None,
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            retention: RetentionPolicy::Permanent,
            retention_until: None,
            archived_at: None,
            expired_at: None,
            summary: None,
            compressed_at: None,
            version: 1,
            parent_id: None,
        }
    }

    #[tokio::test]
    async fn run_once_expires_then_removes() {
        let (database, _guard) = test_database().await;
        let engine = MemoryEngine::new(
            MemoryRepository::new(database.pool().clone()),
            Arc::new(LocalVectorProvider::default()),
        );
        let worker = MemoryCleanupWorker::new(engine.clone());

        // A temporary record past its deadline + an already-expired one.
        let temp = record("temporary goal");
        let mut expired = record("expired goal");
        expired.retention = RetentionPolicy::Expired;
        expired.expired_at = Some(chrono::Utc::now());
        engine.repository.upsert(&temp).await.unwrap();
        engine.repository.upsert(&expired).await.unwrap();
        engine
            .set_retention(
                temp.id,
                RetentionPolicy::Temporary,
                Some(chrono::Utc::now() - ChronoDuration::minutes(5)),
            )
            .await
            .unwrap();

        let report = worker.run_once().await.unwrap();
        assert_eq!(report.expired_marked, 1, "temp past deadline marked");
        assert_eq!(report.removed_expired, 2, "expired rows deleted");

        let remaining = engine.repository.list_all().await.unwrap();
        assert!(remaining.is_empty());
        assert_eq!(
            engine.repository.counts().await.unwrap().3,
            0,
            "store emptied by the pass"
        );
    }

    #[tokio::test]
    async fn maybe_snapshot_respects_the_interval() {
        let (database, _guard) = test_database().await;
        let engine = MemoryEngine::new(
            MemoryRepository::new(database.pool().clone()),
            Arc::new(LocalVectorProvider::default()),
        );
        engine.repository.upsert(&record("a goal")).await.unwrap();
        let worker = MemoryCleanupWorker::new(engine.clone())
            .with_snapshot_interval(Duration::from_secs(60));

        worker.maybe_snapshot().await; // first call: due
        worker.maybe_snapshot().await; // not due yet
        let snapshots = engine.list_snapshots().await.unwrap();
        assert_eq!(snapshots.len(), 1, "one snapshot per interval");
        assert_eq!(snapshots[0].label, "auto");
        assert_eq!(snapshots[0].record_count, 1);
    }

    #[tokio::test]
    async fn run_loop_stops_on_shutdown() {
        let (database, _guard) = test_database().await;
        let engine = MemoryEngine::new(
            MemoryRepository::new(database.pool().clone()),
            Arc::new(LocalVectorProvider::default()),
        );
        let worker =
            MemoryCleanupWorker::new(engine).with_cleanup_interval(Duration::from_secs(3600));
        let task = tokio::spawn({
            let worker = worker.clone();
            async move { worker.run().await }
        });
        worker.notify();
        tokio::time::sleep(Duration::from_millis(100)).await;
        worker.shutdown();
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("worker loop should stop after shutdown")
            .unwrap();
    }
}
