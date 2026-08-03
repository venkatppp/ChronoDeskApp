-- ---------------------------------------------------------------------
-- RC-10 M1: Production Hardening — Performance & Profiling
--
-- Operational ledger behind the performance dashboard. All additive;
-- nothing here rewrites an existing table.
--
-- 1. `performance_profiles` — one row per measured operation
--    (command / service / repository / worker / engine), the append-only
--    history the profiler flushes to. The in-memory ring keeps the
--    live window; this table survives restarts for history + the
--    optimizer's slow-query analysis.
-- 2. `benchmark_runs` — persisted results of the micro-benchmark suites
--    (planner / execution / memory / graph / vector) so results are
--    comparable across sessions.
-- 3. `startup_profiles` — per-stage startup timings, grouped by `run_id`
--    so one launch renders as one timeline and history is queryable.
-- ---------------------------------------------------------------------

-- Append-only sampled operations. `category` is one of 'command' |
-- 'service' | 'repository' | 'worker' | 'engine'; `metadata` is a
-- free-form JSON payload with operation-specific context.
CREATE TABLE IF NOT EXISTS performance_profiles (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    category      TEXT NOT NULL,           -- 'command' | 'service' | 'repository' | 'worker' | 'engine'
    name          TEXT NOT NULL,
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    metadata      TEXT NOT NULL DEFAULT '{}',
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    occurred_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_performance_profiles_recent
    ON performance_profiles(occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_performance_profiles_category
    ON performance_profiles(category, name);

-- Persisted micro-benchmark results. One row per benchmark operation
-- within a suite run (`suite_name` groups a run's benchmarks together).
-- `ok` distinguishes a completed measurement from a skipped/failed one
-- (e.g. planner with no available tools).
CREATE TABLE IF NOT EXISTS benchmark_runs (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    suite_name         TEXT NOT NULL,
    category           TEXT NOT NULL,   -- 'planner' | 'execution' | 'memory' | 'graph' | 'vector'
    benchmark_name     TEXT NOT NULL,
    operation          TEXT NOT NULL,
    iterations         INTEGER NOT NULL DEFAULT 1,
    duration_ms        INTEGER NOT NULL DEFAULT 0,
    ok                 INTEGER NOT NULL DEFAULT 1,
    throughput_per_sec REAL,
    payload            TEXT NOT NULL DEFAULT '{}',
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_benchmark_runs_recent
    ON benchmark_runs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_benchmark_runs_suite
    ON benchmark_runs(suite_name);

-- Per-stage startup timings, grouped into runs by `run_id` so a launch
-- phase surfaces as a single timeline in the frontend.
CREATE TABLE IF NOT EXISTS startup_profiles (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id        TEXT NOT NULL,           -- groups one startup's stages
    stage         TEXT NOT NULL,
    label         TEXT NOT NULL,
    duration_ms   INTEGER NOT NULL DEFAULT 0,
    started_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    recorded_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_startup_profiles_run
    ON startup_profiles(run_id, started_at);
CREATE INDEX IF NOT EXISTS idx_startup_profiles_recent
    ON startup_profiles(recorded_at DESC);