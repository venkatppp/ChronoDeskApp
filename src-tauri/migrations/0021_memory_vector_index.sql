-- Migration 0021: Memory Vector Index & Embedding Cache (RC-6 M2)
-- Production-quality vector memory system. Two new tables:
--
-- 1. `memory_vector_index` - the durable vector index over execution
--    memory. One row per memory record, holding the goal text and its
--    embedding. `indexed_at` tracks when the embedding was produced so
--    the background indexer only re-embeds records that are new or
--    whose goal changed since the last index pass (incremental
--    embedding generation / automatic re-indexing on change).
--
-- 2. `memory_embedding_cache` - persistent text -> embedding cache.
--    Query and goal texts are looked up here before embedding so
--    repeated searches never re-embed the same string, even across
--    restarts.

CREATE TABLE IF NOT EXISTS memory_vector_index (
    memory_id TEXT PRIMARY KEY NOT NULL,
    text_hash TEXT NOT NULL,
    text TEXT NOT NULL,
    embedding BLOB,
    dim INTEGER,
    indexed_at TEXT,
    FOREIGN KEY (memory_id) REFERENCES execution_memory(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memory_vector_index_text_hash
    ON memory_vector_index(text_hash);

CREATE TABLE IF NOT EXISTS memory_embedding_cache (
    text_hash TEXT PRIMARY KEY NOT NULL,
    text TEXT NOT NULL,
    embedding BLOB NOT NULL,
    dim INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
