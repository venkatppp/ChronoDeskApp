//! Memory export/import (RC-6 M4) — the portable, versioned format used
//! by exports, imports, snapshots, and restores. Pure serialization;
//! compatibility rules live here:
//!
//! - `schema_version` is stamped on every payload; imports reject
//!   payloads from a *newer* format so a future ChronoDesk export never
//!   silently corrupts an older store.
//! - Imports are idempotent by record id (the engine skips existing ids).
//!
//! The acceptance ledger travels alongside the records so a snapshot or
//! export restores the complete learning state, not just the rows.

use crate::copilot::memory::models::{
    ExecutionMemoryRecord, ImportResult, MemoryAcceptance, MemoryAcceptanceEntry, MemoryExport,
    MEMORY_EXPORT_SCHEMA_VERSION,
};
use crate::errors::DatabaseError;

/// Serializes an export payload (pretty-printed JSON).
pub fn serialize_export(export: &MemoryExport) -> Result<String, DatabaseError> {
    serde_json::to_string_pretty(export).map_err(DatabaseError::from)
}

/// Parses an export payload, rejecting unknown/newer schema versions.
pub fn parse_export(json: &str) -> Result<MemoryExport, DatabaseError> {
    let export: MemoryExport = serde_json::from_str(json).map_err(DatabaseError::from)?;
    if export.schema_version > MEMORY_EXPORT_SCHEMA_VERSION {
        return Err(DatabaseError::IoError(format!(
            "unsupported memory export schema {} (this build supports up to {})",
            export.schema_version, MEMORY_EXPORT_SCHEMA_VERSION
        )));
    }
    Ok(export)
}

/// Builds a versioned export payload from records + acceptance ledger.
pub fn build_export(
    records: Vec<ExecutionMemoryRecord>,
    acceptance: std::collections::HashMap<uuid::Uuid, MemoryAcceptance>,
) -> MemoryExport {
    let mut entries: Vec<MemoryAcceptanceEntry> = acceptance
        .into_iter()
        .map(|(memory_id, acceptance)| MemoryAcceptanceEntry {
            memory_id,
            acceptance,
        })
        .collect();
    entries.sort_by_key(|e| e.memory_id);
    MemoryExport {
        schema_version: MEMORY_EXPORT_SCHEMA_VERSION,
        exported_at: chrono::Utc::now(),
        records,
        acceptance: entries,
    }
}

/// Plans the import of an export payload against the records already in
/// the store: which records are new (import) and which collide by id
/// (skip). `existing` is the set of ids currently in the store.
pub fn import_plan<'a>(
    export: &'a MemoryExport,
    existing: &std::collections::HashSet<uuid::Uuid>,
) -> (
    Vec<&'a ExecutionMemoryRecord>,
    Vec<&'a ExecutionMemoryRecord>,
) {
    export
        .records
        .iter()
        .partition(|record| !existing.contains(&record.id))
}

/// Summarizes an import plan into the IPC result (pure, so the engine
/// just applies it).
pub fn import_result(imported: usize, skipped: usize, acceptance_restored: usize) -> ImportResult {
    ImportResult {
        imported: imported as u64,
        skipped: skipped as u64,
        acceptance_restored: acceptance_restored as u64,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::copilot::memory::models::{
        MemoryKind, MemoryOutcome, MemoryStatus, RetentionPolicy,
    };
    use chrono::Utc;
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
    fn export_round_trips_through_json() {
        let record = record("resume my focus session");
        let mut acceptance = std::collections::HashMap::new();
        acceptance.insert(
            record.id,
            MemoryAcceptance {
                accepted: 3,
                rejected: 1,
            },
        );
        let export = build_export(vec![record.clone()], acceptance);
        assert_eq!(export.schema_version, MEMORY_EXPORT_SCHEMA_VERSION);

        let json = serialize_export(&export).unwrap();
        let parsed = parse_export(&json).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].goal, record.goal);
        assert_eq!(parsed.records[0].id, record.id);
        assert_eq!(parsed.acceptance.len(), 1);
        assert_eq!(parsed.acceptance[0].acceptance.accepted, 3);
    }

    #[test]
    fn newer_schema_versions_are_rejected() {
        let mut export = build_export(vec![], std::collections::HashMap::new());
        export.schema_version = MEMORY_EXPORT_SCHEMA_VERSION + 1;
        let json = serialize_export(&export).unwrap();
        assert!(
            parse_export(&json).is_err(),
            "future schema must be rejected"
        );
    }

    #[test]
    fn import_plan_partitions_by_existing_id() {
        let a = record("a");
        let b = record("b");
        let mut existing = std::collections::HashSet::new();
        existing.insert(a.id);
        let export = build_export(vec![a, b], std::collections::HashMap::new());
        let (imported, skipped) = import_plan(&export, &existing);
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].goal, "b");
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].goal, "a");
    }
}
