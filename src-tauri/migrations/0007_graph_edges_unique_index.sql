-- Migration 0007: Unique constraint on graph_edges identifying columns.
--
-- The old upsert_edge used a SELECT-then-INSERT/UPDATE pattern without a
-- transaction or unique constraint, so existing databases may contain
-- duplicate rows for the same logical edge. Step 1 removes those duplicates
-- before Step 2 creates the unique index.

-- Step 1: Remove duplicate rows, keeping only the one with the most recent
-- updated_at (then lowest rowid for ties) per set of identifying columns.
DELETE FROM graph_edges WHERE id NOT IN (
    SELECT id FROM (
        SELECT id,
            ROW_NUMBER() OVER (
                PARTITION BY source_entity_type, source_entity_id,
                             target_entity_type, target_entity_id,
                             edge_type, workspace_id
                ORDER BY updated_at DESC, rowid ASC
            ) AS rn
        FROM graph_edges
    ) WHERE rn = 1
);

-- Step 2: Create the unique index (safe now that duplicates are removed).
CREATE UNIQUE INDEX idx_graph_edges_unique_edge
    ON graph_edges(source_entity_type, source_entity_id, target_entity_type, target_entity_id, edge_type, workspace_id);
