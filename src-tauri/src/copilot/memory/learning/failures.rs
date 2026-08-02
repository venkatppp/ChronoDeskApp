//! Failure pattern detection (RC-6 M3) — surfaces the patterns that
//! should worry the planner/runtime before they trust a workflow:
//!
//! - **repeated_failure**: a goal that failed more often than it
//!   succeeded, with at least two failures in recent history,
//! - **unstable_workflow**: a goal with enough samples whose success
//!   rate is below half — it flips between outcomes,
//! - **low_confidence_plan**: a goal whose remembered plans carry low
//!   planner confidence (the planner itself doubted them).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::copilot::memory::learning::core::{fingerprint_success_rate, learned_workflows};
use crate::copilot::memory::models::{goal_fingerprint, ExecutionMemoryRecord, MemoryStatus};

/// Kinds of failure patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePatternType {
    /// Goal failed more often than it succeeded, with ≥2 failures.
    RepeatedFailure,
    /// Goal with ≥3 samples whose success rate is below 0.5.
    UnstableWorkflow,
    /// Goal whose remembered plans carry low planner confidence (< 0.4).
    LowConfidencePlan,
}

impl FailurePatternType {
    fn as_str(&self) -> &'static str {
        match self {
            FailurePatternType::RepeatedFailure => "repeated_failure",
            FailurePatternType::UnstableWorkflow => "unstable_workflow",
            FailurePatternType::LowConfidencePlan => "low_confidence_plan",
        }
    }
}

/// A detected failure pattern over remembered runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_type: FailurePatternType,
    /// Representative goal string.
    pub goal: String,
    pub goal_fingerprint: String,
    /// Human-readable description.
    pub description: String,
    /// Severity 0..1 (1 = most dangerous).
    pub severity: f64,
    /// Number of runs involved in the pattern.
    pub occurrences: usize,
    pub last_seen: DateTime<Utc>,
    /// Average planner confidence of the runs (low-confidence patterns).
    pub avg_plan_confidence: Option<f64>,
}

/// Minimum failures for the repeated-failure pattern.
const MIN_REPEATED_FAILURES: usize = 2;
/// Minimum samples for the unstable-workflow pattern.
const MIN_UNSTABLE_SAMPLES: usize = 3;
/// Planner confidence below which a plan is considered low-confidence.
const LOW_CONFIDENCE_PLAN_THRESHOLD: f64 = 0.4;
/// How recent a failure must be to count toward "repeated".
const RECENT_FAILURE_DAYS: i64 = 90;

/// Detects failure patterns over the whole store, most severe first.
pub fn failure_patterns(
    records: &[ExecutionMemoryRecord],
    now_ms: i64,
    limit: usize,
) -> Vec<FailurePattern> {
    let workflows = learned_workflows(records);
    let mut patterns = Vec::new();

    for workflow in &workflows {
        let members: Vec<&ExecutionMemoryRecord> = records
            .iter()
            .filter(|record| goal_fingerprint(&record.goal) == workflow.goal_fingerprint)
            .collect();

        if let Some(pattern) = repeated_failure(workflow, &members, now_ms) {
            patterns.push(pattern);
        }
        if let Some(pattern) = unstable_workflow(workflow, records, &members) {
            patterns.push(pattern);
        }
        if let Some(pattern) = low_confidence_plan(workflow, &members) {
            patterns.push(pattern);
        }
    }

    patterns.sort_by(|a, b| {
        b.severity
            .total_cmp(&a.severity)
            .then_with(|| b.last_seen.cmp(&a.last_seen))
    });
    patterns.truncate(limit);
    patterns
}

/// Failure patterns relevant to a specific goal (used by the autonomous
/// runtime as an advisory signal before it trusts a remembered plan).
pub fn failure_patterns_for_goal(
    goal: &str,
    records: &[ExecutionMemoryRecord],
    now_ms: i64,
) -> Vec<FailurePattern> {
    let fingerprint = goal_fingerprint(goal);
    failure_patterns(records, now_ms, 10)
        .into_iter()
        .filter(|pattern| pattern.goal_fingerprint == fingerprint)
        .collect()
}

