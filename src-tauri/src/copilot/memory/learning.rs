//! Learning Engine - turns raw execution memory into ranked knowledge:
//! which workflows to recommend, which strategies to avoid, and aggregate
//! statistics. Pure logic over [`ExecutionMemoryRecord`]s so every rule is
//! unit-testable without a database.

use std::collections::HashMap;

use chrono::Utc;

use crate::copilot::memory::models::{
    goal_fingerprint, AvoidedStrategy, ExecutionMemoryRecord, LearnedWorkflow, MemoryHit,
    MemoryKind, MemoryStats, MemoryStatus,
};
use crate::copilot::memory::retrieval::goal_similarity;

/// Similarity threshold below which a remembered run is not considered
/// relevant for the queried goal.
pub const RELEVANCE_THRESHOLD: f64 = 0.25;

/// Similarity threshold for *recommending* a remembered workflow for reuse
/// (used by the planner; a strong match means "reuse this plan").
pub const RECOMMENDATION_THRESHOLD: f64 = 0.6;

/// Weight of similarity in the recommendation blend.
const SIMILARITY_WEIGHT: f64 = 0.5;
/// Weight of historical success rate in the recommendation blend.
const SUCCESS_WEIGHT: f64 = 0.3;
/// Weight of recency (0 = oldest, 1 = newest) in the recommendation blend.
const RECENCY_WEIGHT: f64 = 0.2;

/// Normalized recency factor for a record (1 = most recent, 0 = oldest).
/// `now` is injected so tests stay deterministic.
fn recency_factor(record: &ExecutionMemoryRecord, now_ms: i64) -> f64 {
    let created_ms = record.created_at.timestamp_millis();
    let age_ms = (now_ms - created_ms).max(0);
    const HALF_LIFE_DAYS: i64 = 30;
    let half_life_ms = HALF_LIFE_DAYS * 24 * 60 * 60 * 1000;
    (-(age_ms as f64) / half_life_ms as f64).exp()
}

/// Success rate of the goal fingerprint a record belongs to (0..1).
/// Includes every remembered run of the same goal, so a workflow that
/// failed last time but mostly succeeded still scores well.
fn fingerprint_success_rate(
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

/// Ranks remembered runs by relevance for a goal, with the learned
/// history blend (similarity, success rate, recency). Success-only by
/// default; pass `include_failures` to surface failure history too.
pub fn rank_historical(
    goal: &str,
    query_embedding: Option<&[f32]>,
    records: &[ExecutionMemoryRecord],
    include_failures: bool,
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
        let score_a = learned_score(&a.record, a.similarity, records, now_ms);
        let score_b = learned_score(&b.record, b.similarity, records, now_ms);
        score_b
            .total_cmp(&score_a)
            .then_with(|| b.record.created_at.cmp(&a.record.created_at))
    });
    hits
}

