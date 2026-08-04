-- ---------------------------------------------------------------------
-- RC-10 M3: Data Integrity & Backup
--
-- Audit ledger behind the backup/integrity/maintenance subsystem. All
-- additive; nothing here rewrites an existing table.
--
-- `backup_runs` — one row per completed backup, restore stage, integrity
-- check and maintenance run, so every database-level intervention is
-- auditable after the fact. `kind` is one of
-- 'backup' | 'restore' | 'integrity' | 'maintenance'; `status` is
-- 'success' | 'failed' | 'staged'. `path` names the produced/staged file
-- (empty for in-place operations) and `checksum` is the SHA-256 of the
-- backup file so a restore can be verified before it is staged.
-- ---------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS backup_runs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    kind         TEXT NOT NULL,                  -- 'backup' | 'restore' | 'integrity' | 'maintenance'
    status       TEXT NOT NULL DEFAULT 'success',-- 'success' | 'failed' | 'staged'
    path         TEXT NOT NULL DEFAULT '',       -- backup filename / staged restore target
    size_bytes   INTEGER NOT NULL DEFAULT 0,
    checksum     TEXT NOT NULL DEFAULT '',       -- SHA-256 hex of the backup file
    detail       TEXT NOT NULL DEFAULT '',       -- human-readable summary / error
    duration_ms  INTEGER NOT NULL DEFAULT 0,
    started_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_backup_runs_recent
    ON backup_runs(completed_at DESC);
CREATE INDEX IF NOT EXISTS idx_backup_runs_kind
    ON backup_runs(kind, completed_at DESC);