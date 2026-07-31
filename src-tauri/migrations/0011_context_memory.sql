-- Context Memory Table
-- Stores workspace context snapshots at meaningful milestones

CREATE TABLE context_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT NOT NULL,
    snapshot_type TEXT NOT NULL, -- 'manual', 'milestone', 'auto'
    captured_at TEXT NOT NULL DEFAULT (datetime('now')),
    active_files TEXT NOT NULL DEFAULT '[]', -- JSON array of file paths
    session_summary TEXT, -- JSON session data
    timeline_references TEXT, -- JSON array of timeline event IDs
    analytics_summary TEXT, -- JSON analytics snapshot
    health_score REAL,
    recommendations_summary TEXT, -- JSON array of recommendation IDs
    metadata TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_context_snapshots_workspace ON context_snapshots(workspace_id);
CREATE INDEX idx_context_snapshots_captured_at ON context_snapshots(captured_at DESC);
CREATE INDEX idx_context_snapshots_type ON context_snapshots(snapshot_type);

-- Workspace Relationships Table (enhanced)
-- Stores cross-workspace intelligence relationships

CREATE TABLE IF NOT EXISTS workspace_relationships_v2 (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_workspace_id TEXT NOT NULL,
    target_workspace_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL, -- 'shared_files', 'shared_folders', 'shared_tech', 'similar_patterns'
    strength REAL NOT NULL DEFAULT 0.5, -- 0.0 to 1.0
    evidence TEXT NOT NULL DEFAULT '{}', -- JSON metadata about the relationship
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_updated TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (source_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (target_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_workspace_relationships_v2_unique 
    ON workspace_relationships_v2(source_workspace_id, target_workspace_id, relationship_type);
CREATE INDEX idx_workspace_relationships_v2_source ON workspace_relationships_v2(source_workspace_id);
CREATE INDEX idx_workspace_relationships_v2_target ON workspace_relationships_v2(target_workspace_id);
CREATE INDEX idx_workspace_relationships_v2_strength ON workspace_relationships_v2(strength DESC);
