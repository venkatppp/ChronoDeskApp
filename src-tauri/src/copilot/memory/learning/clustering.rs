//! Workflow clustering (RC-6 M3) — groups remembered goals into reusable
//! **workflow families**: goals whose runs share tools (or embed close)
//! are treated as one family so the dashboard (and later the planner)
//! sees the shape of recurring work instead of a flat list.
//!
//! Greedy single-link clustering over fingerprint-level tool overlap:
//! two workflows join a family when ≥50% of their tool sets are shared,
//! or when their goal embeddings are very close (cosine ≥ 0.65) and
//! neither has tools recorded. Deterministic and pure.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::copilot::memory::learning::core::learned_workflows;
use crate::copilot::memory::models::{goal_fingerprint, ExecutionMemoryRecord, LearnedWorkflow};
use crate::copilot::memory::retrieval::cosine_similarity;

/// Minimum tool Jaccard overlap for two workflows to join a family.
const TOOL_OVERLAP_THRESHOLD: f64 = 0.5;
/// Minimum embedding cosine for two workflows to join a family (used
/// when neither has tools recorded).
const EMBEDDING_COSINE_THRESHOLD: f64 = 0.65;

/// A family of related workflows learned from repeated executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFamily {
    /// Stable index of the family (0-based, largest first).
    pub family_id: usize,
    /// Representative name (the most common member goal).
    pub name: String,
    /// Number of distinct workflow fingerprints in the family.
    pub member_count: usize,
    /// Member goals (most recent goal of each fingerprint).
    pub goals: Vec<String>,
    /// Tools shared by the members (most common first).
    pub shared_tools: Vec<String>,
    /// Total successful runs across the family.
    pub total_successes: u64,
    /// Total failed runs across the family.
    pub total_failures: u64,
    /// Mean completion time of remembered runs (0 when unknown).
    pub avg_duration_seconds: u64,
    /// Mean plan confidence of remembered runs (0 when unknown).
    pub avg_confidence: f64,
}

/// Clusters learned workflows into families.
pub fn workflow_families(records: &[ExecutionMemoryRecord]) -> Vec<WorkflowFamily> {
    let workflows = learned_workflows(records);
    if workflows.is_empty() {
        return Vec::new();
    }

    // Fingerprint → union of tools across its runs, and representative
    // embeddings for cosine fallback.
    let mut tools_by_fingerprint: Vec<(String, HashSet<String>)> = Vec::new();
    let mut embedding_by_fingerprint: Vec<(String, Option<Vec<f32>>)> = Vec::new();
    for workflow in &workflows {
        let fingerprint = &workflow.goal_fingerprint;
        let mut tools = HashSet::new();
        let mut embedding = None;
        let mut newest_embedding_at = None;
        for record in records {
            if goal_fingerprint(&record.goal) != *fingerprint {
                continue;
            }
            for tool in &record.tools_used {
                tools.insert(tool.clone());
            }
            if let Some(vec) = &record.goal_embedding {
                let at = record.updated_at;
                if newest_embedding_at.map_or(true, |t| at > t) {
                    newest_embedding_at = Some(at);
                    embedding = Some(vec.clone());
                }
            }
        }
        tools_by_fingerprint.push((fingerprint.clone(), tools));
        embedding_by_fingerprint.push((fingerprint.clone(), embedding));
    }

    // Greedy single-link: each workflow starts alone; merge when the
    // similarity to any current family member passes the threshold.
    let mut families: Vec<Vec<usize>> = Vec::new();
    for index in 0..workflows.len() {
        let mut joined = None;
        for (family_index, family) in families.iter().enumerate() {
            if family.iter().any(|member| {
                workflow_similar(
                    *member,
                    index,
                    &tools_by_fingerprint,
                    &embedding_by_fingerprint,
                )
            }) {
                joined = Some(family_index);
                break;
            }
        }
        match joined {
            Some(family_index) => families[family_index].push(index),
            None => families.push(vec![index]),
        }
    }

    let mut result: Vec<WorkflowFamily> = families
        .into_iter()
        .enumerate()
        .map(|(family_id, members)| build_family(family_id, &members, &workflows, records))
        .collect();

    result.sort_by(|a, b| {
        b.member_count
            .cmp(&a.member_count)
            .then_with(|| b.total_successes.cmp(&a.total_successes))
            .then_with(|| a.name.cmp(&b.name))
    });
    for (index, family) in result.iter_mut().enumerate() {
        family.family_id = index;
    }
    result
}

