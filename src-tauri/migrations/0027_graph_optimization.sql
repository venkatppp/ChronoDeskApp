-- ---------------------------------------------------------------------
-- RC-8 M4: Knowledge Graph Optimization & Scale
--
-- Operational tables behind the optimization/scale milestone. All
-- additive; nothing here rewrites an existing table.
--
-- 1. `graph_integrity_issues` — persisted findings from integrity
--    checks (orphan edges, dangling workspaces, malformed nodes,
--    invalid confidence) with an open/resolved lifecycle so the
--    frontend can show history and the repair pass can close rows.
-- 2. `graph_maintenance_runs` — one row per integrity/repair/cleanup/
--    consistency/benchmark pass, so maintenance history survives
--    restarts.
-- 3. `graph_query_metrics` — append-only per-operation latency/volume
--    ledger for the performance dashboard (pagination, ranked search,
--    vector search, parallel traversal, ...).
-- 4. `graph_benchmarks` — persisted benchmark suite results (per-suite
--    runs of the micro-benchmarks the health service executes).
-- ---------------------------------------------------------------------

-- Persisted integrity findings. `issue_type` is one of the four
-- integrity categories; `status` tracks the open -> resolved lifecycle.
CREATE TABLE IF NOT EXISTS graph_integrity_issues (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_type    TEXT NOT NULL,            -- 'orphan_edge' | 'dangling_workspace' | 'malformed_node' | 'invalid_confidence'
    severity      TEXT NOT NULL DEFAULT 'warning', -- 'info' | 'warning' | 'critical'
    node_type     TEXT,                     -- graph node type, when the issue targets a node
    entity_id     TEXT,                     -- the affected node/edge id (UUID text)
    detail        TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'open', -- 'open' | 'resolved'
    -- RFC3339 microsecond timestamps (datetime('now') is not RFC3339).
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    resolved_at   TEXT
);

CREATE INDEX IF NOT EXISTS idx_graph_integrity_issues_status
    ON graph_integrity_issues(status, issue_type);
CREATE INDEX IF NOT EXISTS idx_graph_integrity_issues_created
    ON graph_integrity_issues(created_at DESC);

-- One row per maintenance pass (integrity check, repair, orphan
-- cleanup, consistency verification, benchmark suite). `summary` is a
-- free-form JSON payload with pass-specific accounting.
CREATE TABLE IF NOT EXISTS graph_maintenance_runs (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    run_type         TEXT NOT NULL,   -- 'integrity_check' | 'repair' | 'orphan_cleanup' | 'consistency' | 'benchmark'
    status           TEXT NOT NULL,   -- 'completed' | 'failed'
    issues_found     INTEGER NOT NULL DEFAULT 0,
    issues_resolved  INTEGER NOT NULL DEFAULT 0,
    duration_ms      INTEGER NOT NULL DEFAULT 0,
    summary          TEXT NOT NULL DEFAULT '{}',
    -- RFC3339 microsecond timestamps (datetime('now') is not RFC3339).
    started_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    finished_at      TEXT
);

CREATE INDEX IF NOT EXISTS idx_graph_maintenance_runs_recent
    ON graph_maintenance_runs(started_at DESC);

-- Append-only operation metrics for the performance dashboard. One row
-- per tracked query: which operation, how long, how many rows, and
-- whether the result came from the persisted query cache.
CREATE TABLE IF NOT EXISTS graph_query_metrics (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    operation     TEXT NOT NULL,
    scope         TEXT,              -- workspace id or 'all' (NULL = global)
    query         TEXT,              -- the search text, when applicable
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    rows_returned INTEGER NOT NULL DEFAULT 0,
    hit_cache     INTEGER NOT NULL DEFAULT 0,
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    occurred_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_graph_query_metrics_recent
    ON graph_query_metrics(occurred_at DESC);

-- Persisted benchmark results. Each row is one micro-benchmark within
-- a suite run (`suite_name` groups a run's benchmarks together).
CREATE TABLE IF NOT EXISTS graph_benchmarks (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_name     TEXT NOT NULL,
    benchmark_name TEXT NOT NULL,
    operation      TEXT NOT NULL,
    node_count     INTEGER NOT NULL DEFAULT 0,
    edge_count     INTEGER NOT NULL DEFAULT 0,
    duration_ms    INTEGER NOT NULL DEFAULT 0,
    payload        TEXT NOT NULL DEFAULT '{}',
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_graph_benchmarks_recent
    ON graph_benchmarks(created_at DESC);
