-- Action History Table
-- Stores executed actions for audit trail and undo support

CREATE TABLE action_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_type TEXT NOT NULL,
    workspace_id INTEGER,
    recommendation_id TEXT,
    executed_at TEXT NOT NULL DEFAULT (datetime('now')),
    success INTEGER NOT NULL DEFAULT 1,
    metadata TEXT NOT NULL DEFAULT '{}',
    undo_state TEXT,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_action_history_workspace ON action_history(workspace_id);
CREATE INDEX idx_action_history_executed_at ON action_history(executed_at DESC);
CREATE INDEX idx_action_history_recommendation ON action_history(recommendation_id);
