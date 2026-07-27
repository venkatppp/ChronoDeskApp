-- Migration 0006: Knowledge Graph edges and Search metadata.

CREATE TABLE graph_edges (
    id TEXT PRIMARY KEY,
    source_entity_type TEXT NOT NULL,
    source_entity_id TEXT NOT NULL,
    target_entity_type TEXT NOT NULL,
    target_entity_id TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK(edge_type IN ('co_occurrence', 'semantic_similarity', 'explicit_reference', 'derivation')),
    weight REAL NOT NULL CHECK(weight BETWEEN 0.0 AND 1.0),
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    metadata TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_graph_edges_workspace_id ON graph_edges(workspace_id);
CREATE INDEX idx_graph_edges_source ON graph_edges(source_entity_id, source_entity_type);
CREATE INDEX idx_graph_edges_target ON graph_edges(target_entity_id, target_entity_type);

-- Search history for auto-suggestions.
CREATE TABLE search_history (
    query TEXT PRIMARY KEY,
    last_searched_at TEXT NOT NULL
);

-- Saved searches.
CREATE TABLE saved_searches (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    created_at TEXT NOT NULL
);
