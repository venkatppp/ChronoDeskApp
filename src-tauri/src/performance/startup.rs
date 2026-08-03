//! Startup phase timing (RC-10 M1).
//!
//! [`StartupProfiler`] measures how long each initialization stage of the
//! backend takes (database, graph sync, engines, workers, ...). It is
//! created before `Database::initialize` resolves in `lib.rs`, records a
//! start/end marker around each stage with no `?`/await coupling (the
//! markers are infallible, so an early return cannot break the flow),
//! and `finish`es by persisting one run (grouped by `run_id`) once every
//! subsystem is up. The completed run is also kept in memory so the
//! `performance_startup` command can return it without a database read.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::performance::{StartupProfile, StartupStage};
use crate::repositories::PerformanceRepository;

/// One in-progress or completed startup phase.
#[derive(Debug, Clone)]
struct StageEntry {
    name: String,
    label: String,
    started_at: DateTime<Utc>,
    started: Option<std::time::Instant>,
    duration_ms: u64,
}

/// Times backend initialization stage by stage.
#[derive(Clone)]
pub struct StartupProfiler {
    inner: Arc<Mutex<StartupProfilerInner>>,
}

struct StartupProfilerInner {
    run_id: Uuid,
    stages: Vec<StageEntry>,
    completed: Option<StartupProfile>,
}

impl StartupProfiler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StartupProfilerInner {
                run_id: Uuid::new_v4(),
                stages: Vec::new(),
                completed: None,
            })),
        }
    }

    /// Starts measuring a stage. Infallible: a setup path that returns
    /// early simply leaves the stage without a matching end, and `finish`
    /// drops unfinished stages.
    pub fn stage_start(&self, name: &str, label: &str) {
        let mut inner = self.inner.lock();
        inner.stages.push(StageEntry {
            name: name.to_string(),
            label: label.to_string(),
            started_at: Utc::now(),
            started: Some(std::time::Instant::now()),
            duration_ms: 0,
        });
    }

    /// Stops measuring the innermost open stage.
    pub fn stage_end(&self) {
        let mut inner = self.inner.lock();
        if let Some(stage) = inner.stages.iter_mut().rev().find(|s| s.started.is_some()) {
            if let Some(started) = stage.started.take() {
                stage.duration_ms = started.elapsed().as_millis() as u64;
            }
        }
    }

    /// Measures and records a sync stage with a closure (no `?` inside;
    /// closures keep the setup path structurally unchanged).
    pub fn stage<T>(&self, name: &str, label: &str, op: impl FnOnce() -> T) -> T {
        self.stage_start(name, label);
        let result = op();
        self.stage_end();
        result
    }

    /// Persists the run and exposes the final report. Stages still open
    /// (an early `?` returned before their `stage_end`) are dropped so a
    /// half-initialized launch still yields a clean timeline. Each call
    /// finalizes the current run and starts a fresh one for any later
    /// stages.
    pub async fn finish(
        &self,
        repository: &PerformanceRepository,
    ) -> Result<StartupProfile, DatabaseError> {
        let (run_id, recorded_at, stages) = {
            let mut inner = self.inner.lock();
            let complete: Vec<StageEntry> = inner
                .stages
                .drain(..)
                .filter(|s| s.started.is_none())
                .collect();
            // Rotate the run id so a subsequent profiling session (e.g.
            // in tests) groups under its own run.
            let run_id = inner.run_id;
            inner.run_id = Uuid::new_v4();
            (run_id, Utc::now(), complete)
        };

        let mut profile_stages = Vec::with_capacity(stages.len());
        for stage in stages {
            profile_stages.push(StartupStage {
                name: stage.name,
                label: stage.label,
                duration_ms: stage.duration_ms,
                started_at: stage.started_at,
            });
        }
        profile_stages.sort_by_key(|s| s.started_at);
        let profile = StartupProfile {
            run_id,
            total_ms: profile_stages.iter().map(|s| s.duration_ms).sum(),
            stages: profile_stages,
            recorded_at,
        };

        repository
            .record_startup_profile(run_id, &profile.stages)
            .await?;
        self.inner.lock().completed = Some(profile.clone());
        Ok(profile)
    }

    /// The most recent completed startup report (after `finish`).
    pub fn latest(&self) -> Option<StartupProfile> {
        self.inner.lock().completed.clone()
    }

    /// The id of the current run (used for debug tracing).
    pub fn run_id(&self) -> Uuid {
        self.inner.lock().run_id
    }
}

impl Default for StartupProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
