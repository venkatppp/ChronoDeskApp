-- Migration 0024: Knowledge Graph Foundation (RC-8 M1)
--
-- A typed node registry plus relationship table that together form the
-- knowledge graph. Nodes are constructed from six source aggregates —
-- workspaces, files, planner reports, executions, memory records, and
-- autonomous sessions — and edges link them with a small, fixed
-- relationship vocabulary. This is additive: the Phase 4 `graph_edges`
-- adjacency table (workspace/file co-occurrence) is untouched and keeps
-- serving the legacy graph commands.

-- Node registry: one row per entity in the knowledge graph. The natural
-- key is (node_type, entity_id) because each aggregate keys on its own
-- UUIDs, and an autonomous session's id lives in a different namespace
-- than a file's.
CREATE TABLE IF NOT EXISTS graph_nodes (
    node_type TEXT NOT NULL CHECK(node_type IN (
        'workspace',
        'file',
        'planner_report',
        'execution',
        'memory_record',
        'autonomous_session'
    )),
    entity_id TEXT NOT NULL,
    title TEXT NOT NULL,
    workspace_id TEXT,
    summary TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (node_type, entity_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_graph_nodes_workspace ON graph_nodes(workspace_id);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_title ON graph_nodes(title);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_type ON graph_nodes(node_type);

-- Relationships between graph nodes. A node of any type may connect to a
-- node of any other type; the relationship vocabulary is intentionally
-- small and structural (no computed co-occurrence edges yet):
--   contains    workspace -> file
--   runs_in     execution / memory_record / autonomous_session -> workspace
--   reports_on  planner_report -> execution
--   derived_from memory_record -> execution
--   related_to  computed ties (same workspace, shared goal) recorded by
--               context discovery when persisted
CREATE TABLE IF NOT EXISTS graph_relationships (
    id TEXT PRIMARY KEY NOT NULL,
    source_node_type TEXT NOT NULL,
    source_entity_id TEXT NOT NULL,
    target_node_type TEXT NOT NULL,
    target_entity_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL CHECK(relationship_type IN (
        'contains',
        'runs_in',
        'reports_on',
        'derived_from',
        'related_to'
    )),
    weight REAL NOT NULL DEFAULT 1.0 CHECK(weight BETWEEN 0.0 AND 1.0),
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_node_type, source_entity_id)
        REFERENCES graph_nodes(node_type, entity_id) ON DELETE CASCADE,
    FOREIGN KEY (target_node_type, target_entity_id)
        REFERENCES graph_nodes(node_type, entity_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_graph_relationships_unique
    ON graph_relationships(source_node_type, source_entity_id,
                           target_node_type, target_entity_id, relationship_type);
CREATE INDEX IF NOT EXISTS idx_graph_relationships_source
    ON graph_relationships(source_node_type, source_entity_id);
CREATE INDEX IF NOT EXISTS idx_graph_relationships_target
    ON graph_relationships(target_node_type, target_entity_id);
CREATE INDEX IF NOT EXISTS idx_graph_relationships_type
    ON graph_relationships(relationship_type);
