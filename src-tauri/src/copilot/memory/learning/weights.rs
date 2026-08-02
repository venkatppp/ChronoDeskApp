//! Adaptive recommendation weights (RC-6 M3) — the ranking blend no
//! longer uses fixed constants. `learn_weights` reads the store's
//! history (success rate, replay frequency, user acceptance rate) and
//! nudges the base weights within bounded deltas, then renormalizes so
//! the blend always sums to 1. All shifts are explainable and unit
//! tested; the shifts are intentionally modest so the blend stays stable
//! early on, when there is little history to learn from.

use std::collections::HashMap;

use uuid::Uuid;

use crate::copilot::memory::models::{ExecutionMemoryRecord, MemoryAcceptance, MemoryStatus};

/// Weights of the recommendation blend factors. Sums to 1 when normalized
/// (all constructors except [`default_weights`] produce normalized sets).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearningWeights {
    /// Goal similarity.
    pub similarity: f64,
    /// Goal fingerprint success history.
    pub success: f64,
    /// Freshness of the memory.
    pub recency: f64,
    /// Replay history.
    pub replay: f64,
    /// User acceptance (from the acceptance ledger).
    pub acceptance: f64,
    /// Execution duration (faster workflows preferred).
    pub duration: f64,
    /// Failure history of the fingerprint.
    pub failure: f64,
}

impl LearningWeights {
    /// Sum of all weights (1.0 for normalized sets).
    pub fn sum(&self) -> f64 {
        self.similarity
            + self.success
            + self.recency
            + self.replay
            + self.acceptance
            + self.duration
            + self.failure
    }
}

/// The neutral base weights, used when history is too thin to learn from.
pub fn default_weights() -> LearningWeights {
    LearningWeights {
        similarity: 0.35,
        success: 0.20,
        recency: 0.15,
        replay: 0.10,
        acceptance: 0.10,
        duration: 0.05,
        failure: 0.05,
    }
}

/// Bounds for each learned adjustment (a weight can shift by at most
/// this much per observed signal, keeping the blend stable).
const MAX_ADJUSTMENT: f64 = 0.08;

/// Learns the blend weights from store history:
///
/// - a consistently high **acceptance rate** raises the acceptance weight
///   (user feedback is trusted),
/// - frequent **replays** raise the replay weight and lower the recency
///   weight (replayed workflows prove themselves through use),
/// - a shaky **success rate** raises the failure weight and lowers the
///   success weight (avoiding past mistakes matters more),
/// - a strong success rate raises the success weight.
pub fn learn_weights(
    records: &[ExecutionMemoryRecord],
    acceptance: &HashMap<Uuid, MemoryAcceptance>,
    _now_ms: i64,
) -> LearningWeights {
    let mut weights = default_weights();

    let total = records.len() as f64;
    if total < 3.0 {
        // Too little history to learn from: stay neutral.
        return weights;
    }

    let successes = records
        .iter()
        .filter(|r| r.status == MemoryStatus::Success)
        .count() as f64;
    let success_rate = successes / total;

    let total_replays: u64 = records.iter().map(|r| r.replay_count).sum();
    let replay_frequency = (total_replays as f64 / total / 10.0).min(1.0);

    let with_feedback: Vec<f64> = records
        .iter()
        .filter_map(|r| acceptance.get(&r.id))
        .map(MemoryAcceptance::rate)
        .collect();
    let acceptance_rate = if with_feedback.is_empty() {
        0.5
    } else {
        with_feedback.iter().sum::<f64>() / with_feedback.len() as f64
    };

    weights.acceptance += MAX_ADJUSTMENT * acceptance_rate;
    weights.replay += MAX_ADJUSTMENT * replay_frequency;
    weights.recency -= MAX_ADJUSTMENT * replay_frequency;
    weights.failure += MAX_ADJUSTMENT * (1.0 - success_rate);
    weights.success += MAX_ADJUSTMENT * (2.0 * success_rate - 1.0);

    normalize(&mut weights);
    weights
}

/// Renormalizes the weights to sum to exactly 1, clamping negatives first.
fn normalize(weights: &mut LearningWeights) {
    weights.similarity = weights.similarity.max(0.0);
    weights.success = weights.success.max(0.0);
    weights.recency = weights.recency.max(0.0);
    weights.replay = weights.replay.max(0.0);
    weights.acceptance = weights.acceptance.max(0.0);
    weights.duration = weights.duration.max(0.0);
    weights.failure = weights.failure.max(0.0);

    let sum = weights.sum();
    if sum == 0.0 {
        *weights = default_weights();
        return;
    }
    weights.similarity /= sum;
    weights.success /= sum;
    weights.recency /= sum;
    weights.replay /= sum;
    weights.acceptance /= sum;
    weights.duration /= sum;
    weights.failure /= sum;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, RetentionPolicy};
    use chrono::Utc;
    use uuid::Uuid;

    fn record(goal: &str, status: MemoryStatus, replays: u64) -> ExecutionMemoryRecord {
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
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

    #[test]
    fn default_weights_are_normalized() {
        let weights = default_weights();
        assert!((weights.sum() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn thin_history_stays_neutral() {
        let records = vec![
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Failed, 0),
        ];
        let weights = learn_weights(&records, &HashMap::new(), 0);
        assert_eq!(weights, default_weights());
    }

    #[test]
    fn high_acceptance_raises_acceptance_weight() {
        let records = vec![
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Failed, 0),
            record("g", MemoryStatus::Success, 0),
            record("g2", MemoryStatus::Success, 0),
        ];
        let mut acceptance = HashMap::new();
        for (i, r) in records.iter().enumerate() {
            acceptance.insert(
                r.id,
                MemoryAcceptance {
                    accepted: if i == 0 { 0 } else { 5 },
                    rejected: 0,
                },
            );
        }
        let weights = learn_weights(&records, &acceptance, 0);
        assert!(
            weights.acceptance > default_weights().acceptance,
            "acceptance weight must grow with user acceptance"
        );
        assert!((weights.sum() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn replay_history_raises_replay_and_lowers_recency() {
        let mut records = vec![
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Success, 0),
            record("g", MemoryStatus::Failed, 0),
        ];
        records[0].replay_count = 25;
        records[1].replay_count = 20;
        let weights = learn_weights(&records, &HashMap::new(), 0);
        let base = default_weights();
        assert!(weights.replay > base.replay);
        assert!(weights.recency < base.recency);
        assert!((weights.sum() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn failure_history_raises_failure_weight() {
        let records = vec![
            record("g", MemoryStatus::Failed, 0),
            record("g", MemoryStatus::Failed, 0),
            record("g", MemoryStatus::Failed, 0),
            record("g", MemoryStatus::Failed, 0),
        ];
        let weights = learn_weights(&records, &HashMap::new(), 0);
        assert!(weights.failure > default_weights().failure);
    }
}
