-- ---------------------------------------------------------------------
-- RC-8 M3: Context Intelligence
-- Graph-derived workspace similarity + cross-workspace relationships,
-- goal-similarity clusters, and graph context snapshots. All additive;
-- nothing here rewrites an existing table.
-- ---------------------------------------------------------------------

-- Persisted cross-workspace relationships discovered from the graph.
-- Unlike `workspace_relationships_v2` (Phase 5E, file/folder/tech based),
-- these are derived from graph structure: shared goal terms, edges that
-- bridge two workspaces, and semantic profile similarity.
CREATE TABLE IF NOT EXISTS context_intel_workspace_relations (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    source_workspace_id TEXT NOT NULL,
    target_workspace_id TEXT NOT NULL,
    similarity          REAL NOT NULL DEFAULT 0.0, -- combined score 0.0..1.0
    confidence          REAL NOT NULL DEFAULT 0.0, -- input confidence 0.0..1.0
    signals             TEXT NOT NULL DEFAULT '[]', -- JSON array of SignalEvidence
    -- RFC3339 microsecond timestamps (datetime('now') is not RFC3339).
    last_updated        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (source_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (target_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_context_intel_relations_unique
    ON context_intel_workspace_relations(source_workspace_id, target_workspace_id);
CREATE INDEX IF NOT EXISTS idx_context_intel_relations_rank
    ON context_intel_workspace_relations(source_workspace_id, similarity DESC);

-- Graph context snapshots: a persisted picture of a workspace's context
-- at a point in time (node/edge counts, confidence, and a knowledge
-- summary). Distinct from `context_snapshots` (Phase 5E session data).
CREATE TABLE IF NOT EXISTS context_intel_snapshots (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id  TEXT NOT NULL,
    snapshot_type TEXT NOT NULL DEFAULT 'manual', -- 'manual', 'auto'
    node_count    INTEGER NOT NULL DEFAULT 0,
    edge_count    INTEGER NOT NULL DEFAULT 0,
    confidence    REAL NOT NULL DEFAULT 0.0,
    summary       TEXT NOT NULL DEFAULT '[]', -- JSON array of SummaryPoint
    payload       TEXT NOT NULL DEFAULT '{}',
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_context_intel_snapshots_workspace
    ON context_intel_snapshots(workspace_id, created_at DESC);

-- Goal-similarity clusters: groups of goal-bearing nodes (executions,
-- planner reports, memory records) that share topical vocabulary.
-- Scoped to one workspace, or to the whole graph (workspace_id NULL).
CREATE TABLE IF NOT EXISTS context_intel_clusters (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id TEXT, -- NULL = whole-graph scope
    name         TEXT NOT NULL,
    member_count INTEGER NOT NULL DEFAULT 0,
    members      TEXT NOT NULL DEFAULT '[]', -- JSON array of ClusterMember
    centroid     TEXT NOT NULL DEFAULT '[]', -- centroid terms
    confidence   REAL NOT NULL DEFAULT 0.0,
    -- RFC3339 microsecond timestamp (datetime('now') is not RFC3339).
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_context_intel_clusters_workspace
    ON context_intel_clusters(workspace_id);