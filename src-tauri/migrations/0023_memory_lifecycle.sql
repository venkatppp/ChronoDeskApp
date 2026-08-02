-- Migration 0023: Memory Lifecycle (RC-6 M4)
-- Turns execution memory into a managed long-term store:
--
-- 1. Retention policies on every record: `permanent` (default),
--    `temporary` (auto-expires at `retention_until`), `archived`
--    (kept but out of active circulation), and `expired` (deleted by
--    the cleanup worker). `archived_at` / `expired_at` stamp the
--    transitions for the dashboard.
--
-- 2. Versioning + lineage: every reused workflow produces a new
--    version of the record (`version`, `parent_id`), and merge /
--    derivation edges live in `memory_lineage` so ancestry, merges,
--    and workflow evolution can be walked and visualized.
--
-- 3. Compression: large reasoning histories are summarized in place
--    (`summary`, `compressed_at`) while the originals are preserved
--    in `memory_compression_archive` for restoration.
--
-- 4. Snapshots: periodic full-store JSON dumps in `memory_snapshots`
--    that can be restored (import/export compatible format).
--
-- All changes are additive: existing columns/tables are untouched.

ALTER TABLE execution_memory ADD COLUMN retention TEXT NOT NULL DEFAULT 'permanent';
ALTER TABLE execution_memory ADD COLUMN retention_until TEXT;
ALTER TABLE execution_memory ADD COLUMN archived_at TEXT;
ALTER TABLE execution_memory ADD COLUMN expired_at TEXT;
ALTER TABLE execution_memory ADD COLUMN summary TEXT;
ALTER TABLE execution_memory ADD COLUMN compressed_at TEXT;
ALTER TABLE execution_memory ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE execution_memory ADD COLUMN parent_id TEXT;

CREATE INDEX IF NOT EXISTS idx_execution_memory_retention ON execution_memory(retention);

-- Lineage edges: `parent` = a new run derived from a reused workflow,
-- `merged` = a duplicate merged into the keeper. Deleting either end
-- of an edge removes the edge (edges are bookkeeping, not content).
CREATE TABLE IF NOT EXISTS memory_lineage (
    id TEXT PRIMARY KEY NOT NULL,
    memory_id TEXT NOT NULL,
    parent_id TEXT NOT NULL,
    relation TEXT NOT NULL CHECK(relation IN ('parent', 'merged')),
    created_at TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES execution_memory(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES execution_memory(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memory_lineage_memory ON memory_lineage(memory_id);
CREATE INDEX IF NOT EXISTS idx_memory_lineage_parent ON memory_lineage(parent_id);

-- One edge per (child, parent, relation): re-recording a merge or a
-- version derivation must never duplicate the lineage row.
CREATE UNIQUE INDEX IF NOT EXISTS idx_memory_lineage_unique
    ON memory_lineage(memory_id, parent_id, relation);

-- Original reasoning/steps preserved when a record is compressed, so
-- summaries never destroy history irreversibly.
CREATE TABLE IF NOT EXISTS memory_compression_archive (
    memory_id TEXT PRIMARY KEY NOT NULL,
    original_reasoning TEXT NOT NULL,
    original_steps TEXT NOT NULL,
    compressed_at TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES execution_memory(id) ON DELETE CASCADE
);

-- Periodic full-store snapshots (JSON, import/export compatible).
CREATE TABLE IF NOT EXISTS memory_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_snapshots_created ON memory_snapshots(created_at DESC);
