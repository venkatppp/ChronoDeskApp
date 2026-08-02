//! Memory lifecycle (RC-6 M4) — turns execution memory into a managed
//! long-term store.
//!
//! Two halves live here:
//!
//! 1. **Pure rules** (submodules): retention policy transitions and
//!    cleanup/compression decisions (`retention`), lineage graph
//!    building (`lineage`), and the portable export/import format
//!    (`export`, also the snapshot payload).
//! 2. **The `MemoryEngine` lifecycle facade** (`impl MemoryEngine`
//!    below): orchestrates the rules + the lifecycle SQL
//!    (`memory/lifecycle_repository.rs`) into the operations the
//!    planner / runtime / IPC consult: retention changes, the cleanup
//!    pass, compression, lineage queries, import/export, snapshots,
//!    and storage statistics.
//!
//! Everything here is memory-side only: nothing plans, executes, or
//! schedules runs.

use std::collections::HashSet;

use chrono::Utc;
use uuid::Uuid;

use crate::copilot::memory::engine::MemoryEngine;
use crate::copilot::memory::models::{
    CleanupReport, CompressionResult, ImportResult, MemoryLineage, MemorySnapshot,
    MemoryStorageStats, RestoreResult, RetentionPolicy,
};
use crate::errors::DatabaseError;

pub mod export;
pub mod lineage;
pub mod retention;

pub use export::{parse_export, serialize_export};
pub use lineage::{build_lineage, LineageEdge};
pub use retention::{
    build_summary, duplicate_archives, is_compressible, is_due_expiry,
    COMPRESS_REASONING_THRESHOLD, COMPRESS_STEPS_THRESHOLD, SUMMARY_HEAD_TAIL,
};

/// How many snapshots are kept (oldest pruned beyond this).
const MAX_SNAPSHOTS: usize = 10;
/// Per-cleanup-pass compression budget (records).
const COMPRESS_BUDGET: usize = 32;

impl MemoryEngine {
    // ------------------------------------------------------------------
    // Retention policies
    // ------------------------------------------------------------------

    /// Sets a record's retention policy. `Temporary` requires
    /// `retention_until` (when the deadline passes, the cleanup worker
    /// marks the record `Expired` and deletes it). `Permanent` revives an
    /// archived/expired record.
    pub async fn set_retention(
        &self,
        id: Uuid,
        policy: RetentionPolicy,
        retention_until: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let (until, archived_at, expired_at) = match policy {
            RetentionPolicy::Temporary => {
                let until = retention_until.ok_or_else(|| {
                    DatabaseError::InvalidInput(
                        "temporary retention requires a retention_until deadline".into(),
                    )
                })?;
                (Some(until), None, None)
            }
            RetentionPolicy::Permanent => (None, None, None),
            RetentionPolicy::Archived => (None, Some(now), None),
            RetentionPolicy::Expired => (None, None, Some(now)),
        };
        self.lifecycle
            .update_retention(id, &policy, until, archived_at, expired_at)
            .await
    }

    /// Archives a record (kept, but out of active circulation).
    pub async fn archive(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.set_retention(id, RetentionPolicy::Archived, None)
            .await
    }

    /// Marks a record expired immediately; the next cleanup pass deletes
    /// it.
    pub async fn expire(&self, id: Uuid) -> Result<(), DatabaseError> {
        self.set_retention(id, RetentionPolicy::Expired, None).await
    }

    // ------------------------------------------------------------------
    // Automatic cleanup
    // ------------------------------------------------------------------

    /// One cleanup pass: expires temporary memories past their deadline,
    /// deletes expired memories (with their vectors/ledger), removes
    /// archived duplicates, orphans the vector index of leftover rows,
    /// and compresses oversized reasoning histories.
    pub async fn run_cleanup(&self) -> Result<CleanupReport, DatabaseError> {
        let mut report = CleanupReport {
            ran_at: Utc::now().to_rfc3339(),
            ..CleanupReport::default()
        };

        // 1. Temporary memories past their deadline → Expired.
        let due = self.lifecycle.list_due_temporary(1000).await?;
        for id in &due {
            self.lifecycle.mark_expired(*id).await?;
        }
        report.expired_marked = due.len() as u64;

        // 2. Expired memories → deleted (vector + row).
        let expired = self.lifecycle.list_expired(1000).await?;
        for id in &expired {
            self.vectors.remove(*id).await?;
            self.repository.delete(*id).await?;
        }
        report.removed_expired = expired.len() as u64;

        // 3. Archived memories duplicating a live one → deleted.
        let all = self.repository.list_all().await?;
        let duplicate_archives = duplicate_archives(&all);
        for id in &duplicate_archives {
            self.vectors.remove(*id).await?;
            self.repository.delete(*id).await?;
        }
        report.removed_duplicate_archives = duplicate_archives.len() as u64;

        // 4. Orphaned vector rows (safety net) → removed.
        let orphans = self.lifecycle.orphaned_vector_ids().await?;
        for id in &orphans {
            self.vectors.remove(*id).await?;
        }
        report.removed_orphaned_vectors = orphans.len() as u64;

        // 5. Oversized reasoning histories → compressed (budgeted).
        let compression = self.compress_oversized(COMPRESS_BUDGET).await?;
        report.compressed = compression.compressed;

        if report.expired_marked
            + report.removed_expired
            + report.removed_duplicate_archives
            + report.removed_orphaned_vectors
            + report.compressed
            > 0
        {
            tracing::info!(?report, "memory cleanup pass complete");
        }
        Ok(report)
    }

