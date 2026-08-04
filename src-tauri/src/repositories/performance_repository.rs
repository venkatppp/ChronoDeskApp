//! Performance profiling repository (RC-10 M1).
//!
//! Owns every SQL statement behind the production-hardening surfaces:
//! the sampled `performance_profiles` ledger, the persisted
//! `benchmark_runs` history, and the per-stage `startup_profiles`
//! timelines. All SQL stays here; measurement, aggregation, and policy
//! live in [`crate::performance`].

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::performance::{
    BenchmarkCategory, BenchmarkResult, ProfileCategory, ProfileSample, StartupProfile,
    StartupStage,
};

/// Raw `performance_profiles` row.
type ProfileRow = (i64, String, String, i64, String, DateTime<Utc>);
/// Raw `benchmark_runs` row.
type BenchmarkRow = (
    i64,
    String,
    String,
    String,
    String,
    i32,
    i64,
    i64,
    Option<f64>,
    String,
    DateTime<Utc>,
);
/// Raw `startup_profiles` row.
type StartupRow = (String, String, String, i64, DateTime<Utc>, DateTime<Utc>);

/// Repository for the RC-10 M1 performance ledger.
#[derive(Debug, Clone)]
pub struct PerformanceRepository {
    pool: SqlitePool,
}

impl PerformanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Profiling samples
    // ------------------------------------------------------------------

    /// Persists one sampled operation, returning its row id.
    pub async fn record_profile(
        &self,
        category: ProfileCategory,
        name: &str,
        duration_ms: u64,
        metadata: &serde_json::Value,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO performance_profiles (category, name, duration_ms, metadata)
             VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(category.as_str())
        .bind(name)
        .bind(duration_ms as i64)
        .bind(metadata.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Most recent sampled operations, newest-first.
    pub async fn recent_profiles(&self, limit: u32) -> Result<Vec<ProfileSample>, DatabaseError> {
        let rows: Vec<ProfileRow> = sqlx::query_as(
            "SELECT id, category, name, duration_ms, metadata, occurred_at
             FROM performance_profiles ORDER BY occurred_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(ProfileSample::try_from).collect()
    }

    /// Total number of persisted samples.
    pub async fn profile_count(&self) -> Result<u64, DatabaseError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM performance_profiles")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.max(0) as u64)
    }

    /// Deletes samples older than `days` (optimizer memory remediation).
    /// The cutoff is computed by SQLite itself (matching the stored
    /// `%Y-%m-%dT%H:%M:%fZ` format) so the string comparison can never
    /// drift between the Rust and SQLite timestamp representations.
    pub async fn prune_profiles_older_than(&self, days: u64) -> Result<u64, DatabaseError> {
        let modifier = format!("-{days} days");
        let result = sqlx::query(
            "DELETE FROM performance_profiles
             WHERE occurred_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)",
        )
        .bind(modifier)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Benchmarks
    // ------------------------------------------------------------------

    /// Persists one benchmark result, returning its row id.
    pub async fn record_benchmark(
        &self,
        suite_name: &str,
        benchmark: &BenchmarkResult,
    ) -> Result<i64, DatabaseError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO benchmark_runs
               (suite_name, category, benchmark_name, operation, iterations,
                duration_ms, ok, throughput_per_sec, payload)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(suite_name)
        .bind(benchmark.category.as_str())
        .bind(&benchmark.name)
        .bind(&benchmark.operation)
        .bind(benchmark.iterations as i32)
        .bind(benchmark.duration_ms as i64)
        .bind(benchmark.ok as i32)
        .bind(benchmark.throughput_per_sec)
        .bind(benchmark.payload.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Most recent benchmark results, newest-first.
    pub async fn recent_benchmarks(
        &self,
        limit: u32,
    ) -> Result<Vec<BenchmarkResult>, DatabaseError> {
        let rows: Vec<BenchmarkRow> = sqlx::query_as(
            "SELECT id, suite_name, category, benchmark_name, operation, iterations,
                    duration_ms, ok, throughput_per_sec, payload, created_at
             FROM benchmark_runs ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(BenchmarkResult::try_from).collect()
    }

    // ------------------------------------------------------------------
    // Startup profiles
    // ------------------------------------------------------------------

    /// Persists one startup run's stages (single transaction).
    pub async fn record_startup_profile(
        &self,
        run_id: Uuid,
        stages: &[StartupStage],
    ) -> Result<(), DatabaseError> {
        let mut tx = self.pool.begin().await?;
        for stage in stages {
            sqlx::query(
                "INSERT INTO startup_profiles (run_id, stage, label, duration_ms, started_at)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(run_id.to_string())
            .bind(&stage.name)
            .bind(&stage.label)
            .bind(stage.duration_ms as i64)
            .bind(
                stage
                    .started_at
                    .format("%Y-%m-%dT%H:%M:%S%.6fZ")
                    .to_string(),
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Most recent startup runs, newest-first, grouped by `run_id`.
    pub async fn recent_startup_profiles(
        &self,
        limit: u32,
    ) -> Result<Vec<StartupProfile>, DatabaseError> {
        let rows: Vec<StartupRow> = sqlx::query_as(
            "SELECT run_id, stage, label, duration_ms, started_at, recorded_at
             FROM startup_profiles ORDER BY recorded_at DESC, id DESC LIMIT ?",
        )
        .bind(limit * 4)
        .fetch_all(&self.pool)
        .await?;

        let mut runs: Vec<StartupProfile> = Vec::new();
        for (run_id, stage, label, duration_ms, started_at, recorded_at) in rows {
            let Ok(run_uuid) = Uuid::parse_str(&run_id) else {
                continue;
            };
            if let Some(profile) = runs.iter_mut().find(|p| p.run_id == run_uuid) {
                profile.stages.push(StartupStage {
                    name: stage,
                    label,
                    duration_ms: duration_ms as u64,
                    started_at,
                });
            } else if runs.len() < limit as usize {
                runs.push(StartupProfile {
                    run_id: run_uuid,
                    total_ms: duration_ms as u64,
                    stages: vec![StartupStage {
                        name: stage,
                        label,
                        duration_ms: duration_ms as u64,
                        started_at,
                    }],
                    recorded_at,
                });
            }
        }
        for profile in &mut runs {
            profile.stages.sort_by_key(|s| s.started_at);
            profile.total_ms = profile.stages.iter().map(|s| s.duration_ms).sum();
        }
        Ok(runs)
    }

    // ------------------------------------------------------------------
    // Database footprint
    // ------------------------------------------------------------------

    /// On-disk database size in bytes (`page_count * page_size`).
    pub async fn db_size_bytes(&self) -> Result<u64, DatabaseError> {
        let (pages,): (i64,) = sqlx::query_as("PRAGMA page_count")
            .fetch_one(&self.pool)
            .await?;
        let (page_size,): (i64,) = sqlx::query_as("PRAGMA page_size")
            .fetch_one(&self.pool)
            .await?;
        Ok((pages.max(0) as u64).saturating_mul(page_size.max(0) as u64))
    }
}

impl TryFrom<ProfileRow> for ProfileSample {
    type Error = DatabaseError;

    fn try_from(
        (id, category, name, duration_ms, metadata, occurred_at): ProfileRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id,
            category: ProfileCategory::from(category.as_str()),
            name,
            duration_ms: duration_ms as u64,
            metadata: serde_json::from_str(&metadata).unwrap_or(serde_json::Value::Null),
            occurred_at,
        })
    }
}

impl TryFrom<BenchmarkRow> for BenchmarkResult {
    type Error = DatabaseError;

    fn try_from(
        (
            id,
            _suite_name,
            category,
            name,
            operation,
            iterations,
            duration_ms,
            ok,
            throughput_per_sec,
            payload,
            created_at,
        ): BenchmarkRow,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            id,
            name,
            operation,
            category: BenchmarkCategory::from(category.as_str()),
            iterations: iterations.max(0) as u32,
            duration_ms: duration_ms as u64,
            throughput_per_sec,
            ok: ok != 0,
            payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
            created_at,
        })
    }
}

#[cfg(test)]
#[path = "performance_repository_tests.rs"]
mod tests;
