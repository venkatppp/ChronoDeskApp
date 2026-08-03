//! Live command/service/repository/worker timing profiler (RC-10 M1).
//!
//! [`PerformanceProfiler`] is the recording surface of the performance
//! subsystem: callers (performance commands, engine internals, and any
//! future integration point) submit one measured operation via
//! [`PerformanceProfiler::record`], which appends to a bounded in-memory
//! ring (the "live window" used for snapshots and p95 latency) and
//! flushes to the durable `performance_profiles` ledger. Aggregation
//! (count/avg/min/max/p95 per operation) is computed on demand from the
//! ring so snapshots never touch the database.

use std::collections::VecDeque;
use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;
use serde_json::Value;

use crate::errors::DatabaseError;
use crate::models::performance::{
    ProfileAggregate, ProfileCategory, ProfileSample, ProfileSnapshot,
};
use crate::repositories::PerformanceRepository;

/// How many samples the in-memory live window keeps per profiler.
const RING_CAPACITY: usize = 1024;

/// In-memory measured operation (id is assigned by the persistence layer).
#[derive(Debug, Clone)]
struct LiveSample {
    category: ProfileCategory,
    name: String,
    duration_ms: u64,
    metadata: Value,
    occurred_at: chrono::DateTime<Utc>,
}

/// Tracks timings of commands, services, repositories, and workers.
///
/// Cheap to clone; all state lives behind a shared lock so a single
/// profiler can be passed around the application.
#[derive(Clone)]
pub struct PerformanceProfiler {
    repository: PerformanceRepository,
    ring: Arc<RwLock<VecDeque<LiveSample>>>,
}

impl PerformanceProfiler {
    pub fn new(repository: PerformanceRepository) -> Self {
        Self {
            repository,
            ring: Arc::new(RwLock::new(VecDeque::with_capacity(RING_CAPACITY))),
        }
    }

    /// Records one measured operation into the live window and the
    /// durable ledger.
    pub async fn record(
        &self,
        category: ProfileCategory,
        name: &str,
        duration_ms: u64,
        metadata: Value,
    ) -> Result<(), DatabaseError> {
        {
            let mut ring = self.ring.write();
            if ring.len() >= RING_CAPACITY {
                ring.pop_front();
            }
            ring.push_back(LiveSample {
                category,
                name: name.to_string(),
                duration_ms,
                metadata: metadata.clone(),
                occurred_at: Utc::now(),
            });
        }
        self.repository
            .record_profile(category, name, duration_ms, &metadata)
            .await?;
        Ok(())
    }

    /// Measures and records a synchronous operation by wall-clock.
    pub async fn time<F, T>(&self, category: ProfileCategory, name: &str, op: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start = std::time::Instant::now();
        let result = op();
        let duration = start.elapsed();
        let _ = self
            .record(category, name, duration.as_millis() as u64, Value::Null)
            .await;
        result
    }

    /// Point-in-time view: aggregates, recent, and slowest samples from
    /// the live window.
    pub async fn snapshot(&self) -> Result<ProfileSnapshot, DatabaseError> {
        let ring = self.ring.read();
        let samples: Vec<LiveSample> = ring.iter().cloned().collect();
        drop(ring);

        let mut grouped: Vec<(ProfileCategory, String, Vec<u64>)> = Vec::new();
        for sample in &samples {
            if let Some((_, _, durations)) = grouped
                .iter_mut()
                .find(|(c, n, _)| *c == sample.category && *n == sample.name)
            {
                durations.push(sample.duration_ms);
            } else {
                grouped.push((
                    sample.category,
                    sample.name.clone(),
                    vec![sample.duration_ms],
                ));
            }
        }

        let mut aggregates = Vec::with_capacity(grouped.len());
        for (category, name, mut durations) in grouped {
            durations.sort_unstable();
            let count = durations.len() as u64;
            let sum: u64 = durations.iter().sum();
            aggregates.push(ProfileAggregate {
                category,
                name,
                count,
                avg_ms: sum as f64 / count as f64,
                min_ms: durations[0],
                max_ms: *durations.last().unwrap_or(&0),
                p95_ms: percentile(&durations, 0.95),
            });
        }
        aggregates.sort_by(|a, b| {
            b.p95_ms
                .partial_cmp(&a.p95_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });

        let mut recent: Vec<ProfileSample> = samples.iter().rev().take(50).map(to_sample).collect();
        recent.sort_by_key(|s| std::cmp::Reverse(s.occurred_at));

        let mut slowest: Vec<ProfileSample> = samples.iter().map(to_sample).collect();
        slowest.sort_by_key(|s| std::cmp::Reverse(s.duration_ms));
        slowest.truncate(10);

        Ok(ProfileSnapshot {
            captured_at: Utc::now(),
            aggregates,
            recent,
            slowest,
        })
    }

    /// Persisted samples from the durable ledger (history panel).
    pub async fn recent_samples(&self, limit: u32) -> Result<Vec<ProfileSample>, DatabaseError> {
        self.repository.recent_profiles(limit).await
    }

    /// Total persisted samples (memory-optimization analysis input).
    pub async fn persisted_count(&self) -> Result<u64, DatabaseError> {
        self.repository.profile_count().await
    }

    /// Prunes persisted samples older than `days` (applied memory
    /// optimization).
    pub async fn prune_older_than(&self, days: u64) -> Result<u64, DatabaseError> {
        self.repository.prune_profiles_older_than(days).await
    }

    /// Repository handle (the engine composes the profiler with the
    /// other performance subsystems).
    pub fn repository(&self) -> &PerformanceRepository {
        &self.repository
    }
}

fn to_sample(sample: &LiveSample) -> ProfileSample {
    ProfileSample {
        id: 0,
        category: sample.category,
        name: sample.name.clone(),
        duration_ms: sample.duration_ms,
        metadata: sample.metadata.clone(),
        occurred_at: sample.occurred_at,
    }
}

/// P95 (or any percentile) of a sorted duration list.
fn percentile(sorted: &[u64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f64) * q).ceil() as usize;
    let index = index.saturating_sub(1).min(sorted.len() - 1);
    sorted[index] as f64
}

#[cfg(test)]
#[path = "profiler_tests.rs"]
mod tests;
