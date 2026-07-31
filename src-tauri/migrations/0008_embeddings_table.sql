-- Embeddings storage table for Phase 5 ML Layer.
-- Stores vector embeddings separately from ml_metadata (which only stores
-- the embedding_id reference). This design allows:
-- - Efficient vector storage as BLOB
-- - Independent lifecycle management (embeddings can be regenerated)
-- - Future migration to dedicated vector database without schema changes
--
-- ## BLOB Storage Decision
--
-- Embeddings are stored as BLOB (binary large object) containing f32 values
-- serialized as little-endian bytes (4 bytes per float). This was chosen over:
--
-- 1. JSON array: Would be human-readable but 10-20x larger storage footprint,
--    slower to parse, and suffer from float precision loss during text round-trip.
--
-- 2. Comma-separated TEXT: Similar issues to JSON (size, parsing cost) plus
--    no standard serialization for floats.
--
-- 3. One row per dimension: Would explode a 384-dim vector into 384 rows,
--    destroying query performance and bloating the database.
--
-- Tradeoffs of BLOB approach:
--   ✓ Compact: 4 bytes/float, no text overhead
--   ✓ Fast: Single row fetch, no parsing beyond memcpy
--   ✓ Exact: No float→string→float precision loss
--   ✗ Opaque: Not human-readable in sqlite3 CLI (shows as hex)
--   ✗ No SQL operators: Can't do vector math in SQL (but we don't need to—
--     similarity search will happen in Rust via ONNX Runtime or external
--     vector DB, not via SQLite queries)
--
-- For development/debugging, the Rust deserialization in EmbeddingRow::try_from
-- provides the human-readable Vec<f32> representation.

CREATE TABLE embeddings (
    id              TEXT PRIMARY KEY NOT NULL,
    vector          BLOB NOT NULL,
    dimensions      INTEGER NOT NULL CHECK (dimensions > 0),
    model_version   TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_embeddings_model_version ON embeddings (model_version);
