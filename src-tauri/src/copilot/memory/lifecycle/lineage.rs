//! Memory lineage (RC-6 M4) — builds the workflow evolution graph for a
//! memory: its version ancestry (via `parent_id` chains), its direct
//! descendants, and its merge history. Pure logic over records + edges;
//! the SQL lives in the lifecycle repository.

use uuid::Uuid;

use crate::copilot::memory::models::{
    ExecutionMemoryRecord, LineageNode, LineageRelation, MemoryLineage,
};

/// One persisted lineage edge (decoded from `memory_lineage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageEdge {
    pub memory_id: Uuid,
    pub parent_id: Uuid,
    pub relation: LineageRelation,
}

/// Builds the lineage of `memory_id` over the given records and edges.
/// Returns `None` when the memory does not exist.
pub fn build_lineage(
    records: &[ExecutionMemoryRecord],
    edges: &[LineageEdge],
    memory_id: Uuid,
) -> Option<MemoryLineage> {
    let by_id: std::collections::HashMap<Uuid, &ExecutionMemoryRecord> =
        records.iter().map(|r| (r.id, r)).collect();
    let current = by_id.get(&memory_id)?;

    // Ancestry: walk the parent chain (oldest first).
    let mut ancestors = Vec::new();
    let mut cursor = current.parent_id;
    while let Some(parent_id) = cursor {
        if let Some(parent) = by_id.get(&parent_id) {
            ancestors.push(node(parent, Some(LineageRelation::Parent)));
            cursor = parent.parent_id;
        } else {
            break;
        }
    }
    ancestors.reverse();
    let root_id = ancestors.first().map(|n| n.id);

    // Direct descendants: runs derived from this memory + merged children.
    let mut children: Vec<LineageNode> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for record in records {
        if record.parent_id == Some(memory_id) && seen.insert(record.id) {
            children.push(node(record, Some(LineageRelation::Parent)));
        }
    }
    for edge in edges {
        if edge.parent_id == memory_id
            && edge.relation == LineageRelation::Parent
            && seen.insert(edge.memory_id)
        {
            if let Some(record) = by_id.get(&edge.memory_id) {
                children.push(node(record, Some(LineageRelation::Parent)));
            }
        }
    }
    children.sort_by_key(|node| std::cmp::Reverse(node.created_at));

    // Merges: memories merged into this one, and this one merged into a
    // keeper.
    let mut merged_into = Vec::new();
    let mut merged_into_id = None;
    for edge in edges {
        if edge.parent_id == memory_id && edge.relation == LineageRelation::Merged {
            if let Some(record) = by_id.get(&edge.memory_id) {
                merged_into.push(node(record, Some(LineageRelation::Merged)));
            }
        }
        if edge.memory_id == memory_id && edge.relation == LineageRelation::Merged {
            merged_into_id = Some(edge.parent_id);
        }
    }
    merged_into.sort_by_key(|node| std::cmp::Reverse(node.created_at));

    Some(MemoryLineage {
        memory_id,
        root_id,
        version: current.version,
        ancestors,
        children,
        merged_into,
        merged_into_id,
    })
}

fn node(record: &ExecutionMemoryRecord, relation: Option<LineageRelation>) -> LineageNode {
    LineageNode {
        id: record.id,
        goal: record.goal.clone(),
        status: record.status,
        retention: record.retention,
        version: record.version,
        created_at: record.created_at,
        relation,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::copilot::memory::models::{
        MemoryKind, MemoryOutcome, MemoryStatus, RetentionPolicy,
    };
    use chrono::Utc;

    pub fn record(goal: &str) -> ExecutionMemoryRecord {
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
            outcome: MemoryOutcome::default(),
            goal_embedding: None,
            replay_count: 0,
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
    fn lineage_walks_ancestry_and_children() {
        let v1 = record("organize receipts");
        let mut v2 = record("organize receipts");
        v2.parent_id = Some(v1.id);
        v2.version = 2;
        let mut v3 = record("organize receipts");
        v3.parent_id = Some(v2.id);
        v3.version = 3;
        let mut unrelated = record("plan vacation");
        unrelated.id = Uuid::new_v4();

        let records = vec![v1.clone(), v2.clone(), v3.clone(), unrelated];
        let lineage = build_lineage(&records, &[], v3.id).expect("memory exists");
        assert_eq!(lineage.version, 3);
        assert_eq!(lineage.root_id, Some(v1.id));
        let ancestors: Vec<Uuid> = lineage.ancestors.iter().map(|n| n.id).collect();
        assert_eq!(ancestors, vec![v1.id, v2.id], "oldest first");

        let lineage_v1 = build_lineage(&records, &[], v1.id).expect("memory exists");
        let children: Vec<Uuid> = lineage_v1.children.iter().map(|n| n.id).collect();
        assert_eq!(children, vec![v2.id]);
        assert!(lineage_v1.ancestors.is_empty());
        assert_eq!(lineage_v1.root_id, None, "the root has no root");
    }

    #[test]
    fn lineage_tracks_merges() {
        let keeper = record("resume focus");
        let mut duplicate = record("resume focus");
        duplicate.id = Uuid::new_v4();
        let edge = LineageEdge {
            memory_id: duplicate.id,
            parent_id: keeper.id,
            relation: LineageRelation::Merged,
        };
        let lineage = build_lineage(&[keeper.clone(), duplicate.clone()], &[edge], keeper.id)
            .expect("keeper exists");
        assert_eq!(lineage.merged_into.len(), 1);
        assert_eq!(lineage.merged_into[0].id, duplicate.id);

        let merged_lineage =
            build_lineage(&[keeper.clone(), duplicate.clone()], &[edge], duplicate.id)
                .expect("duplicate exists");
        assert_eq!(merged_lineage.merged_into_id, Some(keeper.id));
    }

    #[test]
    fn lineage_unknown_memory_returns_none() {
        let records = vec![record("g")];
        assert!(build_lineage(&records, &[], Uuid::new_v4()).is_none());
    }
}