    // ------------------------------------------------------------------
    // Compression
    // ------------------------------------------------------------------

    /// Compresses oversized records (reasoning/step histories at or above
    /// the thresholds), up to `limit` per call. Originals are preserved
    /// in the compression archive and can be restored.
    pub async fn compress_oversized(
        &self,
        limit: usize,
    ) -> Result<CompressionResult, DatabaseError> {
        let candidates = self
            .lifecycle
            .list_compressible(
                limit,
                COMPRESS_REASONING_THRESHOLD,
                COMPRESS_STEPS_THRESHOLD,
            )
            .await?;
        let mut result = CompressionResult {
            examined: candidates.len() as u64,
            compressed: 0,
            already_compressed: 0,
        };
        for id in candidates {
            result.compressed += u64::from(self.compress_memory(id).await?);
        }
        Ok(result)
    }

    /// Compresses one record: its reasoning history becomes a summary
    /// (head + tail + count) and the originals are archived. Returns
    /// `false` when the record was already compressed or not eligible.
    pub async fn compress_memory(&self, id: Uuid) -> Result<bool, DatabaseError> {
        let Some(record) = self.repository.get(id).await? else {
            return Ok(false);
        };
        if !is_compressible(&record) {
            return Ok(false);
        }
        let entries = if record.reasoning.len() >= record.steps.len() {
            &record.reasoning
        } else {
            &record.steps
        };
        let kind = if record.reasoning.len() >= record.steps.len() {
            "reasoning events"
        } else {
            "steps"
        };
        let summary = build_summary(kind, entries);
        self.lifecycle
            .save_compression_archive(
                id,
                &serde_json::to_string(&record.reasoning)?,
                &serde_json::to_string(&record.steps)?,
            )
            .await?;
        self.lifecycle.set_compressed(id, &summary).await?;
        Ok(true)
    }

    /// Restores a compressed record from its archive (originals return,
    /// summary fields clear). Returns `false` when not compressed.
    pub async fn restore_compressed(&self, id: Uuid) -> Result<bool, DatabaseError> {
        Ok(self.lifecycle.restore_compressed(id).await?.is_some())
    }

    // ------------------------------------------------------------------
    // Lineage
    // ------------------------------------------------------------------

    /// The full lineage of a memory: version ancestry, descendants, and
    /// merge history. `None` when the memory does not exist.
    pub async fn lineage(&self, id: Uuid) -> Result<Option<MemoryLineage>, DatabaseError> {
        let records = self.repository.list_all().await?;
        let edges = self.lifecycle.load_lineage_edges().await?;
        Ok(build_lineage(&records, &edges, id))
    }

    // ------------------------------------------------------------------
    // Import / export
    // ------------------------------------------------------------------

    /// Exports the whole memory store (records + acceptance ledger) as
    /// JSON — the same format snapshots use, so exports and snapshots
    /// are interchangeable.
    pub async fn export_json(&self) -> Result<String, DatabaseError> {
        let records = self.repository.list_all().await?;
        let acceptance = self.repository.acceptance_map().await?;
        serialize_export(&export::build_export(records, acceptance))
    }

    /// Imports an export payload. Idempotent: records whose id already
    /// exists are skipped, new ones are inserted with their exported
    /// lifecycle state, and the acceptance ledger is restored exactly.
    pub async fn import_json(&self, content: &str) -> Result<ImportResult, DatabaseError> {
        let export = parse_export(content)?;
        let existing: HashSet<Uuid> = self
            .repository
            .list_all()
            .await?
            .into_iter()
            .map(|r| r.id)
            .collect();
        let (imported, skipped) = export::import_plan(&export, &existing);
        let imported_count = imported.len();
        let skipped_count = skipped.len();

        for record in imported {
            self.repository.upsert(record).await?;
        }
        let mut acceptance_restored = 0u64;
        for entry in &export.acceptance {
            self.repository
                .restore_acceptance(
                    entry.memory_id,
                    entry.acceptance.accepted,
                    entry.acceptance.rejected,
                )
                .await?;
            acceptance_restored += 1;
        }
        if imported_count > 0 {
            self.vectors.indexer().notify();
        }
        Ok(export::import_result(
            imported_count,
            skipped_count,
            acceptance_restored as usize,
        ))
    }

