-- Migration 0001: initial schema
--
-- Establishes every table ChronoDesk's Phase 2 Database Layer needs.
-- Design notes (apply to the whole file):
--   * Primary keys are UUIDv4 stored as TEXT. SQLite has no native UUID
--     type; TEXT keeps values human-readable in `sqlite3` and avoids the
--     BLOB-vs-TEXT ambiguity that comes with sqlx's default UUID mapping.
--   * Timestamps are TEXT in RFC 3339 (e.g. `2026-07-19T10:00:00Z`),
--     decoded on the Rust side as `chrono::DateTime<Utc>`. SQLite has no
--     native datetime type either; RFC 3339 sorts correctly as a string
--     and is what `chrono`'s sqlx integration expects.
--   * Every mutable table has both `created_at` and `updated_at`.
--   * Foreign keys use `ON DELETE CASCADE` where a child row is
--     meaningless without its parent (files, timeline events, tags-join,
--     search/ML metadata, recent activity) and `ON DELETE SET NULL` where
--     the parent is optional context (timeline_events.file_id).
--   * `PRAGMA foreign_keys = ON` is NOT set here — SQLite requires it to
--     be set per-connection, so it lives in `database/connection.rs`
--     instead of the migration.

-- ---------------------------------------------------------------------
-- workspaces
-- The top-level organizing unit (blueprint §1.2, §7.2).
-- ---------------------------------------------------------------------
CREATE TABLE workspaces (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT,
    status          TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'archived')),
    health_score    REAL NOT NULL DEFAULT 0.0
                        CHECK (health_score >= 0.0 AND health_score <= 100.0),
    last_active_at  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_workspaces_status ON workspaces (status);
CREATE INDEX idx_workspaces_last_active_at ON workspaces (last_active_at DESC);

