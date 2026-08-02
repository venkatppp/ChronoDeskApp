//! Memory aging — instead of treating every memory equally, memories
//! decay over time (freshness), and memories past the archival horizon
//! carry an archival weight so they rank below fresh equivalents.
//!
//! - `freshness`: exponential decay with a 30-day half-life (1 = just
//!   captured, → 0 over time). Replaces the fixed recency factor.
//! - `archival_weight`: 1.0 for current memories, 0.3 once the record is
//!   older than `ARCHIVE_AFTER_DAYS`. Multiplies learned scores and
//!   confidence so aged knowledge fades from recommendations.
//! - `aging_summary`: dashboard buckets (fresh / aging / archived) plus
//!   the average freshness of the store.

use serde::{Deserialize, Serialize};

use crate::copilot::memory::models::ExecutionMemoryRecord;

/// Half-life of the freshness decay.
pub const FRESHNESS_HALF_LIFE_DAYS: i64 = 30;

/// Age at which a memory becomes "aging" (freshness below half).
pub const AGING_AFTER_DAYS: i64 = 30;

/// Age at which a memory is considered archived.
pub const ARCHIVE_AFTER_DAYS: i64 = 180;

/// Score multiplier for archived memories.
pub const ARCHIVE_WEIGHT: f64 = 0.3;

/// Exponential freshness of a record (1 = just captured, → 0).
/// `now_ms` is injected so tests stay deterministic.
pub fn freshness(record: &ExecutionMemoryRecord, now_ms: i64) -> f64 {
    let age_ms = (now_ms - record.created_at.timestamp_millis()).max(0);
    let half_life_ms = FRESHNESS_HALF_LIFE_DAYS * 24 * 60 * 60 * 1000;
    (-(age_ms as f64) / half_life_ms as f64).exp()
}

/// Age of the record in days (0 for future timestamps).
pub fn age_days(record: &ExecutionMemoryRecord, now_ms: i64) -> i64 {
    (now_ms - record.created_at.timestamp_millis()).max(0) / (24 * 60 * 60 * 1000)
}

/// Archival weight (0.3 once older than [`ARCHIVE_AFTER_DAYS`], else 1.0).
pub fn archival_weight(record: &ExecutionMemoryRecord, now_ms: i64) -> f64 {
    if age_days(record, now_ms) >= ARCHIVE_AFTER_DAYS {
        ARCHIVE_WEIGHT
    } else {
        1.0
    }
}

/// Combined aging factor used by the confidence engine: freshness with a
/// floor lifted by archival status — archived memories can never be
/// "fresh" even when recently updated, because their history is old.
pub fn aging_factor(record: &ExecutionMemoryRecord, now_ms: i64) -> f64 {
    let fresh = freshness(record, now_ms);
    if archival_weight(record, now_ms) < 1.0 {
        fresh.min(ARCHIVE_WEIGHT)
    } else {
        fresh
    }
}

/// Dashboard snapshot of how the memory store is aging.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryAgingSummary {
    /// Total remembered runs.
    pub total_records: u64,
    /// Records younger than `AGING_AFTER_DAYS`.
    pub fresh_records: u64,
    /// Records between `AGING_AFTER_DAYS` and `ARCHIVE_AFTER_DAYS`.
    pub aging_records: u64,
    /// Records past `ARCHIVE_AFTER_DAYS`.
    pub archived_records: u64,
    /// Average freshness across the store (0..1).
    pub avg_freshness: f64,
    /// Oldest record age in days.
    pub oldest_days: i64,
    /// Newest record age in days.
    pub newest_days: i64,
}

/// Buckets the store by age and computes the average freshness.
pub fn aging_summary(records: &[ExecutionMemoryRecord], now_ms: i64) -> MemoryAgingSummary {
    let mut summary = MemoryAgingSummary {
        total_records: records.len() as u64,
        ..MemoryAgingSummary::default()
    };
    if records.is_empty() {
        return summary;
    }

    let mut freshness_sum = 0.0;
    let mut oldest = i64::MIN;
    let mut newest = i64::MAX;
    for record in records {
        let days = age_days(record, now_ms);
        if days >= ARCHIVE_AFTER_DAYS {
            summary.archived_records += 1;
        } else if days >= AGING_AFTER_DAYS {
            summary.aging_records += 1;
        } else {
            summary.fresh_records += 1;
        }
        freshness_sum += freshness(record, now_ms);
        oldest = oldest.max(days);
        newest = newest.min(days);
    }
    summary.avg_freshness = freshness_sum / records.len() as f64;
    summary.oldest_days = oldest.max(0);
    summary.newest_days = newest.max(0);
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, MemoryStatus};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn record(days_old: i64) -> ExecutionMemoryRecord {
        let created = Utc::now() - Duration::days(days_old);
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: "g".into(),
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
            created_at: created,
            updated_at: created,
        }
    }

    #[test]
    fn freshness_decays_exponentially() {
        let now = Utc::now().timestamp_millis();
        let fresh = record(0);
        let old = record(60);
        assert!(freshness(&fresh, now) > 0.9);
        assert!(freshness(&old, now) < 0.3);
        assert!(freshness(&old, now) > 0.0);
    }

    #[test]
    fn archival_weight_flips_after_horizon() {
        let now = Utc::now().timestamp_millis();
        assert_eq!(archival_weight(&record(179), now), 1.0);
        assert_eq!(archival_weight(&record(180), now), ARCHIVE_WEIGHT);
        assert_eq!(archival_weight(&record(400), now), ARCHIVE_WEIGHT);
    }

    #[test]
    fn aging_summary_buckets_the_store() {
        let now = Utc::now().timestamp_millis();
        let records = vec![record(1), record(45), record(200), record(500)];
        let summary = aging_summary(&records, now);
        assert_eq!(summary.total_records, 4);
        assert_eq!(summary.fresh_records, 1);
        assert_eq!(summary.aging_records, 1);
        assert_eq!(summary.archived_records, 2);
        assert!(summary.avg_freshness > 0.0 && summary.avg_freshness <= 1.0);
        assert_eq!(summary.oldest_days, 500);
        assert_eq!(summary.newest_days, 1);
        assert_eq!(aging_summary(&[], now).total_records, 0);
    }
}
