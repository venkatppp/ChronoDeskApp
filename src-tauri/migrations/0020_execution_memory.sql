-- Migration 0020: Execution Memory Store (RC-6 M1)
-- Lets ChronoDesk learn from previous executions: every plan execution,
-- planner report, and autonomous session reaches a durable memory row that
-- the semantic retrieval + learning engines rank and reuse when planning
-- new goals.

CREATE TABLE IF NOT EXISTS execution_memory (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('execution', 'planner_report', 'autonomous_session')),
    source_id TEXT NOT NULL,
    workspace_id TEXT,
    goal TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('success', 'failed', 'cancelled')),
    plan TEXT,
    steps TEXT NOT NULL DEFAULT '[]',
    reasoning TEXT NOT NULL DEFAULT '[]',
    tools_used TEXT NOT NULL DEFAULT '[]',
    failed_steps TEXT NOT NULL DEFAULT '[]',
    error TEXT,
    outcome TEXT NOT NULL DEFAULT '{}',
    goal_embedding BLOB,
    goal_embedding_dim INTEGER,
    replay_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(kind, source_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX idx_execution_memory_kind ON execution_memory(kind);
CREATE INDEX idx_execution_memory_status ON execution_memory(status);
CREATE INDEX idx_execution_memory_goal ON execution_memory(goal);
CREATE INDEX idx_execution_memory_created ON execution_memory(created_at DESC);
