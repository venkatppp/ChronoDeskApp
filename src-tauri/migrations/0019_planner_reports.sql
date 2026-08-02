-- Migration 0019: Planner Reports (execution progress streaming, RC-5 M5)
-- Persists the autonomous planner's final run summary per execution so the
-- live Execution Dashboard and frontend reconnect both recover replan/retry
-- accounting (completed/skipped/replaced tasks, replan count) even after an
-- application restart.

CREATE TABLE IF NOT EXISTS plan_execution_reports (
    execution_id TEXT PRIMARY KEY NOT NULL,
    report TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

CREATE INDEX idx_plan_execution_reports_created ON plan_execution_reports(created_at DESC);