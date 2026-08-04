-- ---------------------------------------------------------------------
-- RC-10 M4: Security Hardening
--
-- Audit + policy ledgers behind the security subsystem. All additive;
-- nothing here rewrites an existing table.
--
-- `security_audit_log`      — append-only audit ledger. One row per
--   security-relevant intervention (startup validation, diagnostics runs,
--   monitor ticks, config changes, recommendation applies/dismissals,
--   prunes), so every action of the subsystem is auditable after the fact.
--   `action` is a snake_case string; `severity` is 'info' | 'warning' |
--   'critical'; `actor` is who/what triggered it ('system' | 'monitor' |
--   'user').
--
-- `security_config`         — key/value policy table. The security engine's
--   thresholds (monitor interval, audit retention, history retention) live
--   here so they are user-overridable and persisted, never hardcoded.
--
-- `security_findings`       — persisted per-run check results. `run_id`
--   groups one battery (startup validation, diagnostics run, monitor tick)
--   so the History surface can show whole runs; `passed` is 0/1 so the
--   score can be recomputed from history.
--
-- `security_recommendations` — persisted recommendations produced by the
--   pure rule engine, each with a status ('open' | 'applied' | 'dismissed')
--   so the user can ack or dismiss a suggestion without it reappearing.
-- ---------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS security_audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    action     TEXT NOT NULL,                       -- 'startup_validation' | 'diagnostics_run' | 'monitor_tick' | 'config_set' | 'recommendation_apply' | 'recommendation_dismiss' | 'prune'
    severity   TEXT NOT NULL DEFAULT 'info',        -- 'info' | 'warning' | 'critical'
    actor      TEXT NOT NULL DEFAULT 'system',      -- 'system' | 'monitor' | 'user'
    target     TEXT NOT NULL DEFAULT '',            -- db, file, secret, config key, recommendation id, ...
    detail     TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_security_audit_recent
    ON security_audit_log(created_at DESC);

CREATE TABLE IF NOT EXISTS security_config (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS security_findings (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id     TEXT NOT NULL,                       -- UUID grouping one battery
    category   TEXT NOT NULL,                       -- 'database' | 'files' | 'secrets' | 'backup' | 'input' | 'config'
    severity   TEXT NOT NULL DEFAULT 'info',        -- 'info' | 'warning' | 'critical'
    check_name TEXT NOT NULL,
    passed     INTEGER NOT NULL,                    -- 0/1
    detail     TEXT NOT NULL DEFAULT '',
    checked_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_security_findings_recent
    ON security_findings(checked_at DESC);
CREATE INDEX IF NOT EXISTS idx_security_findings_run
    ON security_findings(run_id, checked_at DESC);

CREATE TABLE IF NOT EXISTS security_recommendations (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    rule       TEXT NOT NULL,
    severity   TEXT NOT NULL DEFAULT 'info',        -- 'info' | 'warning' | 'critical'
    title      TEXT NOT NULL DEFAULT '',
    detail     TEXT NOT NULL DEFAULT '',
    status     TEXT NOT NULL DEFAULT 'open',        -- 'open' | 'applied' | 'dismissed'
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_security_recommendations_status
    ON security_recommendations(status, created_at DESC);
-- One recommendation per rule: lets `upsert_recommendation` target a rule
-- uniquely while preserving an applied/dismissed status across runs.
CREATE UNIQUE INDEX IF NOT EXISTS idx_security_recommendations_rule
    ON security_recommendations(rule);