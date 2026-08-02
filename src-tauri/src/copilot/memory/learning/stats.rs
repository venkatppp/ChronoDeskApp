//! Learning statistics (RC-6 M3) — exposes **learning health** through
//! IPC: how confident the system is in its memories, how good its
//! workflows are, whether success is trending, and how well the store is
//! utilized. Built from the pure rules in this module family so there is
//! exactly one source of truth for every metric.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::memory::learning::aging::aging_summary;
use crate::copilot::memory::learning::confidence::confidence_score;
use crate::copilot::memory::learning::core::{compute_stats, learned_workflows};
use crate::copilot::memory::models::{ExecutionMemoryRecord, MemoryAcceptance, MemoryStatus};

/// Days of success history covered by the trend chart.
pub const TREND_DAYS: i64 = 14;

/// One day of success history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessTrend {
    /// ISO date (YYYY-MM-DD, UTC).
    pub date: String,
    pub successes: u64,
    pub failures: u64,
    pub success_rate: f64,
}

/// Aggregate workflow quality metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowQuality {
    /// Distinct workflow fingerprints learned.
    pub workflow_count: u64,
    /// Mean success rate across workflows (0..1).
    pub avg_success_rate: f64,
    /// Mean planner confidence across remembered plans (0..1).
    pub avg_plan_confidence: f64,
    /// Mean completion time of remembered runs (seconds, 0 when unknown).
    pub avg_duration_seconds: u64,
    /// Share of workflows that have been replayed at least once.
    pub replay_adoption_rate: f64,
    /// Share of replays per successful run (reuse intensity).
    pub replay_per_run: f64,
}

/// How well the memory store is being used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUtilization {
    pub total_records: u64,
    pub active_records: u64,
    pub aging_records: u64,
    pub archived_records: u64,
    pub avg_freshness: f64,
    pub utilization_ratio: f64,
    pub workflows_per_record: f64,
}

/// The learning health payload for the dashboard (RC-6 M3).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LearningHealth {
    /// Mean recommendation confidence across the store (0..1).
    pub confidence_average: f64,
    /// Mean recommendation confidence of successful memories.
    pub confidence_successful: f64,
    /// Overall user acceptance rate of recommendations (0..1).
    pub acceptance_rate: f64,
    pub workflow_quality: WorkflowQuality,
    pub success_trends: Vec<SuccessTrend>,
    pub memory_utilization: MemoryUtilization,
    /// Mean learned-score across the store (0..1).
    pub score_average: f64,
}

/// Computes the learning health of the store.
pub fn learning_health(
    records: &[ExecutionMemoryRecord],
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
    now_ms: i64,
) -> LearningHealth {
    let mut health = LearningHealth::default();
    if records.is_empty() {
        return health;
    }

    // Confidence averages (self-similarity: how trustworthy is each
    // memory for its own goal, on average).
    let mut confidence_sum = 0.0;
    let mut confidence_success_sum = 0.0;
    let mut success_count = 0.0;
    for record in records {
        let result = confidence_score(record, 1.0, records, acceptance, now_ms);
        confidence_sum += result.score;
        if record.status == MemoryStatus::Success {
            confidence_success_sum += result.score;
            success_count += 1.0;
        }
    }
    health.confidence_average = confidence_sum / records.len() as f64;
    health.confidence_successful = if success_count > 0.0 {
        confidence_success_sum / success_count
    } else {
        0.0
    };

    // Acceptance rate over the ledger.
    let with_feedback: Vec<f64> = records
        .iter()
        .filter_map(|record| acceptance.get(&record.id))
        .map(MemoryAcceptance::rate)
        .collect();
    health.acceptance_rate = if with_feedback.is_empty() {
        0.0
    } else {
        with_feedback.iter().sum::<f64>() / with_feedback.len() as f64
    };

    health.workflow_quality = workflow_quality(records);
    health.success_trends = success_trends(records, now_ms);
    health.memory_utilization = memory_utilization(records, now_ms);
    health.score_average = score_average(records, acceptance, now_ms);
    health
}

/// Workflow quality from the aggregated workflows.
fn workflow_quality(records: &[ExecutionMemoryRecord]) -> WorkflowQuality {
    let workflows = learned_workflows(records);
    let mut quality = WorkflowQuality {
        workflow_count: workflows.len() as u64,
        ..WorkflowQuality::default()
    };
    if workflows.is_empty() {
        return quality;
    }

    let mut success_rate_sum = 0.0;
    let mut replayed = 0u64;
    let mut confidence_sum = 0.0;
    let mut confidence_count = 0u64;
    let mut duration_sum = 0u64;
    let mut duration_count = 0u64;
    let mut successful_runs = 0u64;

    for workflow in &workflows {
        let total = workflow.success_count + workflow.failure_count;
        if total > 0 {
            success_rate_sum += workflow.success_count as f64 / total as f64;
        }
        if workflow.success_count > 0 {
            replayed += 1;
        }
        successful_runs += workflow.success_count;
    }
    for record in records {
        if let Some(plan) = &record.plan {
            confidence_sum += plan.confidence;
            confidence_count += 1;
        }
        if record.outcome.duration_seconds > 0 {
            duration_sum += record.outcome.duration_seconds;
            duration_count += 1;
        }
    }

    quality.avg_success_rate = success_rate_sum / workflows.len() as f64;
    quality.avg_plan_confidence = if confidence_count > 0 {
        confidence_sum / confidence_count as f64
    } else {
        0.0
    };
    quality.avg_duration_seconds = duration_sum.checked_div(duration_count).unwrap_or(0);
    quality.replay_adoption_rate = replayed as f64 / workflows.len() as f64;
    quality.replay_per_run = if successful_runs > 0 {
        records.iter().map(|r| r.replay_count).sum::<u64>() as f64 / successful_runs as f64
    } else {
        0.0
    };
    quality
}

