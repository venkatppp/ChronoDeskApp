//! Execution Memory retrieval - semantic similarity over remembered goals
//! and plans.
//!
//! Scoring blends embedding cosine similarity with a term-frequency
//! weighted token overlap so the store stays useful even before the
//! vector index has embedded a record:
//! - identical goals score 1.0,
//! - goals sharing repeated keywords score higher than one-off overlaps,
//! - unrelated goals score near 0.

use std::collections::HashMap;

use crate::copilot::memory::models::{ExecutionMemoryRecord, MemoryHit, MemorySearchRequest};

/// Weight of the embedding cosine in the blended similarity score.
const COSINE_WEIGHT: f64 = 0.6;
/// Weight of the weighted token overlap in the blend.
const TOKEN_WEIGHT: f64 = 0.4;

/// Cosine similarity between two *zero-centered* vectors (0 when lengths
/// differ). Centering before the cosine is required for embedding providers
/// that emit all-positive vectors: the uncentered cosine of two
/// independent positive vectors is far from 0, which would make every
/// goal look similar to every other goal. Centered cosine is 1 for
/// identical vectors and ~0 for unrelated ones.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mean_a: f64 = a.iter().map(|x| *x as f64).sum::<f64>() / a.len() as f64;
    let mean_b: f64 = b.iter().map(|x| *x as f64).sum::<f64>() / b.len() as f64;
    let mut dot = 0.0;
    let mut mag_a = 0.0;
    let mut mag_b = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let centered_a = *x as f64 - mean_a;
        let centered_b = *y as f64 - mean_b;
        dot += centered_a * centered_b;
        mag_a += centered_a * centered_a;
        mag_b += centered_b * centered_b;
    }
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    (dot / (mag_a * mag_b).sqrt()).clamp(0.0, 1.0)
}

/// Term frequencies of the words in a text.
fn term_frequencies(text: &str) -> HashMap<&str, u64> {
    let mut frequencies = HashMap::new();
    for word in text.split_whitespace() {
        *frequencies.entry(word).or_insert(0) += 1;
    }
    frequencies
}

/// Term-frequency weighted token overlap (0..1): the share of token
/// mass the two texts share. Unlike a set Jaccard, a word repeated in
/// both texts counts as much as a unique one, so goals with a strong
/// repeated keyword ("resume", "resume", "resume") are not diluted.
fn weighted_token_overlap(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let fa = term_frequencies(a);
    let fb = term_frequencies(b);
    let shared: u64 = fa
        .iter()
        .map(|(word, count)| {
            if let Some(other) = fb.get(word) {
                (*count).min(*other)
            } else {
                0
            }
        })
        .sum();
    let total = fa.values().sum::<u64>().max(fb.values().sum::<u64>());
    if total == 0 {
        0.0
    } else {
        shared as f64 / total as f64
    }
}

/// Blended similarity between a query goal and a remembered goal.
/// Uses the record's stored goal embedding when present; without an
/// embedding the token overlap alone determines the score. An exact
/// match normalizes to 1.0.
pub fn goal_similarity(
    query: &str,
    query_embedding: Option<&[f32]>,
    record: &ExecutionMemoryRecord,
) -> f64 {
    let query_normalized = query.trim().to_lowercase();
    let goal_normalized = record.goal.trim().to_lowercase();
    if !query_normalized.is_empty() && query_normalized == goal_normalized {
        return 1.0;
    }
    let tokens = weighted_token_overlap(&query_normalized, &goal_normalized);
    match (query_embedding, record.goal_embedding.as_deref()) {
        (Some(q), Some(r)) => {
            let cosine = cosine_similarity(q, r);
            (COSINE_WEIGHT * cosine + TOKEN_WEIGHT * tokens).clamp(0.0, 1.0)
        }
        _ => tokens.clamp(0.0, 1.0),
    }
}

/// Ranks the remembered plans for a goal query, newest first within equal
/// scores. The optional query embedding is produced once by the caller
/// (`MemoryEngine`) so a search does not re-embed per record.
pub fn rank_records(
    query: &str,
    query_embedding: Option<&[f32]>,
    records: &[ExecutionMemoryRecord],
) -> Vec<MemoryHit> {
    let mut hits: Vec<MemoryHit> = records
        .iter()
        .map(|record| MemoryHit {
            record: record.clone(),
            similarity: goal_similarity(query, query_embedding, record),
        })
        .collect();
    hits.sort_by(|a, b| {
        b.similarity
            .total_cmp(&a.similarity)
            .then_with(|| b.record.created_at.cmp(&a.record.created_at))
    });
    hits
}