fn repeated_failure(
    workflow: &crate::copilot::memory::models::LearnedWorkflow,
    members: &[&ExecutionMemoryRecord],
    now_ms: i64,
) -> Option<FailurePattern> {
    let failures: Vec<&ExecutionMemoryRecord> = members
        .iter()
        .filter(|record| record.status == MemoryStatus::Failed)
        .cloned()
        .collect();
    if failures.len() < MIN_REPEATED_FAILURES {
        return None;
    }
    if failures.len() as u64 <= workflow.success_count {
        return None;
    }
    let recent_failures: Vec<&&ExecutionMemoryRecord> = failures
        .iter()
        .filter(|record| {
            let age_days =
                (now_ms - record.created_at.timestamp_millis()).max(0) / (24 * 60 * 60 * 1000);
            age_days <= RECENT_FAILURE_DAYS
        })
        .collect();
    if recent_failures.is_empty() {
        return None;
    }
    let last_seen = failures
        .iter()
        .map(|record| record.created_at)
        .max()
        .unwrap_or_else(Utc::now);
    Some(FailurePattern {
        pattern_type: FailurePatternType::RepeatedFailure,
        goal: workflow.goal.clone(),
        goal_fingerprint: workflow.goal_fingerprint.clone(),
        description: format!(
            "Goal failed {} time(s) with only {} success(es) — repeating it likely fails again",
            failures.len(),
            workflow.success_count
        ),
        severity: (0.4 + 0.1 * failures.len() as f64).min(0.95),
        occurrences: failures.len(),
        last_seen,
        avg_plan_confidence: None,
    })
}

fn unstable_workflow(
    workflow: &crate::copilot::memory::models::LearnedWorkflow,
    records: &[ExecutionMemoryRecord],
    members: &[&ExecutionMemoryRecord],
) -> Option<FailurePattern> {
    let samples = members.len();
    if samples < MIN_UNSTABLE_SAMPLES {
        return None;
    }
    let success_rate = fingerprint_success_rate(members[0], records);
    if success_rate >= 0.5 {
        return None;
    }
    let last_seen = members
        .iter()
        .map(|record| record.created_at)
        .max()
        .unwrap_or_else(Utc::now);
    Some(FailurePattern {
        pattern_type: FailurePatternType::UnstableWorkflow,
        goal: workflow.goal.clone(),
        goal_fingerprint: workflow.goal_fingerprint.clone(),
        description: format!(
            "Workflow succeeded in only {:.0}% of {} runs — outcome is unstable",
            success_rate * 100.0,
            samples
        ),
        severity: (0.4 + 0.6 * (0.5 - success_rate) / 0.5).min(0.95),
        occurrences: samples,
        last_seen,
        avg_plan_confidence: None,
    })
}

fn low_confidence_plan(
    workflow: &crate::copilot::memory::models::LearnedWorkflow,
    members: &[&ExecutionMemoryRecord],
) -> Option<FailurePattern> {
    let low_confidence: Vec<f64> = members
        .iter()
        .filter_map(|record| {
            record
                .plan
                .as_ref()
                .map(|plan| plan.confidence)
                .filter(|confidence| *confidence < LOW_CONFIDENCE_PLAN_THRESHOLD)
        })
        .collect();
    if low_confidence.len() < MIN_REPEATED_FAILURES {
        return None;
    }
    let avg_confidence = low_confidence.iter().sum::<f64>() / low_confidence.len() as f64;
    let last_seen = members
        .iter()
        .map(|record| record.created_at)
        .max()
        .unwrap_or_else(Utc::now);
    Some(FailurePattern {
        pattern_type: FailurePatternType::LowConfidencePlan,
        goal: workflow.goal.clone(),
        goal_fingerprint: workflow.goal_fingerprint.clone(),
        description: format!(
            "{} remembered plan(s) carried planner confidence below {:.0}% (avg {:.0}%)",
            low_confidence.len(),
            LOW_CONFIDENCE_PLAN_THRESHOLD * 100.0,
            avg_confidence * 100.0
        ),
        severity: (0.3 + 0.2 * low_confidence.len() as f64).min(0.8),
        occurrences: low_confidence.len(),
        last_seen,
        avg_plan_confidence: Some(avg_confidence),
    })
}

