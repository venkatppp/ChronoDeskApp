//! Learning core — the pure ranking and aggregation rules over execution
//! memory that the rest of the learning modules build on:
//! relevance thresholds, the learned-score blend, workflow aggregation,
//! and the strategies-to-avoid list. RC-6 M3: the blend no longer uses
//! fixed constants — it takes the adaptive [`LearningWeights`] learned
//! from store history (see `weights`) and the acceptance ledger.

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::copilot::memory::learning::aging::{archival_weight, freshness};
use crate::copilot::memory::learning::weights::LearningWeights;
use crate::copilot::memory::models::{
    goal_fingerprint, AvoidedStrategy, ExecutionMemoryRecord, LearnedWorkflow, MemoryAcceptance,
    MemoryHit, MemoryKind, MemoryStats, MemoryStatus,
};
use crate::copilot::memory::retrieval::goal_similarity;

/// Similarity threshold below which a remembered run is not considered
/// relevant for the queried goal.
pub const RELEVANCE_THRESHOLD: f64 = 0.25;

/// Similarity threshold for *recommending* a remembered workflow for reuse
/// (used by the planner; a strong match means "reuse this plan").
pub const RECOMMENDATION_THRESHOLD: f64 = 0.6;

/// Replays beyond this count saturate the replay factor at 1.0.
const REPLAY_SATURATION: u64 = 10;