/// Applies a search request's filters to candidate records (pure, so tests
/// can exercise filtering without a database).
pub fn filter_records(
    request: &MemorySearchRequest,
    records: &[ExecutionMemoryRecord],
) -> Vec<ExecutionMemoryRecord> {
    records
        .iter()
        .filter(|record| {
            if let Some(kind) = request.kind {
                if record.kind != kind {
                    return false;
                }
            }
            if let Some(status) = request.status {
                if record.status != status {
                    return false;
                }
            }
            if let Some(workspace_id) = request.workspace_id {
                if record.workspace_id != Some(workspace_id) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryStatus};
    use chrono::Utc;
    use uuid::Uuid;

    fn record(goal: &str, embedding: Option<Vec<f32>>) -> ExecutionMemoryRecord {
        ExecutionMemoryRecord {
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
            outcome: Default::default(),
            goal_embedding: embedding,
            replay_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn cosine_similarity_matches_expected_values() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), 1.0);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(
            cosine_similarity(&[1.0, 0.0], &[1.0]),
            0.0,
            "length mismatch"
        );
        assert_eq!(cosine_similarity(&[], &[]), 0.0, "empty vectors");
    }

    #[test]
    fn weighted_token_overlap_scores_shared_goals() {
        let overlap = weighted_token_overlap("resume my focus session", "resume my focus session");
        assert!((overlap - 1.0).abs() < 1e-6);
        let partial =
            weighted_token_overlap("resume my focus session", "resume another focus session");
        assert!(partial > 0.0 && partial < 1.0);
        assert_eq!(weighted_token_overlap("alpha beta", "gamma delta"), 0.0);
        assert_eq!(weighted_token_overlap("", "anything"), 0.0);
    }

    #[test]
    fn weighted_overlap_reflects_term_frequencies() {
        // The multiset coverage: shared mass over the larger text mass.
        // A word repeated in one text but not the other counts once, so
        // TF-aware scoring differs from a set Jaccard (which would give
        // ("a a b c", "a b c") = 1.0).
        let repeated = weighted_token_overlap("a a b c", "a b c");
        assert!((repeated - 0.75).abs() < 1e-6, "got {repeated}");
        let contained = weighted_token_overlap("resume", "resume my focus session");
        assert!((contained - 0.25).abs() < 1e-6, "got {contained}");
        let shared_word = weighted_token_overlap("a b c d", "a e f g");
        assert!((shared_word - 0.25).abs() < 1e-6, "got {shared_word}");
        let identical = weighted_token_overlap("a b c d", "a b c d");
        assert!((identical - 1.0).abs() < 1e-6);
    }

    #[test]
    fn goal_similarity_ranks_identical_goals_highest() {
        let same = record("resume my focus session", None);
        let different = record("buy groceries and cook dinner", None);
        let query = "resume my focus session";

        let score_same = goal_similarity(query, None, &same);
        let score_diff = goal_similarity(query, None, &different);
        assert!(
            score_same > score_diff,
            "{score_same} should beat {score_diff}"
        );
        assert!((score_same - 1.0).abs() < 1e-6);
    }

    #[test]
    fn exact_match_normalizes_even_with_no_embedding() {
        let same = record("  RESUME My Focus Session ", None);
        let score = goal_similarity("resume my focus session", None, &same);
        assert!(
            (score - 1.0).abs() < 1e-6,
            "case/space-insensitive exact match"
        );
    }

    #[test]
    fn embedding_boosts_identical_vectors() {
        let embedding = vec![1.0, 0.0, 0.0, 0.0];
        let same = record("resume focus session", Some(embedding.clone()));
        let query_embedding = Some(embedding.as_slice());
        let score = goal_similarity("resume focus session", query_embedding, &same);
        assert!(score >= 1.0 - 1e-6);
    }

    #[test]
    fn rank_records_sorts_by_similarity_descending() {
        let close = record("resume my focus session", None);
        let far = record("organize receipts", None);
        let hits = rank_records(
            "resume my focus session",
            None,
            &[far.clone(), close.clone()],
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].record.id, close.id);
        assert!(hits[0].similarity >= hits[1].similarity);
    }

    #[test]
    fn filter_records_honors_kind_status_and_workspace() {
        let ws = Uuid::new_v4();
        let mut execution = record("goal a", None);
        execution.kind = MemoryKind::Execution;
        execution.status = MemoryStatus::Failed;
        execution.workspace_id = Some(ws);
        let mut session = record("goal b", None);
        session.kind = MemoryKind::AutonomousSession;
        session.status = MemoryStatus::Success;
        let records = vec![execution, session];

        let request = MemorySearchRequest {
            query: "g".into(),
            kind: Some(MemoryKind::Execution),
            workspace_id: Some(ws),
            status: None,
            limit: 10,
        };
        let filtered = filter_records(&request, &records);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].goal, "goal a");

        let request = MemorySearchRequest {
            query: "g".into(),
            kind: None,
            workspace_id: None,
            status: Some(MemoryStatus::Success),
            limit: 10,
        };
        let filtered = filter_records(&request, &records);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].goal, "goal b");
    }
}
