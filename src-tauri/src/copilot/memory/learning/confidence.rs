//! Confidence Engine (RC-6 M3) — every recommendation exposes a
//! `confidence_score` (0..1) built from five explainable factors:
//! similarity, success history, replay history, freshness, and usage
//! count, then scaled by the memory's archival weight. Each factor
//! contributes an [`ExplanationReason`] so the dashboard can show *why*
//! a workflow is (or is not) trusted.

use std::collections::HashMap;

use uuid::Uuid;

use crate::copilot::memory::learning::aging::{aging_factor, archival_weight};
use crate::copilot::memory::learning::core::fingerprint_success_rate;
use crate::copilot::memory::models::{ExecutionMemoryRecord, MemoryAcceptance};
use crate::learning::models::ExplanationReason;

/// Weights of the confidence factors (normalized).
const FACTOR_WEIGHTS: [(f64, f64, &str); 5] = [
    // (weight, neutral_value, factor name)
    (0.30, 0.5, "similarity"),
    (0.25, 0.5, "success_history"),
    (0.15, 0.0, "replay_history"),
    (0.20, 0.5, "freshness"),
    (0.10, 1.0, "usage_count"),
];

/// Result of a confidence evaluation.
#[derive(Debug, Clone)]
pub struct ConfidenceResult {
    /// Blended confidence, 0..1 (archival-weighted).
    pub score: f64,
    /// Why the score is what it is, per factor.
    pub factors: Vec<ExplanationReason>,
}

/// Replays beyond this count saturate the replay factor at 1.0.
const REPLAY_SATURATION: u64 = 5;
/// Fingerprint runs beyond this count saturate the usage factor at 1.0.
const USAGE_SATURATION: u64 = 10;

/// Computes the confidence score for a candidate recommendation.
///
/// `similarity` is the query-goal similarity (0..1); `records` is the
/// store the record belongs to (used for fingerprint-level history).
pub fn confidence_score(
    record: &ExecutionMemoryRecord,
    similarity: f64,
    records: &[ExecutionMemoryRecord],
    _acceptance: &HashMap<Uuid, MemoryAcceptance>,
    now_ms: i64,
) -> ConfidenceResult {
    let success_rate = fingerprint_success_rate(record, records);
    let replay_factor = (record.replay_count as f64 / REPLAY_SATURATION as f64).min(1.0);
    let usage_count = records.iter().filter(|r| r.goal == record.goal).count() as f64;
    let usage_factor = (usage_count / USAGE_SATURATION as f64).min(1.0);
    let aging = aging_factor(record, now_ms);

    let values: [f64; 5] = [similarity, success_rate, replay_factor, aging, usage_factor];

    let mut score = 0.0;
    let mut factors = Vec::with_capacity(5);
    for ((weight, neutral, name), value) in FACTOR_WEIGHTS.iter().zip(values) {
        score += weight * value;
        factors.push(ExplanationReason {
            factor: (*name).to_string(),
            impact: weight * (value - neutral),
            description: describe_factor(name, value, record),
        });
    }
    let archived = archival_weight(record, now_ms) < 1.0;
    if archived {
        score *= archival_weight(record, now_ms);
        factors.push(ExplanationReason {
            factor: "memory_archival".to_string(),
            impact: archival_weight(record, now_ms) - 1.0,
            description: "Memory is past the archival horizon and weighted down".to_string(),
        });
    }

    ConfidenceResult {
        score: score.clamp(0.0, 1.0),
        factors,
    }
}

fn describe_factor(name: &str, value: f64, record: &ExecutionMemoryRecord) -> String {
    match name {
        "similarity" => format!("Goal similarity is {:.0}%", value * 100.0),
        "success_history" => format!("{:.0}% of runs of this goal succeeded", value * 100.0),
        "replay_history" => format!("Replayed {} time(s); proven by reuse", record.replay_count),
        "freshness" => {
            if value > 0.5 {
                "Memory is recent".to_string()
            } else if value > 0.0 {
                "Memory is aging".to_string()
            } else {
                "Memory has decayed".to_string()
            }
        }
        "usage_count" => format!(
            "Workflow has been used enough to be trusted ({:.0}%)",
            value * 100.0
        ),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, MemoryStatus};
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    fn record(
        goal: &str,
        status: MemoryStatus,
        days_old: i64,
        replays: u64,
    ) -> ExecutionMemoryRecord {
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
            replay_count: replays,
            created_at: created,
            updated_at: created,
        }
    }

    #[test]
    fn confidence_blends_the_five_factors() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("resume focus", MemoryStatus::Success, 0, 3),
            record("resume focus", MemoryStatus::Success, 1, 1),
        ];
        let result = confidence_score(&records[0], 1.0, &records, &HashMap::new(), now);
        assert!(
            result.score > 0.6,
            "strong record scores high: {}",
            result.score
        );
        assert!(result.score <= 1.0);
        // All five factors are explained.
        let factors: Vec<&str> = result.factors.iter().map(|f| f.factor.as_str()).collect();
        for name in [
            "similarity",
            "success_history",
            "replay_history",
            "freshness",
            "usage_count",
        ] {
            assert!(
                factors.contains(&name),
                "missing factor {name}: {factors:?}"
            );
        }
    }

    #[test]
    fn confidence_rewards_replay_and_punishes_age() {
        let now = Utc::now().timestamp_millis();
        let fresh_replayed = record("g", MemoryStatus::Success, 0, 4);
        let aged = record("g", MemoryStatus::Success, 400, 0);
        let records = vec![fresh_replayed.clone(), aged.clone()];

        let young = confidence_score(&fresh_replayed, 1.0, &records, &HashMap::new(), now);
        let old = confidence_score(&aged, 1.0, &records, &HashMap::new(), now);
        assert!(
            young.score > old.score,
            "fresh replayed must outscore archived"
        );
        assert!(old.score < 0.5, "archived memory confidence must decay");
        assert!(
            old.factors.iter().any(|f| f.factor == "memory_archival"),
            "archival factor must be explained"
        );
    }

    #[test]
    fn failing_history_lowers_confidence() {
        let now = Utc::now().timestamp_millis();
        // Two distinct goals: one with a clean history, one that keeps
        // failing — the fingerprint-level history drives the difference.
        let ok = record("stable goal", MemoryStatus::Success, 0, 0);
        let ok_again = record("stable goal", MemoryStatus::Success, 0, 0);
        let bad = record("fragile goal", MemoryStatus::Failed, 0, 0);
        let bad_again = record("fragile goal", MemoryStatus::Failed, 0, 0);
        let records = vec![ok.clone(), ok_again, bad.clone(), bad_again];

        let good = confidence_score(&ok, 1.0, &records, &HashMap::new(), now);
        let poor = confidence_score(&bad, 1.0, &records, &HashMap::new(), now);
        assert!(
            good.score > poor.score,
            "clean history must outscore failing history: {} vs {}",
            good.score,
            poor.score
        );
    }
}
