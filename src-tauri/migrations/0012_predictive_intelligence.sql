-- Phase 5F: Predictive Intelligence & Workflow Automation
-- Learning profiles, automation rules, and execution logs

-- Learning profiles (aggregated user behavior, no personal content)
CREATE TABLE IF NOT EXISTS learning_profiles (
    user_id TEXT PRIMARY KEY NOT NULL,
    preferred_work_hours TEXT NOT NULL, -- JSON array of hours [0-23]
    avg_session_duration_seconds INTEGER NOT NULL DEFAULT 3600,
    workspace_switch_frequency REAL NOT NULL DEFAULT 0.0, -- switches per hour
    technology_preferences TEXT NOT NULL, -- JSON array of TechPreference
    focus_patterns TEXT NOT NULL, -- JSON FocusPattern
    last_updated TEXT NOT NULL -- ISO 8601 timestamp
);

-- Automation rules
CREATE TABLE IF NOT EXISTS automation_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1, -- Boolean: 1 = enabled, 0 = disabled
    trigger_type TEXT NOT NULL, -- workspace_activated, long_inactive, etc.
    trigger_config TEXT NOT NULL, -- JSON configuration for trigger
    action_type TEXT NOT NULL, -- restore_context, create_snapshot, etc.
    action_config TEXT NOT NULL, -- JSON configuration for action
    created_at TEXT NOT NULL -- ISO 8601 timestamp
);

CREATE INDEX IF NOT EXISTS idx_automation_rules_enabled 
    ON automation_rules(enabled, trigger_type);

-- Automation execution logs
CREATE TABLE IF NOT EXISTS automation_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER NOT NULL,
    executed_at TEXT NOT NULL, -- ISO 8601 timestamp
    success INTEGER NOT NULL, -- Boolean: 1 = success, 0 = failure
    result TEXT NOT NULL, -- JSON result or error details
    FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_automation_executions_rule_id 
    ON automation_executions(rule_id, executed_at DESC);

CREATE INDEX IF NOT EXISTS idx_automation_executions_executed_at 
    ON automation_executions(executed_at DESC);