    // ------------------------------------------------------------------
    // Snapshots
    // ------------------------------------------------------------------

    /// Creates a snapshot: the full export JSON stored under a label.
    /// Oldest snapshots beyond [`MAX_SNAPSHOTS`] are pruned.
    pub async fn create_snapshot(
        &self,
        label: Option<&str>,
    ) -> Result<MemorySnapshot, DatabaseError> {
        let data = self.export_json().await?;
        let id = Uuid::new_v4();
        let label = label.unwrap_or("auto").to_string();
        self.lifecycle.insert_snapshot(id, &label, &data).await?;
        self.lifecycle.prune_snapshots(MAX_SNAPSHOTS).await?;
        let record_count = snapshot_record_count(&data);
        Ok(MemorySnapshot {
            id,
            label,
            created_at: Utc::now(),
            record_count,
        })
    }

    /// Lists stored snapshots, newest first.
    pub async fn list_snapshots(&self) -> Result<Vec<MemorySnapshot>, DatabaseError> {
        let rows = self.lifecycle.list_snapshots().await?;
        Ok(rows
            .into_iter()
            .map(|row| MemorySnapshot {
                id: row.id,
                label: row.label,
                created_at: row.created_at,
                record_count: snapshot_record_count(&row.data),
            })
            .collect())
    }

    /// Restores the store from a snapshot: every current memory is
    /// removed (cascading acceptance, lineage, compression archive, and
    /// vector rows) and the snapshot's records + ledger are re-inserted.
    /// The vector index is then rebuilt so retrieval is immediately
    /// correct.
    pub async fn restore_snapshot(&self, id: Uuid) -> Result<RestoreResult, DatabaseError> {
        let Some(data) = self.lifecycle.snapshot_data(id).await? else {
            return Err(DatabaseError::not_found("memory_snapshot", id));
        };
        let export = parse_export(&data)?;
        self.lifecycle.clear_store().await?;
        for record in &export.records {
            self.repository.upsert(record).await?;
        }
        let mut acceptance_restored = 0u64;
        for entry in &export.acceptance {
            self.repository
                .restore_acceptance(
                    entry.memory_id,
                    entry.acceptance.accepted,
                    entry.acceptance.rejected,
                )
                .await?;
            acceptance_restored += 1;
        }
        self.vectors.indexer().reindex_all().await?;
        let kept = self.lifecycle.list_snapshots().await?.len() as u64;
        Ok(RestoreResult {
            records_restored: export.records.len() as u64,
            acceptance_restored,
            snapshots_kept: kept,
        })
    }

    // ------------------------------------------------------------------
    // Storage statistics
    // ------------------------------------------------------------------

    /// Storage statistics over the whole memory system: database size,
    /// vector index size, cache usage, retention counts, snapshots, and
    /// compression state.
    pub async fn storage_stats(&self) -> Result<MemoryStorageStats, DatabaseError> {
        let (cache_entries, cache_size_bytes) = self.lifecycle.cache_storage().await?;
        let (snapshots, snapshot_size_bytes) = self.lifecycle.snapshot_storage().await?;
        let (compressed_records, compression_archive_count) =
            self.lifecycle.compressed_counts().await?;
        let cache = self.vectors.cache_stats();
        Ok(MemoryStorageStats {
            database_size_bytes: self.lifecycle.database_size().await?,
            vector_index_size_bytes: self.lifecycle.vector_index_size().await?,
            cache_entries,
            cache_size_bytes,
            cache_capacity: cache.capacity,
            cache_occupancy: cache.size,
            archived_memories: self
                .lifecycle
                .count_by_retention(&RetentionPolicy::Archived)
                .await?,
            expired_memories: self
                .lifecycle
                .count_by_retention(&RetentionPolicy::Expired)
                .await?,
            temporary_memories: self
                .lifecycle
                .count_by_retention(&RetentionPolicy::Temporary)
                .await?,
            permanent_memories: self
                .lifecycle
                .count_by_retention(&RetentionPolicy::Permanent)
                .await?,
            snapshots,
            snapshot_size_bytes,
            compressed_records,
            compression_archive_count,
        })
    }
}

/// Counts the records inside a snapshot payload's JSON (best-effort).
fn snapshot_record_count(data: &str) -> u64 {
    match serde_json::from_str::<serde_json::Value>(data) {
        Ok(value) => value
            .get("records")
            .and_then(|records| records.as_array())
            .map(|array| array.len() as u64)
            .unwrap_or(0),
        Err(_) => 0,
    }
}

#[cfg(test)]
#[path = "../lifecycle_engine_tests.rs"]
mod engine_tests;
