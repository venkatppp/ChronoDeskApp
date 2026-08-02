//! Tests for the learning core: ranking, blends, aggregation, and
//! stats. Adapted from the RC-6 M1/M2 suite to the RC-6 M3 signatures
//! (adaptive weights + acceptance ledger + archival scaling).

use std::collections::HashMap;

use crate::copilot::memory::learning::*;
use crate::copilot::memory::models::{
    ExecutionMemoryRecord, MemoryAcceptance, MemoryKind, MemoryOutcome, MemorySearchRequest,
    MemoryStatus, RetentionPolicy,
};
use crate::copilot::memory::retrieval::filter_records;
use chrono::{Duration, Utc};
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

fn no_acceptance() -> HashMap<Uuid, MemoryAcceptance> {
    HashMap::new()
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

    let score = learned_score(
        fresh_success,
        1.0,
        &records,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert!(score > 0.7 && score <= 1.0, "score was {score}");

    let stale_failure = &records[1];
    let stale_score = learned_score(
        stale_failure,
        1.0,
        &records,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert!(stale_score < score, "failures must score below successes");
}

#[test]
fn learned_score_accepts_adaptive_weights() {
    let now_ms = 1_000_000_000_000;
    let records = vec![
        record("resume focus", MemoryStatus::Success, now_ms, true, None),
        record(
            "resume focus",
            MemoryStatus::Success,
            now_ms - 1,
            true,
            None,
        ),
        record(
            "resume focus",
            MemoryStatus::Success,
            now_ms - 2,
            true,
            None,
        ),
    ];
    let mut heavy_replay = default_weights();
    heavy_replay.replay = 0.8;
    heavy_replay.similarity = 0.2;

    let mut replayed = records[0].clone();
    replayed.replay_count = 8;
    let score = learned_score(
        &replayed,
        1.0,
        &records,
        &no_acceptance(),
        &heavy_replay,
        now_ms,
    );
    assert!(
        score > 0.9,
        "high replay weight must lift the score: {score}"
    );

    let low_replay = learned_score(
        &records[0],
        1.0,
        &records,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert!(low_replay < score);
}

#[test]
fn learned_score_respects_archival_weight() {
    let now_ms = 1_000_000_000_000;
    let fresh = record("g", MemoryStatus::Success, now_ms, true, None);
    let mut aged = fresh.clone();
    aged.created_at =
        chrono::DateTime::from_timestamp_millis(now_ms - 200 * 24 * 3600 * 1000).unwrap();
    let records = vec![fresh.clone(), aged.clone()];

    let fresh_score = learned_score(
        &fresh,
        1.0,
        &records,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    let aged_score = learned_score(
        &aged,
        1.0,
        &records,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert!(
        fresh_score > aged_score * 2.0,
        "archived memories must rank far below fresh ones: {fresh_score} vs {aged_score}"
    );
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
    let successes = rank_historical(
        "resume focus",
        None,
        &records,
        false,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert_eq!(successes.len(), 1);
    assert!(matches!(successes[0].record.status, MemoryStatus::Success));

    let with_failures = rank_historical(
        "resume focus",
        None,
        &records,
        true,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
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
    let hits = rank_historical(
        "resume my focus session",
        None,
        &records,
        false,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
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
fn acceptance_ledger_lifts_preferred_records() {
    let now_ms = 1_000_000_000_000;
    let records = vec![
        record("resume focus", MemoryStatus::Success, now_ms, true, None),
        record(
            "resume focus",
            MemoryStatus::Success,
            now_ms - 1000,
            true,
            None,
        ),
    ];
    let mut acceptance = HashMap::new();
    acceptance.insert(
        records[0].id,
        MemoryAcceptance {
            accepted: 6,
            rejected: 0,
        },
    );
    acceptance.insert(
        records[1].id,
        MemoryAcceptance {
            accepted: 0,
            rejected: 5,
        },
    );

    let score_liked = learned_score(
        &records[0],
        1.0,
        &records,
        &acceptance,
        &default_weights(),
        now_ms,
    );
    let score_rejected = learned_score(
        &records[1],
        1.0,
        &records,
        &acceptance,
        &default_weights(),
        now_ms,
    );
    assert!(
        score_liked > score_rejected,
        "accepted memories must outrank rejected ones"
    );
}

#[test]
fn rank_historical_balances_replays_against_freshness() {
    let now = Utc::now();
    let now_ms = now.timestamp_millis();
    let mut old_replayed = record(
        "g",
        MemoryStatus::Success,
        (now - Duration::days(3)).timestamp_millis(),
        true,
        None,
    );
    old_replayed.replay_count = 9;
    let fresh = record("g", MemoryStatus::Success, now_ms, true, None);
    let records = vec![old_replayed.clone(), fresh.clone()];

    let hits = rank_historical(
        "g",
        None,
        &records,
        false,
        &no_acceptance(),
        &default_weights(),
        now_ms,
    );
    assert_eq!(hits.len(), 2);
    // Strong replay evidence (9 reuses) outweighs a 3-day freshness
    // difference: proven workflows rank above barely-fresher ones.
    assert_eq!(hits[0].record.id, old_replayed.id, "replayed workflows win");
}