/// Success rate of the goal fingerprint a record belongs to (0..1).
/// Includes every remembered run of the same goal, so a workflow that
/// failed last time but mostly succeeded still scores well.
pub fn fingerprint_success_rate(
    record: &ExecutionMemoryRecord,
    records: &[ExecutionMemoryRecord],
) -> f64 {
    let fingerprint = goal_fingerprint(&record.goal);
    let mut successes = 0u64;
    let mut total = 0u64;
    for candidate in records {
        if goal_fingerprint(&candidate.goal) != fingerprint {
            continue;
        }
        total += 1;
        if candidate.status == MemoryStatus::Success {
            successes += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        successes as f64 / total as f64
    }
}

/// Normalized replay factor for a record (1 = replayed often, 0 = never).
fn replay_factor(record: &ExecutionMemoryRecord) -> f64 {
    (record.replay_count as f64 / REPLAY_SATURATION as f64).min(1.0)
}

/// Normalized duration factor: shorter runs score higher (1.0 for
/// unknown/instant, decaying with a 2 h half-life). Lets the learned
/// blend prefer workflows that completed quickly.
fn duration_factor(record: &ExecutionMemoryRecord) -> f64 {
    const HALF_LIFE_SECONDS: f64 = 2.0 * 60.0 * 60.0;
    let duration = record.outcome.duration_seconds as f64;
    if duration <= 0.0 {
        1.0
    } else {
        (-duration / HALF_LIFE_SECONDS).exp()
    }
}

/// The user's acceptance rate for a record (0..1). Records without any
/// recorded feedback assume a neutral 0.5 until evidence arrives.
fn acceptance_rate_for(
    record: &ExecutionMemoryRecord,
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
) -> f64 {
    acceptance
        .get(&record.id)
        .map(MemoryAcceptance::rate)
        .unwrap_or(0.5)
}

/// Ranks remembered runs by relevance for a goal with the learned blend
/// (similarity, success rate, recency, replay, acceptance, duration,
/// failure history) under the given adaptive weights. Success-only by
/// default; pass `include_failures` to surface failure history too.
#[allow(clippy::too_many_arguments)] // one call site in the facade; the payload is the point
pub fn rank_historical(
    goal: &str,
    query_embedding: Option<&[f32]>,
    records: &[ExecutionMemoryRecord],
    include_failures: bool,
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
    weights: &LearningWeights,
    now_ms: i64,
) -> Vec<MemoryHit> {
    let mut hits: Vec<MemoryHit> = records
        .iter()
        .filter(|record| {
            if !include_failures && record.status != MemoryStatus::Success {
                return false;
            }
            goal_similarity(goal, query_embedding, record) >= RELEVANCE_THRESHOLD
        })
        .map(|record| MemoryHit {
            record: record.clone(),
            similarity: goal_similarity(goal, query_embedding, record),
        })
        .collect();

    hits.sort_by(|a, b| {
        let score_a = learned_score(
            &a.record,
            a.similarity,
            records,
            acceptance,
            weights,
            now_ms,
        );
        let score_b = learned_score(
            &b.record,
            b.similarity,
            records,
            acceptance,
            weights,
            now_ms,
        );
        score_b
            .total_cmp(&score_a)
            .then_with(|| b.record.created_at.cmp(&a.record.created_at))
    });
    hits
}

/// The blended "learned score" for a candidate: weighted similarity,
/// success history, recency, replay history, user acceptance, duration,
/// and failure history — with the weights learned from store history —
/// then scaled by the record's archival weight so aged memories rank
/// below fresh equivalents (memory aging, RC-6 M3).
pub fn learned_score(
    record: &ExecutionMemoryRecord,
    similarity: f64,
    records: &[ExecutionMemoryRecord],
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
    weights: &LearningWeights,
    now_ms: i64,
) -> f64 {
    let success_rate = fingerprint_success_rate(record, records);
    let blend = weights.similarity * similarity
        + weights.success * success_rate
        + weights.recency * freshness(record, now_ms)
        + weights.replay * replay_factor(record)
        + weights.acceptance * acceptance_rate_for(record, acceptance)
        + weights.duration * duration_factor(record)
        + weights.failure * (1.0 - success_rate);
    (blend * archival_weight(record, now_ms)).clamp(0.0, 1.0)
}

/// Aggregates repeated goals into learned workflows with their history.
pub fn learned_workflows(records: &[ExecutionMemoryRecord]) -> Vec<LearnedWorkflow> {
    let mut grouped: HashMap<String, Vec<&ExecutionMemoryRecord>> = HashMap::new();
    for record in records {
        grouped
            .entry(goal_fingerprint(&record.goal))
            .or_default()
            .push(record);
    }

    let mut workflows = Vec::new();
    for (fingerprint, members) in grouped {
        let mut success_count = 0u64;
        let mut failure_count = 0u64;
        let mut best_plan = None;
        let mut last_success_at = None;
        let mut latest_goal = String::new();

        let mut newest: Option<&ExecutionMemoryRecord> = None;
        for member in members {
            match member.status {
                MemoryStatus::Success => success_count += 1,
                MemoryStatus::Failed => failure_count += 1,
                MemoryStatus::Cancelled => {}
            }
            if member.status == MemoryStatus::Success {
                if best_plan.is_none() && member.plan.is_some() {
                    best_plan = member.plan.clone();
                }
                if last_success_at.map_or(true, |t: chrono::DateTime<Utc>| member.created_at > t) {
                    last_success_at = Some(member.created_at);
                }
            }
            if newest.map_or(true, |n: &ExecutionMemoryRecord| {
                member.created_at > n.created_at
            }) {
                newest = Some(member);
            }
        }
        if let Some(newest) = newest {
            latest_goal = newest.goal.clone();
        }

        workflows.push(LearnedWorkflow {
            goal_fingerprint: fingerprint,
            goal: latest_goal,
            success_count,
            failure_count,
            best_plan,
            last_success_at,
        });
    }

    workflows.sort_by(|a, b| {
        b.success_count
            .cmp(&a.success_count)
            .then_with(|| b.goal_fingerprint.cmp(&a.goal_fingerprint))
    });
    workflows
}

/// Failed/cancelled runs relevant to a goal, ranked by relevance — the
/// strategies the runtime should avoid repeating.
pub fn avoid_strategies(
    goal: &str,
    query_embedding: Option<&[f32]>,
    records: &[ExecutionMemoryRecord],
    limit: usize,
) -> Vec<AvoidedStrategy> {
    let mut avoided: Vec<AvoidedStrategy> = records
        .iter()
        .filter(|record| record.status != MemoryStatus::Success)
        .map(|record| AvoidedStrategy {
            record: record.clone(),
            similarity: goal_similarity(goal, query_embedding, record),
            failure: record
                .error
                .clone()
                .or_else(|| {
                    record
                        .first_failed_tool()
                        .map(|tool| format!("tool '{tool}' failed"))
                })
                .unwrap_or_else(|| "run was cancelled".to_string()),
        })
        .filter(|candidate| candidate.similarity >= RELEVANCE_THRESHOLD)
        .collect();

    avoided.sort_by(|a, b| {
        b.similarity
            .total_cmp(&a.similarity)
            .then_with(|| b.record.created_at.cmp(&a.record.created_at))
    });
    avoided.truncate(limit);
    avoided
}

/// Builds dashboard statistics from the raw records.
pub fn compute_stats(records: &[ExecutionMemoryRecord]) -> MemoryStats {
    let mut stats = MemoryStats {
        total_records: records.len() as u64,
        ..MemoryStats::default()
    };
    for record in records {
        match record.status {
            MemoryStatus::Success => stats.successful += 1,
            MemoryStatus::Failed => stats.failed += 1,
            MemoryStatus::Cancelled => stats.cancelled += 1,
        }
        match record.kind {
            MemoryKind::Execution => stats.executions += 1,
            MemoryKind::PlannerReport => stats.planner_reports += 1,
            MemoryKind::AutonomousSession => stats.autonomous_sessions += 1,
        }
        stats.total_replays += record.replay_count;
    }
    stats.learned_workflows = learned_workflows(records).len() as u64;
    stats
}
