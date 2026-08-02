-- Migration 0017: Plan Execution System
-- Adds execution tracking, progress events, and audit logs for approved plans

-- Plan execution progress tracking
CREATE TABLE IF NOT EXISTS plan_executions (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL,
    conversation_id TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    current_step INTEGER NOT NULL DEFAULT 0,
    total_steps INTEGER NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (conversation_id) REFERENCES copilot_conversations(id) ON DELETE SET NULL
);

CREATE INDEX idx_plan_executions_status ON plan_executions(status);
CREATE INDEX idx_plan_executions_conversation ON plan_executions(conversation_id);
CREATE INDEX idx_plan_executions_created ON plan_executions(created_at DESC);

-- Plan execution step tracking
CREATE TABLE IF NOT EXISTS plan_execution_steps (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    step_number INTEGER NOT NULL,
    description TEXT NOT NULL,
    tool_name TEXT,
    arguments TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'running', 'completed', 'failed', 'skipped')),
    result TEXT,
    error TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

CREATE INDEX idx_plan_execution_steps_execution ON plan_execution_steps(execution_id);
CREATE INDEX idx_plan_execution_steps_status ON plan_execution_steps(status);

-- Plan execution events (progress tracking)
CREATE TABLE IF NOT EXISTS plan_execution_events (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('started', 'step_started', 'step_completed', 'step_failed', 'paused', 'resumed', 'completed', 'failed', 'cancelled')),
    step_number INTEGER,
    message TEXT NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

CREATE INDEX idx_plan_execution_events_execution ON plan_execution_events(execution_id);
CREATE INDEX idx_plan_execution_events_type ON plan_execution_events(event_type);
CREATE INDEX idx_plan_execution_events_created ON plan_execution_events(created_at DESC);

-- Plan execution audit log
CREATE TABLE IF NOT EXISTS plan_execution_audit (
    id TEXT PRIMARY KEY NOT NULL,
    execution_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL CHECK(actor IN ('user', 'system', 'ai')),
    details TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);

CREATE INDEX idx_plan_execution_audit_execution ON plan_execution_audit(execution_id);
CREATE INDEX idx_plan_execution_audit_created ON plan_execution_audit(created_at DESC);

-- Conversation management extensions
-- Add pinned flag to conversations
ALTER TABLE copilot_conversations ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT 0;
CREATE INDEX idx_copilot_conversations_pinned ON copilot_conversations(pinned DESC, updated_at DESC);
