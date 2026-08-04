-- ---------------------------------------------------------------------
-- RC-10 M2: Reliability & Recovery
--
-- Operational ledger behind the fault-tolerance subsystem. All additive;
-- nothing here rewrites an existing table.
--
-- 1. `recovery_journal` — append-only log of every reliability event
--    (checkpoint, heartbeat, crash, rollback, recovery run, self-healing
--    action, health snapshot). Checkpoints live here (`entry_type =
--    'checkpoint'`): each carries the caller's `state` (`running` /
--    `clean`), the active-job payload, and a SHA-256 `checksum` so a
--    half-written row can be detected after a crash.
-- 2. `crash_reports` — one row per detected crash, with the component,
--    crash type, message, stack trace and whether automatic recovery
--    already handled it (`was_recovered`).
-- 3. `worker_health` — one row per monitored background worker (and the
--    runtime's own liveness marker). Heartbeats refresh `last_heartbeat`;
--    the watchdog increments `consecutive_misses` until self-healing
--    restarts the worker's monitoring state.
-- 4. `recovery_history` — one row per completed recovery run (startup
--    crash recovery, watchdog pass, rollback), so every automatic
--    intervention is auditable after the fact.
-- ---------------------------------------------------------------------

-- Append-only journal of reliability events. `entry_type` is one of
-- 'checkpoint' | 'heartbeat' | 'crash' | 'rollback' | 'recovery' |
-- 'self_healing' | 'health'; `payload` is the free-form JSON detail.
CREATE TABLE IF NOT EXISTS recovery_journal (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_type  TEXT NOT NULL,
    scope       TEXT NOT NULL DEFAULT '',      -- 'startup' | 'watchdog' | 'runtime' | ...
    entity      TEXT NOT NULL DEFAULT '',      -- worker name / run id / 'app'
    state       TEXT NOT NULL DEFAULT '',      -- caller-provided state label
    payload     TEXT NOT NULL DEFAULT '{}',
    checksum    TEXT NOT NULL DEFAULT '',
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_recovery_journal_recent
    ON recovery_journal(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recovery_journal_entity
    ON recovery_journal(entity, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recovery_journal_checkpoints
    ON recovery_journal(entry_type, created_at DESC);

-- One row per detected crash. `was_recovered` flips to 1 once automatic
-- recovery has handled the report so the history view can distinguish
-- "handled" from "still open".
CREATE TABLE IF NOT EXISTS crash_reports (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    component     TEXT NOT NULL,
    crash_type    TEXT NOT NULL DEFAULT 'unknown',  -- 'panic' | 'timeout' | 'worker_failure' | 'database' | 'checkpoint_corrupt' | 'unknown'
    severity      TEXT NOT NULL DEFAULT 'error',    -- 'error' | 'critical'
    message       TEXT NOT NULL DEFAULT '',
    stack_trace   TEXT NOT NULL DEFAULT '',
    metadata      TEXT NOT NULL DEFAULT '{}',
    was_recovered INTEGER NOT NULL DEFAULT 0,
    recovered_at  TEXT,
    reported_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_crash_reports_recent
    ON crash_reports(reported_at DESC);
CREATE INDEX IF NOT EXISTS idx_crash_reports_unhandled
    ON crash_reports(was_recovered, reported_at DESC);

-- One row per monitored worker (upsert keyed on `worker`). Heartbeats
-- refresh `last_heartbeat`; the watchdog marks stalled workers and
-- self-healing resets the row once its monitoring state "restarts".
CREATE TABLE IF NOT EXISTS worker_health (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    worker             TEXT NOT NULL UNIQUE,
    status             TEXT NOT NULL DEFAULT 'healthy',  -- 'healthy' | 'stalled' | 'failed' | 'idle'
    last_heartbeat     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    consecutive_misses INTEGER NOT NULL DEFAULT 0,
    execution_count    INTEGER NOT NULL DEFAULT 0,
    error_count        INTEGER NOT NULL DEFAULT 0,
    last_error         TEXT NOT NULL DEFAULT '',
    details            TEXT NOT NULL DEFAULT '{}',
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_worker_health_heartbeat
    ON worker_health(last_heartbeat DESC);

-- One row per completed recovery run (startup detection, watchdog pass,
-- rollback). `actions` lists the recovery actions executed and
-- `recovered_jobs` the jobs resumed/rolled back, both as JSON arrays.
CREATE TABLE IF NOT EXISTS recovery_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id        TEXT NOT NULL,                    -- groups runs; uuid per run
    trigger       TEXT NOT NULL,                    -- 'startup' | 'crash' | 'watchdog' | 'rollback' | 'manual'
    outcome       TEXT NOT NULL DEFAULT 'no_action',-- 'recovered' | 'no_action' | 'failed' | 'rolled_back' | 'partial'
    status        TEXT NOT NULL DEFAULT 'success',  -- 'success' | 'partial' | 'failed'
    actions       TEXT NOT NULL DEFAULT '[]',
    recovered_jobs TEXT NOT NULL DEFAULT '[]',
    rolled_back_to TEXT NOT NULL DEFAULT '',        -- checkpoint journal id
    errors        TEXT NOT NULL DEFAULT '[]',
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    started_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_recovery_history_recent
    ON recovery_history(completed_at DESC);
CREATE INDEX IF NOT EXISTS idx_recovery_history_trigger
    ON recovery_history(trigger, completed_at DESC);