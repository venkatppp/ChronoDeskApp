//! Retention & compression rules (RC-6 M4) — pure decisions over memory
//! records: when a temporary memory is due for expiry, which archived
//! memories duplicate a live one (cleanup candidates), and how oversized
//! reasoning histories are summarized for compression.

use chrono::{DateTime, Utc};

use crate::copilot::memory::models::{goal_fingerprint, ExecutionMemoryRecord, RetentionPolicy};

/// Temporary memories older than this are due for expiry *as soon as
/// their `retention_until` passes*; this is the day-boundary guard used
/// by [`is_due_expiry`].
/// Reasoning events above this count make a record compressible.
pub const COMPRESS_REASONING_THRESHOLD: usize = 80;
/// Step descriptions above this count make a record compressible.
pub const COMPRESS_STEPS_THRESHOLD: usize = 150;
/// How many leading/trailing entries a summary preserves.
pub const SUMMARY_HEAD_TAIL: usize = 5;

/// Whether a temporary record has passed its retention deadline and the
/// cleanup worker should mark it `Expired`.
pub fn is_due_expiry(record: &ExecutionMemoryRecord, now: DateTime<Utc>) -> bool {
    match (record.retention, record.retention_until) {
        (RetentionPolicy::Temporary, Some(until)) => until <= now,
        _ => false,
    }
}

/// Candidate ids for the "duplicate archives" cleanup: archived memories
/// that exactly duplicate another non-expired memory which is preferred
/// (a live record beats an archived one; between two archived copies the
/// newer `archived_at` wins). Keeps the newest copy, removes the rest.
pub fn duplicate_archives(records: &[ExecutionMemoryRecord]) -> Vec<uuid::Uuid> {
    let archived: Vec<&ExecutionMemoryRecord> = records
        .iter()
        .filter(|r| r.retention == RetentionPolicy::Archived)
        .collect();

    let preferred =
        |candidate: &ExecutionMemoryRecord, reference: &ExecutionMemoryRecord| -> bool {
            // A non-archived live record always beats an archived duplicate.
            if candidate.retention != RetentionPolicy::Archived {
                return true;
            }
            // Between two archived copies, the one archived later wins.
            match (candidate.archived_at, reference.archived_at) {
                (Some(c), Some(r)) => c > r,
                (Some(_), None) => true,
                _ => false,
            }
        };

    archived
        .iter()
        .filter_map(|archived_record| {
            let duplicate_kept = records.iter().any(|other| {
                other.id != archived_record.id
                    && other.retention != RetentionPolicy::Expired
                    && identical_content(archived_record, other)
                    && preferred(other, archived_record)
            });
            duplicate_kept.then_some(archived_record.id)
        })
        .collect()
}

/// Whether two records are exact duplicates for cleanup purposes:
/// same goal fingerprint, status, steps, and tools.
pub fn identical_content(a: &ExecutionMemoryRecord, b: &ExecutionMemoryRecord) -> bool {
    a.status == b.status
        && goal_fingerprint(&a.goal) == goal_fingerprint(&b.goal)
        && a.steps == b.steps
        && a.tools_used == b.tools_used
}

/// Whether a record is compressible: not already compressed and either
/// its reasoning or step history exceeds the thresholds.
pub fn is_compressible(record: &ExecutionMemoryRecord) -> bool {
    record.compressed_at.is_none()
        && (record.reasoning.len() >= COMPRESS_REASONING_THRESHOLD
            || record.steps.len() >= COMPRESS_STEPS_THRESHOLD)
}

