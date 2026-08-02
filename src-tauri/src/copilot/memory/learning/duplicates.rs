//! Duplicate memory detection (RC-6 M3) — memories are merged when they
//! are *identical*: same goal fingerprint, same outcome status, same step
//! sequence, and same tools. The best record of each group (most completed
//! steps, then most replays, then newest) is kept; the rest are scheduled
//! for removal. Merging is executed by the [`MemoryEngine`] facade, which
//! also removes the duplicates from the vector index.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::copilot::memory::models::{goal_fingerprint, ExecutionMemoryRecord};

/// A set of identical memories with the record chosen to survive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// Fingerprint shared by all members.
    pub goal_fingerprint: String,
    /// All identical records, newest first.
    pub records: Vec<ExecutionMemoryRecord>,
    /// Id of the record that should survive the merge.
    pub keep_id: Uuid,
    /// Human-readable reason the group is considered duplicate.
    pub reason: String,
}

/// Identity of an identical memory: normalized goal + status + step
/// sequence + tool list (order-sensitive, since executions replay the
/// same sequence).
type DuplicateKey = (String, String, Vec<String>, Vec<String>);

fn duplicate_key(record: &ExecutionMemoryRecord) -> DuplicateKey {
    (
        goal_fingerprint(&record.goal),
        record.status.to_string(),
        record.steps.clone(),
        record.tools_used.clone(),
    )
}

/// Groups identical memories together. Groups of one are not reported.
pub fn duplicate_groups(records: &[ExecutionMemoryRecord]) -> Vec<DuplicateGroup> {
    let mut by_key: HashMap<DuplicateKey, Vec<&ExecutionMemoryRecord>> = HashMap::new();
    for record in records {
        by_key
            .entry(duplicate_key(record))
            .or_default()
            .push(record);
    }

    let mut groups = Vec::new();
    for (_, members) in by_key {
        if members.len() < 2 {
            continue;
        }
        let mut members = members.clone();
        members.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        let member_count = members.len();

        let mut keep = members[0];
        for member in &members {
            let member_score = (
                member.outcome.completed,
                member.replay_count,
                member.created_at,
            );
            let keep_score = (keep.outcome.completed, keep.replay_count, keep.created_at);
            if member_score > keep_score {
                keep = member;
            }
        }

        groups.push(DuplicateGroup {
            goal_fingerprint: goal_fingerprint(&keep.goal),
            records: members.into_iter().cloned().collect(),
            keep_id: keep.id,
            reason: format!(
                "{} identical run(s) of '{}' with the same outcome",
                member_count,
                goal_fingerprint(&keep.goal)
            ),
        });
    }

    groups.sort_by(|a, b| {
        b.records
            .len()
            .cmp(&a.records.len())
            .then_with(|| a.goal_fingerprint.cmp(&b.goal_fingerprint))
    });
    groups
}

/// Builds the merge plan: for every duplicate group, which records to
/// remove (every member except the keeper). Returns `(keeper, removals)`.
pub fn merge_plan(groups: &[DuplicateGroup]) -> Vec<(Uuid, Vec<Uuid>)> {
    groups
        .iter()
        .map(|group| {
            let removals: Vec<Uuid> = group
                .records
                .iter()
                .filter(|record| record.id != group.keep_id)
                .map(|record| record.id)
                .collect();
            (group.keep_id, removals)
        })
        .filter(|(_, removals)| !removals.is_empty())
        .collect()
}

/// Outcome of a merge pass, for the dashboard.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct MergeResult {
    /// Duplicate groups merged.
    pub groups_merged: usize,
    /// Records removed as duplicates.
    pub records_merged: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, MemoryStatus};
    use chrono::Utc;
    use uuid::Uuid;

    fn record(
        goal: &str,
        status: MemoryStatus,
        steps: Vec<&str>,
        tools: Vec<&str>,
        completed: usize,
        replays: u64,
    ) -> ExecutionMemoryRecord {
        let now = Utc::now();
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.into(),
            status,
            plan: None,
            steps: steps.into_iter().map(String::from).collect(),
            reasoning: vec![],
            tools_used: tools.into_iter().map(String::from).collect(),
            failed_steps: vec![],
            error: None,
            outcome: MemoryOutcome {
                completed,
                ..MemoryOutcome::default()
            },
            goal_embedding: None,
            replay_count: replays,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn identical_runs_group_and_single_runs_do_not() {
        let a = record(
            "Resume Focus Session",
            MemoryStatus::Success,
            vec!["a", "b"],
            vec!["tool_x"],
            2,
            0,
        );
        let b = record(
            "resume focus session",
            MemoryStatus::Success,
            vec!["a", "b"],
            vec!["tool_x"],
            2,
            0,
        );
        let different_status = record(
            "resume focus session",
            MemoryStatus::Failed,
            vec!["a", "b"],
            vec!["tool_x"],
            1,
            0,
        );
        let different_steps = record(
            "resume focus session",
            MemoryStatus::Success,
            vec!["a", "c"],
            vec!["tool_x"],
            2,
            0,
        );

        let groups = duplicate_groups(&[a, b, different_status, different_steps]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].records.len(), 2);
        assert_eq!(groups[0].goal_fingerprint, "resume focus session");
    }

    #[test]
    fn keeper_is_the_best_record() {
        let weak = record("g", MemoryStatus::Success, vec!["a"], vec!["t"], 1, 0);
        let strong = record("g", MemoryStatus::Success, vec!["a"], vec!["t"], 1, 4);
        let weak_id = weak.id;
        let strong_id = strong.id;
        let groups = duplicate_groups(&[weak, strong]);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].keep_id, strong_id,
            "most-replayed record survives"
        );

        let plan = merge_plan(&groups);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].0, strong_id);
        assert_eq!(plan[0].1, vec![weak_id]);
    }

    #[test]
    fn no_duplicates_yields_empty_plan() {
        let a = record("g1", MemoryStatus::Success, vec!["a"], vec!["t"], 1, 0);
        let b = record("g2", MemoryStatus::Success, vec!["a"], vec!["t"], 1, 0);
        assert!(duplicate_groups(&[a, b]).is_empty());
        assert!(merge_plan(&[]).is_empty());
    }
}