/// Counts of patterns per type, used by the dashboard header.
pub fn failure_pattern_counts(
    records: &[ExecutionMemoryRecord],
    now_ms: i64,
) -> HashMap<&'static str, usize> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    for pattern in failure_patterns(records, now_ms, usize::MAX) {
        *counts.entry(pattern.pattern_type.as_str()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome};
    use chrono::Duration;
    use uuid::Uuid;

    fn record(
        goal: &str,
        status: MemoryStatus,
        days_old: i64,
        plan_confidence: Option<f64>,
    ) -> ExecutionMemoryRecord {
        let created = Utc::now() - Duration::days(days_old);
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.into(),
            status,
            plan: plan_confidence.map(|confidence| {
                crate::copilot::proactive_models::ExecutionPlan {
                    id: Uuid::new_v4(),
                    workspace_id: None,
                    goal: goal.into(),
                    tasks: vec![],
                    estimated_duration_minutes: 0,
                    required_files: vec![],
                    checkpoints: vec![],
                    confidence,
                    reasoning: "".into(),
                    status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
                    created_at: Utc::now(),
                }
            }),
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
    fn repeated_failures_are_detected() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("deploy app", MemoryStatus::Failed, 2, None),
            record("deploy app", MemoryStatus::Failed, 5, None),
            record("deploy app", MemoryStatus::Success, 60, None),
        ];
        let patterns = failure_patterns(&records, now, 10);
        assert!(
            patterns
                .iter()
                .any(|p| p.pattern_type == FailurePatternType::RepeatedFailure),
            "{patterns:?}"
        );
    }

    #[test]
    fn old_failures_do_not_trigger_repeated_pattern() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("deploy app", MemoryStatus::Failed, 100, None),
            record("deploy app", MemoryStatus::Failed, 200, None),
        ];
        let patterns = failure_patterns(&records, now, 10);
        assert!(
            !patterns
                .iter()
                .any(|p| p.pattern_type == FailurePatternType::RepeatedFailure),
            "{patterns:?}"
        );
    }

    #[test]
    fn unstable_workflow_needs_enough_samples() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("g", MemoryStatus::Success, 1, None),
            record("g", MemoryStatus::Failed, 2, None),
            record("g", MemoryStatus::Failed, 3, None),
        ];
        let patterns = failure_patterns(&records, now, 10);
        assert!(
            patterns
                .iter()
                .any(|p| p.pattern_type == FailurePatternType::UnstableWorkflow),
            "{patterns:?}"
        );
    }

    #[test]
    fn low_confidence_plans_are_surfaced() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("g", MemoryStatus::Success, 1, Some(0.3)),
            record("g", MemoryStatus::Success, 2, Some(0.25)),
        ];
        let patterns = failure_patterns(&records, now, 10);
        let low = patterns
            .iter()
            .find(|p| p.pattern_type == FailurePatternType::LowConfidencePlan);
        assert!(low.is_some(), "{patterns:?}");
        let avg = low.unwrap().avg_plan_confidence.unwrap();
        assert!((avg - 0.275).abs() < 1e-6);
    }

    #[test]
    fn goal_scoped_patterns_filter_by_fingerprint() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("deploy app", MemoryStatus::Failed, 2, None),
            record("deploy app", MemoryStatus::Failed, 5, None),
            record("plan vacation", MemoryStatus::Success, 1, None),
        ];
        let scoped = failure_patterns_for_goal("deploy app", &records, now);
        assert!(!scoped.is_empty());
        let unrelated = failure_patterns_for_goal("unrelated goal", &records, now);
        assert!(unrelated.is_empty());
    }

    #[test]
    fn pattern_counts_cover_every_type() {
        let now = Utc::now().timestamp_millis();
        let records = vec![
            record("a", MemoryStatus::Failed, 2, None),
            record("a", MemoryStatus::Failed, 5, None),
            record("b", MemoryStatus::Success, 1, Some(0.3)),
            record("b", MemoryStatus::Success, 2, Some(0.2)),
            record("c", MemoryStatus::Success, 1, None),
            record("c", MemoryStatus::Failed, 2, None),
            record("c", MemoryStatus::Failed, 3, None),
        ];
        let counts = failure_pattern_counts(&records, now);
        assert!(counts.contains_key("repeated_failure"));
        assert!(counts.contains_key("unstable_workflow"));
        assert!(counts.contains_key("low_confidence_plan"));
    }
}
