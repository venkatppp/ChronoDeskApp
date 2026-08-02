# RC-6 M4 Engineering Report — Memory Management & Lifecycle

**Date:** 2026-08-02
**Branch:** `main`

---

## Summary

RC-6 M4 turns ChronoDesk's execution memory from a store it *remembers*
with into a store it **manages**: every memory carries a **retention
policy** (permanent / temporary / archived / expired), a **background
cleanup worker** enforces those policies (expiring, deleting, deduping
archives, removing orphaned vectors, compressing), oversized reasoning
histories are **compressed into summaries** while originals are
preserved and restorable, reused workflows **version** with full
**ancestry tracking**, merged memories keep their history through
**lineage edges**, the store can be **exported and imported** as
versioned JSON, **periodic snapshots** can be captured and restored, and
**storage statistics** (database / vector index / cache sizes, retention
counts) feed an expanded Memory Dashboard with retention management,
storage usage, snapshot management, a lineage explorer, and version
history.

The RC-6 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. The new
schema is fully additive (migration `0023`); the planner only *reads*
(unchanged), the execution engine only *records* (unchanged), the
autonomous runtime only *consults* (unchanged), all lifecycle logic
lives inside `copilot::memory`, and the repository owns every SQL
statement.

---

## Architecture

### What changed (additive only)

| Component | RC-6 M4 change |
|-----------|----------------|
| **`memory/lifecycle/` (new module)** | pure rules: `retention` (policy transitions, expiry checks, duplicate-archive planning, compression rules + summaries), `lineage` (version/merge graph building), `export` (versioned JSON format, compatibility rules — also the snapshot payload) |
| **`memory/lifecycle_repository.rs` (new)** | all lifecycle SQL: retention transitions, due/expired listings, reusable-ancestor lookup for versioning, lineage edges, compression candidates/archive/restore, snapshots (insert/list/prune/data/clear), storage statistics, orphaned-vector detection |
| **`memory/lifecycle/mod.rs` (new facade half)** | `impl MemoryEngine` for `set_retention`, `archive`, `expire`, `run_cleanup`, `compress_oversized`, `compress_memory`, `restore_compressed`, `lineage`, `export_json`, `import_json`, `create_snapshot`, `list_snapshots`, `restore_snapshot`, `storage_stats` |
| **`memory/cleanup_worker.rs` (new)** | background worker: periodic cleanup passes (15 min) + automatic snapshots (6 h), notify/shutdown like the indexer |
| `models.rs` | `RetentionPolicy`; `ExecutionMemoryRecord` gains `retention`, `retention_until`, `archived_at`, `expired_at`, `summary`, `compressed_at`, `version`, `parent_id`; new payloads `CleanupReport`, `CompressionResult`, `MemoryStorageStats`, `MemorySnapshot`, `MemoryLineage`, `LineageNode`, `MemoryExport`, `MemoryAcceptanceEntry`, `ImportResult`, `RestoreResult` |
| `MemoryRepository` | upsert/row-decode the new columns; `restore_acceptance` (exact ledger restore for import/snapshot restore) |
| `MemoryEngine` | versioning on capture (a successful run of a learned goal chains to its most-replayed ancestor and increments the version), merged-lineage edges on duplicate merge, expired records excluded from search/recommend/avoid, `lifecycle` repository wired in |
| `vector/` | `MemoryVectorSystem::cache_stats` (in-memory cache occupancy for the storage card) |
| IPC | 12 new thin commands: `memory_set_retention`, `memory_cleanup_now`, `memory_compress_oversized`, `memory_restore_compressed`, `memory_lineage`, `memory_export_json`, `memory_import_json`, `memory_snapshot_create`, `memory_snapshot_list`, `memory_snapshot_restore`, `memory_storage_stats` |
| Runtime wiring | `lib.rs` spawns the `MemoryCleanupWorker` next to the indexer |
| Frontend | Memory Dashboard gains a "Memory lifecycle" section: Storage usage card (db/vector/cache/snapshot sizes + retention counts), Retention manager (policy badges + Permanent/Archive/Expire actions + "Clean up now"), Snapshot manager (capture/list/restore), Lineage explorer (version ancestry, descendants, merges, export/import JSON) |

### Data flow (M4)

```
capture (engine / runtime terminal state)
   │  record_execution(…) ──► best_reusable_ancestor(goal fingerprint)
   │                            └─► version = ancestor.version + 1,
   │                                parent_id = ancestor,
   │                                memory_lineage edge (relation 'parent')
   ▼
execution_memory rows (retention = 'permanent' by default)
   │  user / dashboard: set_retention → temporary/archived/expired
   ▼
MemoryCleanupWorker (every 15 min + notify)
   │  run_cleanup():
   │    1. temporary past deadline   → retention = 'expired'
   │    2. expired                   → delete (vectors + ledger cascade)
   │    3. archived duplicates       → delete (live copy wins)
   │    4. orphaned vector rows      → remove
   │    5. oversized histories       → compress (summary + archive)
   ▼
lineage / export / snapshots / stats (facade reads, thin IPC, dashboard)
   export_json ──► MemoryExport (schema v1) ──► file / import_json
   create_snapshot ──► memory_snapshots (pruned to 10) ──► restore_snapshot
   storage_stats ──► database / vector / cache / retention sizes
```