/// Per-day success/failure counts for the last [`TREND_DAYS`] days.
fn success_trends(records: &[ExecutionMemoryRecord], now_ms: i64) -> Vec<SuccessTrend> {
    let day_ms = 24 * 60 * 60 * 1000;
    let now_days = now_ms / day_ms;
    let mut by_day: HashMap<i64, (u64, u64)> = HashMap::new();
    for record in records {
        let day = record.created_at.timestamp_millis() / day_ms;
        if now_days - day >= TREND_DAYS || day > now_days {
            continue;
        }
        let entry = by_day.entry(day).or_insert((0, 0));
        match record.status {
            MemoryStatus::Success => entry.0 += 1,
            MemoryStatus::Failed => entry.1 += 1,
            MemoryStatus::Cancelled => {}
        }
    }

    let mut trends = Vec::with_capacity(TREND_DAYS as usize);
    for offset in (0..TREND_DAYS).rev() {
        let day = now_days - offset;
        let (successes, failures) = by_day.get(&day).copied().unwrap_or((0, 0));
        let total = successes + failures;
        let date = chrono::DateTime::from_timestamp(day * day_ms / 1000, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        trends.push(SuccessTrend {
            date,
            successes,
            failures,
            success_rate: if total > 0 {
                successes as f64 / total as f64
            } else {
                0.0
            },
        });
    }
    trends
}

/// Memory utilization from the aging buckets.
fn memory_utilization(records: &[ExecutionMemoryRecord], now_ms: i64) -> MemoryUtilization {
    let aging = aging_summary(records, now_ms);
    let total = aging.total_records.max(1);
    MemoryUtilization {
        total_records: aging.total_records,
        active_records: aging.fresh_records,
        aging_records: aging.aging_records,
        archived_records: aging.archived_records,
        avg_freshness: aging.avg_freshness,
        utilization_ratio: aging.fresh_records as f64 / total as f64,
        workflows_per_record: compute_stats(records).learned_workflows as f64 / total as f64,
    }
}

/// Mean learned score across the store (self-similarity).
fn score_average(
    records: &[ExecutionMemoryRecord],
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
    now_ms: i64,
) -> f64 {
    let weights =
        crate::copilot::memory::learning::weights::learn_weights(records, acceptance, now_ms);
    let mut sum = 0.0;
    for record in records {
        sum += crate::copilot::memory::learning::core::learned_score(
            record, 1.0, records, acceptance, &weights, now_ms,
        );
    }
    sum / records.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn record(goal: &str, status: MemoryStatus, days_old: i64) -> ExecutionMemoryRecord {
        let created = Utc::now() - Duration::days(days_old);
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.into(),
            status,
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
    fn health_averages_confidence_over_the_store() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("resume focus", MemoryStatus::Success, 0),
            record("resume focus", MemoryStatus::Failed, 1),
        ];
        let health = learning_health(&records, &HashMap::new(), now);
        assert!(health.confidence_average > 0.0 && health.confidence_average <= 1.0);
        assert!(health.confidence_successful > health.confidence_average);
    }

    #[test]
    fn health_reports_workflow_quality() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("resume focus", MemoryStatus::Success, 0),
            record("resume focus", MemoryStatus::Success, 1),
            record("organize receipts", MemoryStatus::Failed, 2),
        ];
        let health = learning_health(&records, &HashMap::new(), now);
        assert_eq!(health.workflow_quality.workflow_count, 2);
        assert!(health.workflow_quality.avg_success_rate > 0.0);
        assert!(health.memory_utilization.total_records >= 3);
    }

    #[test]
    fn success_trends_cover_last_two_weeks() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Failed, 0),
        ];
        let trends = success_trends(&records, now);
        assert_eq!(trends.len(), TREND_DAYS as usize);
        assert_eq!(trends.last().unwrap().successes, 2);
        assert_eq!(trends.last().unwrap().failures, 1);
        assert!(trends.last().unwrap().success_rate > 0.0);
    }

    #[test]
    fn empty_store_has_zero_health() {
        let health = learning_health(&[], &HashMap::new(), 0);
        assert_eq!(health.confidence_average, 0.0);
        assert_eq!(health.workflow_quality.workflow_count, 0);
        assert!(health.success_trends.is_empty());
    }
}
