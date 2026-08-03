-- Migration 0025: Live Knowledge Graph (RC-8 M2)
--
-- Additive to the 0024 knowledge graph schema. Three additions:
--
-- 1. `graph_relationships.confidence` — a per-edge confidence in [0,1]
--    that semantic `related_to` edges start with (hered similarity) and
--    that decays over time via the maintenance pass. Structural edges
--    keep the default 1.0 and are never decayed.
-- 2. `graph_sync_state` — per-aggregate watermark for incremental,
--    event-driven sync: the source aggregate is processed only when it
--    changed since the watermark, so a background worker keeps the graph
--    live without full rebuilds.
-- 3. `graph_query_cache` — persisted, TTL-scoped cache for expensive
--    derived results (analytics, multi-hop expansion, recommendations,
--    cached subgraphs), keyed by an argument digest. Invalidated by every
--    graph write (sync, semantic rebuild, decay).

ALTER TABLE graph_relationships ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0
    CHECK(confidence BETWEEN 0.0 AND 1.0);

CREATE INDEX IF NOT EXISTS idx_graph_relationships_confidence
    ON graph_relationships(relationship_type, confidence);

-- Per-aggregate last-synced watermark. `source_aggregate` is one of the
-- six graph node sources: workspace, file, planner_report, execution,
-- memory_record, autonomous_session.
CREATE TABLE IF NOT EXISTS graph_sync_state (
    source_aggregate TEXT PRIMARY KEY,
    last_synced_at TEXT NOT NULL
);

-- Query/analytics cache. `cache_key` is a stable digest of the query
-- arguments (plus a scope prefix); the payload is the serialized result.
-- Freshness is enforced by the service layer (row TTL stored alongside).
CREATE TABLE IF NOT EXISTS graph_query_cache (
    cache_key TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL,
    ttl_seconds INTEGER NOT NULL
);