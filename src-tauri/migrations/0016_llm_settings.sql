-- Migration 0016: LLM Settings
-- Stores LLM provider configuration for AI Copilot

CREATE TABLE IF NOT EXISTS llm_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Single row table
    provider TEXT NOT NULL CHECK (provider IN ('openai', 'ollama', 'custom')),
    base_url TEXT NOT NULL,
    api_key TEXT NOT NULL, -- Stored encrypted in production
    model TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 2000,
    context_window INTEGER NOT NULL DEFAULT 128000,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Token usage tracking
CREATE TABLE IF NOT EXISTS llm_usage (
    id TEXT PRIMARY KEY,
    conversation_id TEXT,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (conversation_id) REFERENCES copilot_conversations(id) ON DELETE SET NULL
);

CREATE INDEX idx_llm_usage_conversation ON llm_usage(conversation_id);
CREATE INDEX idx_llm_usage_created_at ON llm_usage(created_at);

-- Insert default settings (empty API key - must be configured)
INSERT INTO llm_settings (id, provider, base_url, api_key, model, temperature, max_tokens, context_window)
VALUES (1, 'openai', 'https://api.openai.com/v1', '', 'gpt-4o-mini', 0.7, 2000, 128000)
ON CONFLICT (id) DO NOTHING;
