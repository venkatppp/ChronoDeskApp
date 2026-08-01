-- Learning Feedback Table
CREATE TABLE IF NOT EXISTS learning_feedback (
    id TEXT PRIMARY KEY NOT NULL,
    feedback_type TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    action TEXT NOT NULL,
    context TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_learning_feedback_target ON learning_feedback(target_type, target_id);
CREATE INDEX idx_learning_feedback_created ON learning_feedback(created_at DESC);
CREATE INDEX idx_learning_feedback_action ON learning_feedback(action);

-- User Preferences Table
CREATE TABLE IF NOT EXISTS learning_preferences (
    id TEXT PRIMARY KEY NOT NULL,
    preference_type TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    confidence REAL NOT NULL,
    evidence_count INTEGER NOT NULL,
    last_updated TEXT NOT NULL,
    UNIQUE(preference_type, key)
);

CREATE INDEX idx_learning_preferences_type ON learning_preferences(preference_type);
CREATE INDEX idx_learning_preferences_confidence ON learning_preferences(confidence DESC);

-- Behavioral Patterns Table
CREATE TABLE IF NOT EXISTS learning_patterns (
    id TEXT PRIMARY KEY NOT NULL,
    pattern_type TEXT NOT NULL,
    description TEXT NOT NULL,
    conditions TEXT NOT NULL,
    frequency REAL NOT NULL,
    confidence REAL NOT NULL,
    occurrences INTEGER NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL
);

CREATE INDEX idx_learning_patterns_type ON learning_patterns(pattern_type);
CREATE INDEX idx_learning_patterns_confidence ON learning_patterns(confidence DESC);
CREATE INDEX idx_learning_patterns_frequency ON learning_patterns(frequency DESC);

-- Confidence Adjustments Table
CREATE TABLE IF NOT EXISTS learning_confidence_adjustments (
    id TEXT PRIMARY KEY NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    original_confidence REAL NOT NULL,
    adjusted_confidence REAL NOT NULL,
    adjustment_factor REAL NOT NULL,
    reason TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE INDEX idx_learning_adjustments_target ON learning_confidence_adjustments(target_type, target_id);
CREATE INDEX idx_learning_adjustments_applied ON learning_confidence_adjustments(applied_at DESC);

-- Workflow Learning Table
CREATE TABLE IF NOT EXISTS learning_workflows (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_type TEXT NOT NULL UNIQUE,
    typical_duration_seconds INTEGER NOT NULL,
    typical_files TEXT NOT NULL,
    typical_time_of_day TEXT NOT NULL,
    success_indicators TEXT NOT NULL,
    confidence REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    last_updated TEXT NOT NULL
);

CREATE INDEX idx_learning_workflows_type ON learning_workflows(workflow_type);
CREATE INDEX idx_learning_workflows_confidence ON learning_workflows(confidence DESC);
