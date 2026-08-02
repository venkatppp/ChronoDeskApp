-- Migration 0018: Execution Checkpoints (durable pause/resume, RC-5 M4)
-- Adds a single-row checkpoint per execution so a run can pause, resume,
-- and survive an application restart by reconstructing the plan DAG, the
-- execution context, and step outcomes without re-running completed steps.

-- One checkpoint row per execution (UPSERT keyed on execution_id).
CREATE TABLE IF NOT EXISTS plan_execution_checkpoints (
    execution_id TEXT PRIMARY KEY NOT NULL,
    plan TEXT NOT NULL,
    context TEXT NOT NULL,
    status TEXT NOT NULL,
    completed_steps TEXT NOT NULL,
    skipped_steps TEXT NOT NULL,
    failed_steps TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_plan_execution_checkpoints_updated
    ON plan_execution_checkpoints(updated_at DESC);

-- Widen plan_execution_events CHECK to accept checkpoint lifecycle events.
-- SQLite cannot alter a CHECK constraint, so the events table is rebuilt.
CREATE TABLE plan_execution_events_v18 (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('started', 'step_started', 'step_completed', 'step_failed', 'paused', 'resumed', 'completed', 'failed', 'cancelled', 'checkpoint_saved', 'checkpoint_loaded')),
    step_number INTEGER,
    message TEXT NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

INSERT INTO plan_execution_events_v18 (id, execution_id, event_type, step_number, message, metadata, created_at)
    SELECT id, execution_id, event_type, step_number, message, metadata, created_at
    FROM plan_execution_events;

DROP TABLE plan_execution_events;

ALTER TABLE plan_execution_events_v18 RENAME TO plan_execution_events;

CREATE INDEX idx_plan_execution_events_execution ON plan_execution_events(execution_id);
CREATE INDEX idx_plan_execution_events_type ON plan_execution_events(event_type);
CREATE INDEX idx_plan_execution_events_created ON plan_execution_events(created_at DESC);