---

## Deliverables

### Retention policies

`execution_memory` gains `retention` (`permanent` default, `temporary`,
`archived`, `expired`) with `retention_until` / `archived_at` /
`expired_at` stamps. `set_retention` is the single transition point
(`Permanent` revives a record by clearing the state; `Temporary`
requires a deadline; `Expired` schedules deletion by the next pass).
Temporary memories past their deadline are surfaced by
`list_due_temporary` and flipped by the worker.

### Automatic cleanup

The `MemoryCleanupWorker` (mirroring the `MemoryIndexer` loop:
interval + notify + shutdown) runs `run_cleanup()`: expire-due →
delete-expired (vectors via `MemoryVectorSystem::remove`, rows via the
repository) → duplicate archives (pure rule: a live copy — or a newer
archived copy — wins) → orphaned vector rows (safety net over the
`LEFT JOIN` SQL) → budgeted compression. Every pass returns a
`CleanupReport` for the dashboard.

### Memory compression

Records with ≥ 80 reasoning events or ≥ 150 steps are compressible.
Compression stores the originals in `memory_compression_archive`, writes
a head + tail + count summary into `summary`, and replaces the in-row
reasoning with the summary entry (`compressed_at` stamped). `restore_compressed`
reverses this in a transaction. Compression never loses history.

### Memory versioning + lineage

A successful capture looks up the most-replayed successful memory with
the same goal fingerprint; when found, the new record becomes version
`parent.version + 1` with `parent_id` set, and a `memory_lineage` edge
(`relation = 'parent'`) is recorded. Duplicate merges (M3) now record
`relation = 'merged'` edges *before* the deletion, so merged histories
survive. `build_lineage` (pure) assembles ancestors (oldest first), the
root id, direct descendants, merged-into lists, and the merged-into id
from records + edges.

### Import / export

`MemoryExport` (schema `1`, `exported_at`, records, acceptance ledger)
is the single portable format used by exports, imports, snapshots, and
restores — guaranteed compatible by construction. Imports are idempotent
by record id (existing ids skipped), restore the acceptance ledger to
its exact tallies, and reject newer schema versions.

### Snapshots

`memory_snapshots` stores full-store export JSON under a label. The
worker captures an `auto` snapshot every 6 h; the dashboard can capture
on demand; the store prunes to the 10 newest. `restore_snapshot` wipes
the store in a cascade (acceptance, lineage, compression archive, vector
rows all follow), re-inserts the snapshot's records + ledger, and
rebuilds the vector index so retrieval is immediately correct.

### Storage statistics

`storage_stats()` exposes the SQLite file size (`PRAGMA page_count *
page_size`), stored vector bytes, persistent cache entries + bytes,
in-memory cache occupancy, records per retention policy, snapshot count
+ bytes, and compressed/archive counts — all pure SQL in the lifecycle
repository.

---

## Testing

- **New pure-rule tests** (`lifecycle/retention.rs`, `lineage.rs`,
  `export.rs`): due-expiry rules, duplicate-archive preferences,
  compressibility + summary shape, ancestry/merge graph building,
  export round trips, schema-version rejection, import partitioning.
- **New SQL tests** (`lifecycle_repository.rs`): retention transitions,
  due/expired listings, reusable-ancestor preference (replay count, then
  created; expired excluded), lineage edge dedupe + cascade, compression
  candidate listing + archive restore, snapshot insert/list/prune, and
  storage statistics.
- **New engine tests** (`lifecycle_engine_tests.rs`): retention
  transitions through the facade, version chaining + lineage walk,
  expired records hidden from search/recommend, compression round trip,
  export/import idempotency, snapshot create/list/restore (with vector
  rebuild), storage stats, and the cleanup pass (expire → delete,
  duplicate archives, compression).
- **Worker tests** (`cleanup_worker.rs`): pass lifecycle, snapshot
  interval gating, shutdown.
- Full suite: **402 lib tests + 5 integration tests pass**; 31 frontend
  tests pass; `tsc -b` and `vite build` clean.

---

## Compatibility

- No existing IPC command changed shape; 12 new commands added.
- Migration `0023` is strictly additive (new columns with defaults,
  new tables, new indexes).
- Old databases upgrade in place; existing records default to
  `permanent`, version 1, no parent.
- The export format is versioned; future schema changes bump
  `MEMORY_EXPORT_SCHEMA_VERSION` and older builds refuse newer payloads.

---

## Verification

Run: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`,
`cargo build`, `cargo test`, `npm run build`, `npx tsc -b`, `npm test`
— all pass with zero warnings.

Committed to `main` and pushed to `origin/main`.
