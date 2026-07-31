-- Workspace health history table
-- Stores historical health assessments for workspaces

CREATE TABLE IF NOT EXISTS workspace_health_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL,
    overall_score REAL NOT NULL,
    factors_json TEXT NOT NULL, -- JSON serialized WorkspaceHealth
    calculated_at TEXT NOT NULL, -- ISO 8601 timestamp
    
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

-- Index for efficient lookups by workspace and time
CREATE INDEX idx_health_workspace_time ON workspace_health_history(workspace_id, calculated_at DESC);

-- Index for finding recent health assessments
CREATE INDEX idx_health_calculated_at ON workspace_health_history(calculated_at DESC);