-- ---------------------------------------------------------------------
-- files
-- Any artifact belonging to a workspace: file, browser tab, note, git
-- commit, screenshot, or terminal session (blueprint §2.1, §7.2 `artifacts`).
-- ---------------------------------------------------------------------
CREATE TABLE files (
    id              TEXT PRIMARY KEY NOT NULL,
    workspace_id    TEXT NOT NULL
                        REFERENCES workspaces (id) ON DELETE CASCADE,
    artifact_type   TEXT NOT NULL
                        CHECK (artifact_type IN (
                            'file', 'tab', 'note', 'commit',
                            'screenshot', 'terminal_session'
                        )),
    path_or_url     TEXT NOT NULL,
    content_hash    TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX idx_files_workspace_id ON files (workspace_id);
CREATE INDEX idx_files_content_hash ON files (content_hash);

-- ---------------------------------------------------------------------
-- timeline_events
-- Append-only activity log (blueprint §10). Never updated after insert;
-- there is deliberately no `updated_at` column.
-- ---------------------------------------------------------------------
CREATE TABLE timeline_events (
    id              TEXT PRIMARY KEY NOT NULL,
    workspace_id    TEXT NOT NULL
                        REFERENCES workspaces (id) ON DELETE CASCADE,
    file_id         TEXT
                        REFERENCES files (id) ON DELETE SET NULL,
    event_type      TEXT NOT NULL
                        CHECK (event_type IN (
                            'open', 'close', 'edit', 'move', 'delete',
                            'commit', 'visit', 'screenshot', 'workspace_switch'
                        )),
    occurred_at     TEXT NOT NULL,
    metadata        TEXT,  -- free-form JSON payload, event-type specific
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_timeline_events_workspace_occurred
    ON timeline_events (workspace_id, occurred_at DESC);
CREATE INDEX idx_timeline_events_file_id ON timeline_events (file_id);

-- ---------------------------------------------------------------------
-- tags + workspace_tags
-- Free-form labeling, many-to-many between tags and workspaces.
-- ---------------------------------------------------------------------
CREATE TABLE tags (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL UNIQUE,
    color           TEXT,
    created_at      TEXT NOT NULL
);

CREATE TABLE workspace_tags (
    workspace_id    TEXT NOT NULL
                        REFERENCES workspaces (id) ON DELETE CASCADE,
    tag_id          TEXT NOT NULL
                        REFERENCES tags (id) ON DELETE CASCADE,
    created_at      TEXT NOT NULL,
    PRIMARY KEY (workspace_id, tag_id)
);

CREATE INDEX idx_workspace_tags_tag_id ON workspace_tags (tag_id);

-- ---------------------------------------------------------------------
-- workspace_relationships
-- Directed edges between workspaces themselves (distinct from the
-- artifact-level knowledge graph in blueprint §8, which ships in Phase 4).
-- Useful today for "related workspace" / "derived from" links a user or
-- the recommendation engine draws between whole projects.
-- ---------------------------------------------------------------------
CREATE TABLE workspace_relationships (
    id                      TEXT PRIMARY KEY NOT NULL,
    source_workspace_id     TEXT NOT NULL
                                REFERENCES workspaces (id) ON DELETE CASCADE,
    target_workspace_id     TEXT NOT NULL
                                REFERENCES workspaces (id) ON DELETE CASCADE,
    relationship_type       TEXT NOT NULL
                                CHECK (relationship_type IN (
                                    'related', 'derived_from', 'blocks', 'duplicate_of'
                                )),
    created_at              TEXT NOT NULL,
    UNIQUE (source_workspace_id, target_workspace_id, relationship_type),
    CHECK (source_workspace_id <> target_workspace_id)
);

CREATE INDEX idx_workspace_relationships_source ON workspace_relationships (source_workspace_id);
CREATE INDEX idx_workspace_relationships_target ON workspace_relationships (target_workspace_id);

-- ---------------------------------------------------------------------
-- settings
-- Simple application-wide key/value store (blueprint §3.2 Settings screen).
-- Values are JSON-encoded so a single table can hold heterogeneous
-- preference shapes without a migration per new setting.
-- ---------------------------------------------------------------------
CREATE TABLE settings (
    key             TEXT PRIMARY KEY NOT NULL,
    value           TEXT NOT NULL,  -- JSON-encoded
    updated_at      TEXT NOT NULL
);

-- ---------------------------------------------------------------------
-- search_index_metadata
-- Tracks the Search Engine's (Phase 4) indexing state per file, so a
-- crash or version bump can cheaply determine what needs re-indexing
-- without re-scanning file contents.
-- ---------------------------------------------------------------------
CREATE TABLE search_index_metadata (
    id              TEXT PRIMARY KEY NOT NULL,
    file_id         TEXT NOT NULL UNIQUE
                        REFERENCES files (id) ON DELETE CASCADE,
    index_version   INTEGER NOT NULL DEFAULT 1,
    checksum        TEXT,
    indexed_at      TEXT NOT NULL
);

-- ---------------------------------------------------------------------
-- ml_metadata
-- Placeholder table for the ML Layer (Phase 5): classification labels,
-- embedding references, and confidence scores per file. Created now so
-- the schema is stable and Phase 5 only ever inserts into it.
-- ---------------------------------------------------------------------
CREATE TABLE ml_metadata (
    id              TEXT PRIMARY KEY NOT NULL,
    file_id         TEXT NOT NULL
                        REFERENCES files (id) ON DELETE CASCADE,
    model_version   TEXT NOT NULL,
    embedding_id    TEXT,
    classification  TEXT,
    confidence      REAL CHECK (confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
    created_at      TEXT NOT NULL,
    UNIQUE (file_id, model_version)
);

CREATE INDEX idx_ml_metadata_file_id ON ml_metadata (file_id);

-- ---------------------------------------------------------------------
-- recent_activity
-- Denormalized, per-workspace rolling feed optimized for the dashboard's
-- "Today's briefing" / recommendations panel (blueprint §3.2), so those
-- screens don't have to aggregate the full `timeline_events` log on every
-- render. Populated by the service layer alongside timeline inserts.
-- ---------------------------------------------------------------------
CREATE TABLE recent_activity (
    id              TEXT PRIMARY KEY NOT NULL,
    workspace_id    TEXT NOT NULL
                        REFERENCES workspaces (id) ON DELETE CASCADE,
    activity_type   TEXT NOT NULL,
    summary         TEXT NOT NULL,
    occurred_at     TEXT NOT NULL,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_recent_activity_workspace_occurred
    ON recent_activity (workspace_id, occurred_at DESC);