/// Builds the human-readable summary for a compressed history: the
/// total event count plus the first and last entries, so the shape of
/// the reasoning survives compression.
pub fn build_summary(kind: &str, entries: &[String]) -> String {
    let total = entries.len();
    let head = entries
        .iter()
        .take(SUMMARY_HEAD_TAIL)
        .cloned()
        .collect::<Vec<_>>();
    let tail = entries
        .iter()
        .rev()
        .take(SUMMARY_HEAD_TAIL)
        .cloned()
        .collect::<Vec<_>>();
    let head_text = head.join(" | ");
    let tail_text = tail.iter().rev().cloned().collect::<Vec<_>>().join(" | ");
    if total <= SUMMARY_HEAD_TAIL * 2 {
        format!("{total} {kind}: {head_text}")
    } else {
        format!("{total} {kind} (first {head_text} … last {tail_text})")
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::copilot::memory::models::{MemoryKind, MemoryOutcome, MemoryStatus};
    use chrono::Duration;
    use uuid::Uuid;

    pub fn record(goal: &str) -> ExecutionMemoryRecord {
        ExecutionMemoryRecord {
            id: Uuid::new_v4(),
            kind: MemoryKind::Execution,
            source_id: Uuid::new_v4(),
            workspace_id: None,
            goal: goal.to_string(),
            status: MemoryStatus::Success,
            plan: None,
            steps: vec!["step one".into(), "step two".into()],
            reasoning: vec![],
            tools_used: vec!["tool_a".into(), "tool_b".into()],
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
    fn due_expiry_only_for_temporary_past_deadline() {
        let now = Utc::now();
        let mut temp = record("g");
        temp.retention = RetentionPolicy::Temporary;
        temp.retention_until = Some(now + Duration::days(1));
        assert!(!is_due_expiry(&temp, now));

        temp.retention_until = Some(now - Duration::minutes(1));
        assert!(is_due_expiry(&temp, now), "deadline passed");

        temp.retention = RetentionPolicy::Permanent;
        assert!(!is_due_expiry(&temp, now), "permanent never expires");

        temp.retention_until = None;
        assert!(!is_due_expiry(&temp, now), "no deadline");
    }

    #[test]
    fn duplicate_archives_keeps_the_preferred_copy() {
        let live = record("resume my focus session");
        let mut archived = record("  RESUME My Focus Session ");
        archived.retention = RetentionPolicy::Archived;
        archived.archived_at = Some(Utc::now());

        // Live copy exists: the archived one is removable.
        let removals = duplicate_archives(&[live.clone(), archived.clone()]);
        assert_eq!(removals, vec![archived.id]);

        // No live copy: both archived → only the older is removed.
        let mut older = archived.clone();
        older.id = Uuid::new_v4();
        older.archived_at = Some(Utc::now() - Duration::days(2));
        let removals = duplicate_archives(&[archived.clone(), older.clone()]);
        assert_eq!(removals, vec![older.id]);

        // Different content: nothing removed.
        let mut different = archived.clone();
        different.steps = vec!["something else".into()];
        assert!(duplicate_archives(&[live.clone(), different]).is_empty());
    }

    #[test]
    fn compression_rules_and_summary() {
        let mut record = record("g");
        record.reasoning = vec!["r".into(); COMPRESS_REASONING_THRESHOLD];
        assert!(is_compressible(&record));

        record.reasoning = vec!["r".into(); 10];
        record.steps = vec!["s".into(); COMPRESS_STEPS_THRESHOLD];
        assert!(is_compressible(&record), "large steps compress too");

        record.compressed_at = Some(Utc::now());
        assert!(!is_compressible(&record), "already compressed");

        record.compressed_at = None;
        record.steps = vec!["s".into(); 3];
        assert!(!is_compressible(&record), "small history stays");

        let entries = (0..12).map(|i| format!("event {i}")).collect::<Vec<_>>();
        let summary = build_summary("reasoning events", &entries);
        assert!(summary.contains("12 reasoning events"));
        assert!(summary.contains("event 0") && summary.contains("event 11"));
        assert!(summary.contains("…"), "head/tail separator");

        let short = build_summary("steps", &entries[..3]);
        assert!(short.contains("3 steps") && !short.contains("…"));
    }
}
