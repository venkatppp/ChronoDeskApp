-- Copilot Conversations Table
CREATE TABLE IF NOT EXISTS copilot_conversations (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX idx_copilot_conversations_workspace ON copilot_conversations(workspace_id);
CREATE INDEX idx_copilot_conversations_updated ON copilot_conversations(updated_at DESC);

-- Copilot Messages Table
CREATE TABLE IF NOT EXISTS copilot_messages (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    tool_calls TEXT,
    reasoning TEXT,
    sources TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES copilot_conversations(id) ON DELETE CASCADE
);

CREATE INDEX idx_copilot_messages_conversation ON copilot_messages(conversation_id);
CREATE INDEX idx_copilot_messages_created ON copilot_messages(created_at DESC);

-- Copilot Tool Executions Table
CREATE TABLE IF NOT EXISTS copilot_tool_executions (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL,
    result TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'success', 'failed', 'cancelled')),
    requires_confirmation BOOLEAN NOT NULL DEFAULT 0,
    confirmed BOOLEAN NOT NULL DEFAULT 0,
    error TEXT,
    executed_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES copilot_messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_copilot_tool_executions_message ON copilot_tool_executions(message_id);
CREATE INDEX idx_copilot_tool_executions_status ON copilot_tool_executions(status);

-- Copilot Context Snapshots Table (captures workspace state at conversation time)
CREATE TABLE IF NOT EXISTS copilot_context_snapshots (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    workspace_id TEXT,
    active_files TEXT NOT NULL,
    recent_events TEXT NOT NULL,
    session_summary TEXT,
    captured_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES copilot_conversations(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE INDEX idx_copilot_context_snapshots_conversation ON copilot_context_snapshots(conversation_id);
CREATE INDEX idx_copilot_context_snapshots_workspace ON copilot_context_snapshots(workspace_id);

-- Copilot Plans Table (multi-step action plans)
CREATE TABLE IF NOT EXISTS copilot_plans (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL,
    goal TEXT NOT NULL,
    steps TEXT NOT NULL,
    current_step INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK(status IN ('planning', 'executing', 'completed', 'failed', 'cancelled')),
    created_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (message_id) REFERENCES copilot_messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_copilot_plans_message ON copilot_plans(message_id);
CREATE INDEX idx_copilot_plans_status ON copilot_plans(status);