/// The blended "learned score" for a candidate: similarity, the success
/// history of its goal fingerprint, and recency.
pub fn learned_score(
    record: &ExecutionMemoryRecord,
    similarity: f64,
    records: &[ExecutionMemoryRecord],
    now_ms: i64,
) -> f64 {
    let success_rate = fingerprint_success_rate(record, records);
    (SIMILARITY_WEIGHT * similarity
        + SUCCESS_WEIGHT * success_rate
        + RECENCY_WEIGHT * recency_factor(record, now_ms))
    .clamp(0.0, 1.0)
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

        // Members arrive newest-first (repository `list_all` is ascending,
        // but `learned_workflows` is order-agnostic: pick the most recent
        // goal string by timestamp).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryOutcome, MemorySearchRequest};
    use crate::copilot::memory::retrieval::filter_records;
    use chrono::Utc;
    use uuid::Uuid;

    fn record(
        goal: &str,
        status: MemoryStatus,
        created_ms: i64,
        plan: bool,
        error: Option<&str>,
    ) -> ExecutionMemoryRecord {
        let now = chrono::DateTime::from_timestamp_millis(created_ms).unwrap();
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.to_string(),
            status,
            plan: plan.then(|| crate::copilot::proactive_models::ExecutionPlan {
                id: Uuid::new_v4(),
                workspace_id: None,
                goal: goal.to_string(),
                tasks: vec![],
                estimated_duration_minutes: 0,
                required_files: vec![],
                checkpoints: vec![],
                confidence: 0.8,
                reasoning: "remembered".into(),
                status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
                created_at: Utc::now(),
            }),
            steps: vec![],
            reasoning: vec![],
            tools_used: vec!["list_workspaces".into()],
            failed_steps: error
                .map(|_| vec!["get_recent_events".into()])
                .unwrap_or_default(),
            error: error.map(String::from),
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn learned_score_blends_similarity_success_and_recency() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record(
                "resume my focus session",
                MemoryStatus::Success,
                now_ms,
                true,
                None,
            ),
            record(
                "resume my focus session",
                MemoryStatus::Failed,
                now_ms - 1_000,
                false,
                Some("denied"),
            ),
        ];
        let fresh_success = &records[0];

        let score = learned_score(fresh_success, 1.0, &records, now_ms);
        // similarity 1.0 * 0.5 + success rate 0.5 * 0.3 + recency ~1.0 * 0.2
        assert!(score > 0.8 && score <= 1.0);

        let stale_failure = &records[1];
        let stale_score = learned_score(stale_failure, 1.0, &records, now_ms);
        assert!(stale_score < score, "failures must score below successes");
    }

    #[test]
    fn rank_historical_excludes_failures_unless_requested() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record("resume focus", MemoryStatus::Success, now_ms, true, None),
            record(
                "resume focus",
                MemoryStatus::Failed,
                now_ms,
                false,
                Some("denied"),
            ),
        ];
        let successes = rank_historical("resume focus", None, &records, false, now_ms);
        assert_eq!(successes.len(), 1);
        assert!(matches!(successes[0].record.status, MemoryStatus::Success));

        let with_failures = rank_historical("resume focus", None, &records, true, now_ms);
        assert_eq!(with_failures.len(), 2);
    }

    #[test]
    fn rank_historical_respects_relevance_threshold() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record(
                "resume my focus session",
                MemoryStatus::Success,
                now_ms,
                true,
                None,
            ),
            record(
                "organize tax documents",
                MemoryStatus::Success,
                now_ms,
                true,
                None,
            ),
        ];
        let hits = rank_historical("resume my focus session", None, &records, false, now_ms);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].record.goal, "resume my focus session");
    }

    #[test]
    fn avoid_strategies_surfaces_failed_tools() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record(
                "resume focus",
                MemoryStatus::Failed,
                now_ms,
                false,
                Some("permission denied"),
            ),
            record("resume focus", MemoryStatus::Success, now_ms, true, None),
        ];
        let avoided = avoid_strategies("resume focus", None, &records, 5);
        assert_eq!(avoided.len(), 1);
        assert!(avoided[0].failure.contains("permission denied"));
        assert!(avoided[0].similarity > 0.9);
    }

    #[test]
    fn learned_workflows_aggregate_by_fingerprint() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record(
                "Resume My Focus Session",
                MemoryStatus::Success,
                now_ms,
                true,
                None,
            ),
            record(
                "resume my focus session",
                MemoryStatus::Success,
                now_ms - 1000,
                false,
                None,
            ),
            record(
                "resume my focus session",
                MemoryStatus::Failed,
                now_ms - 2000,
                false,
                Some("x"),
            ),
            record("plan a vacation", MemoryStatus::Success, now_ms, true, None),
        ];
        let workflows = learned_workflows(&records);
        assert_eq!(workflows.len(), 2);

        let focus = workflows
            .iter()
            .find(|w| w.goal_fingerprint == "resume my focus session")
            .expect("focus workflow exists");
        assert_eq!(focus.success_count, 2);
        assert_eq!(focus.failure_count, 1);
        assert!(
            focus.best_plan.is_some(),
            "best plan from the success is kept"
        );
    }

    #[test]
    fn compute_stats_tallies_every_axis() {
        let now_ms = 1_000_000_000_000;
        let mut failed_session = record("g", MemoryStatus::Failed, now_ms, false, Some("e"));
        failed_session.kind = MemoryKind::AutonomousSession;
        failed_session.replay_count = 3;
        let mut report = record("g2", MemoryStatus::Success, now_ms, true, None);
        report.kind = MemoryKind::PlannerReport;
        let records = vec![
            record("g", MemoryStatus::Success, now_ms, true, None),
            failed_session,
            report,
        ];

        let stats = compute_stats(&records);
        assert_eq!(stats.total_records, 3);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.executions, 1);
        assert_eq!(stats.planner_reports, 1);
        assert_eq!(stats.autonomous_sessions, 1);
        assert_eq!(stats.total_replays, 3);
        // Fingerprints: "g" (execution + session) and "g2" (report) → 2.
        assert_eq!(stats.learned_workflows, 2);
    }

    #[test]
    fn filter_records_combines_with_ranking_pipeline() {
        let now_ms = 1_000_000_000_000;
        let records = vec![
            record("resume focus", MemoryStatus::Success, now_ms, true, None),
            record(
                "resume focus",
                MemoryStatus::Failed,
                now_ms,
                false,
                Some("denied"),
            ),
        ];
        let request = MemorySearchRequest {
            query: "resume focus".into(),
            kind: None,
            workspace_id: None,
            status: Some(MemoryStatus::Success),
            limit: 5,
        };
        let filtered = filter_records(&request, &records);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn recency_favors_newer_records() {
        let now = Utc::now();
        let old = now - chrono::Duration::days(60);
        let recent = record(
            "g",
            MemoryStatus::Success,
            now.timestamp_millis(),
            false,
            None,
        );
        let stale = record(
            "g",
            MemoryStatus::Success,
            old.timestamp_millis(),
            false,
            None,
        );
        let now_ms = now.timestamp_millis();
        assert!(recency_factor(&recent, now_ms) > recency_factor(&stale, now_ms));
    }
}
