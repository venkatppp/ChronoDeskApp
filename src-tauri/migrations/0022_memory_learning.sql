-- Migration 0022: Adaptive Learning (RC-6 M3)
-- Adds the recommendation acceptance ledger that lets the learning
-- engine adapt its recommendation weights and confidence from user
-- feedback. Additive only: no existing table is modified.

CREATE TABLE IF NOT EXISTS memory_acceptance (
    memory_id TEXT PRIMARY KEY NOT NULL,
    accepted_count INTEGER NOT NULL DEFAULT 0,
    rejected_count INTEGER NOT NULL DEFAULT 0,
    first_feedback_at TEXT NOT NULL,
    last_feedback_at TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES execution_memory(id) ON DELETE CASCADE
);

CREATE INDEX idx_memory_acceptance_last_feedback ON memory_acceptance(last_feedback_at DESC);
