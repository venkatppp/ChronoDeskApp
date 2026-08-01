-- Migration 0013: UUID migration for intelligence subsystem
-- Converts workspace_id from INTEGER to TEXT (UUID) in intelligence tables

-- Step 1: Create new health history table with UUID
CREATE TABLE workspace_health_history_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    overall_score REAL NOT NULL,
    factors_json TEXT NOT NULL,
    calculated_at TEXT NOT NULL,
    
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

-- Step 2: Migrate existing data (if any exists, skip since we can't convert INTEGER to UUID)
-- The intelligence system is new in Phase 5G, so no data loss expected

-- Step 3: Drop old table first (which removes old indexes)
DROP TABLE workspace_health_history;

-- Step 4: Rename new table
ALTER TABLE workspace_health_history_new RENAME TO workspace_health_history;

-- Step 5: Create indexes on the renamed table
CREATE INDEX idx_health_workspace_time ON workspace_health_history(workspace_id, calculated_at DESC);
CREATE INDEX idx_health_calculated_at ON workspace_health_history(calculated_at DESC);

-- Step 6: Migrate action_history table
CREATE TABLE action_history_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT,
    action_type TEXT NOT NULL,
    action_data TEXT NOT NULL,
    success INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    executed_at TEXT NOT NULL,
    
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

-- Drop old table first
DROP TABLE action_history;

-- Rename new table
ALTER TABLE action_history_new RENAME TO action_history;

-- Create indexes on renamed table
CREATE INDEX idx_action_workspace_time ON action_history(workspace_id, executed_at DESC);
CREATE INDEX idx_action_executed_at ON action_history(executed_at DESC);