/// Whether two workflows are similar enough to share a family.
fn workflow_similar(
    a: usize,
    b: usize,
    tools_by_fingerprint: &[(String, HashSet<String>)],
    embedding_by_fingerprint: &[(String, Option<Vec<f32>>)],
) -> bool {
    if a == b {
        return true;
    }
    let (_, tools_a) = &tools_by_fingerprint[a];
    let (_, tools_b) = &tools_by_fingerprint[b];
    if !tools_a.is_empty() || !tools_b.is_empty() {
        if tools_a.is_empty() || tools_b.is_empty() {
            return false;
        }
        let shared = tools_a.intersection(tools_b).count();
        let union = tools_a.len().max(tools_b.len());
        if union > 0 && shared as f64 / union as f64 >= TOOL_OVERLAP_THRESHOLD {
            return true;
        }
        return false;
    }

    let (_, embedding_a) = &embedding_by_fingerprint[a];
    let (_, embedding_b) = &embedding_by_fingerprint[b];
    match (embedding_a, embedding_b) {
        (Some(vec_a), Some(vec_b)) => cosine_similarity(vec_a, vec_b) >= EMBEDDING_COSINE_THRESHOLD,
        _ => false,
    }
}

/// Aggregates a family of workflow fingerprints into its summary.
fn build_family(
    family_id: usize,
    members: &[usize],
    workflows: &[LearnedWorkflow],
    records: &[ExecutionMemoryRecord],
) -> WorkflowFamily {
    let member_workflows: Vec<&LearnedWorkflow> =
        members.iter().map(|index| &workflows[*index]).collect();

    // Most common goal string across members is the representative name.
    let mut goal_counts: Vec<(String, usize)> = Vec::new();
    let mut tools_counts: Vec<(String, usize)> = Vec::new();
    let mut successes = 0u64;
    let mut failures = 0u64;
    let mut duration_sum = 0u64;
    let mut duration_count = 0u64;
    let mut confidence_sum = 0.0;
    let mut confidence_count = 0u64;

    for workflow in &member_workflows {
        if let Some(slot) = goal_counts.iter_mut().find(|(g, _)| *g == workflow.goal) {
            slot.1 += 1;
        } else {
            goal_counts.push((workflow.goal.clone(), 1));
        }
        successes += workflow.success_count;
        failures += workflow.failure_count;
    }

    for record in records {
        if !members
            .iter()
            .any(|index| goal_fingerprint(&record.goal) == workflows[*index].goal_fingerprint)
        {
            continue;
        }
        for tool in &record.tools_used {
            if let Some(slot) = tools_counts.iter_mut().find(|(t, _)| t == tool) {
                slot.1 += 1;
            } else {
                tools_counts.push((tool.clone(), 1));
            }
        }
        if record.outcome.duration_seconds > 0 {
            duration_sum += record.outcome.duration_seconds;
            duration_count += 1;
        }
        if let Some(plan) = &record.plan {
            confidence_sum += plan.confidence;
            confidence_count += 1;
        }
    }

    goal_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    tools_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let goals: Vec<String> = member_workflows.iter().map(|w| w.goal.clone()).collect();
    let shared_tools: Vec<String> = tools_counts
        .iter()
        .filter(|(_, count)| *count >= members.len().min(2))
        .map(|(tool, _)| tool.clone())
        .collect();

    WorkflowFamily {
        family_id,
        name: goal_counts
            .first()
            .map(|(goal, _)| goal.clone())
            .unwrap_or_default(),
        member_count: member_workflows.len(),
        goals,
        shared_tools,
        total_successes: successes,
        total_failures: failures,
        avg_duration_seconds: duration_sum.checked_div(duration_count).unwrap_or(0),
        avg_confidence: if confidence_count > 0 {
            confidence_sum / confidence_count as f64
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, MemoryStatus};
    use chrono::Utc;
    use uuid::Uuid;

    fn record(goal: &str, status: MemoryStatus, tools: Vec<&str>) -> ExecutionMemoryRecord {
        let now = Utc::now();
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
            tools_used: tools.into_iter().map(String::from).collect(),
            failed_steps: vec![],
            error: None,
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn tools_group_workflows_into_families() {
        let records = vec![
            record(
                "resume focus session",
                MemoryStatus::Success,
                vec!["get_recent_events", "list_workspaces", "resume_workspace"],
            ),
            record(
                "resume my last focus session",
                MemoryStatus::Success,
                vec!["get_recent_events", "list_workspaces", "resume_workspace"],
            ),
            record(
                "organize tax receipts",
                MemoryStatus::Success,
                vec!["search_files", "create_workspace"],
            ),
        ];
        let families = workflow_families(&records);
        assert_eq!(families.len(), 2);
        let focus = families
            .iter()
            .find(|f| f.member_count == 2)
            .expect("focus family has two members");
        assert!(focus.shared_tools.contains(&"resume_workspace".to_string()));
        assert_eq!(focus.total_successes, 2);
    }

    #[test]
    fn empty_store_yields_no_families() {
        assert!(workflow_families(&[]).is_empty());
    }
}
