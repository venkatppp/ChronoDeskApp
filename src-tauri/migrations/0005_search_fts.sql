-- Migration 0005: full-text search index (Phase 4 Search Engine).
--
-- `search_index` is a standalone (not "external content") FTS5 virtual
-- table: `entity_type`/`entity_id`/`workspace_id` are UNINDEXED metadata
-- columns (never matched against, only returned/filtered), `title`/
-- `body` are the indexed full-text columns. Kept in sync with
-- `workspaces` and `files` entirely by the triggers below — no Rust
-- code ever writes to this table directly (see
-- `repositories::search_repository::SearchRepository`, which only ever
-- SELECTs from it).
--
-- A standalone (non-"content=") table is chosen over an external-content
-- table deliberately: it duplicates title/body text into the FTS index,
-- but needs no `rowid` bookkeeping to stay aligned with the source
-- tables, and each entity's row is simply replaced (DELETE + INSERT) on
-- every UPDATE rather than requiring the more fragile external-content
-- `UPDATE ... SET` trigger dance.
--
-- SQLite fires AFTER DELETE triggers for rows removed via a cascading
-- foreign key (e.g. deleting a workspace cascades to its `files` rows)
-- exactly as it would for an explicit `DELETE`, so the `files` triggers
-- below also keep `search_index` clean when a whole workspace is deleted
-- — no separate cleanup path is needed for cascades.
CREATE VIRTUAL TABLE search_index USING fts5(
    entity_type UNINDEXED,
    entity_id UNINDEXED,
    workspace_id UNINDEXED,
    title,
    body
);

-- ---------------------------------------------------------------------
-- workspaces --> search_index
-- ---------------------------------------------------------------------
CREATE TRIGGER search_index_workspaces_ai AFTER INSERT ON workspaces BEGIN
    INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
    VALUES ('workspace', NEW.id, NEW.id, NEW.name, COALESCE(NEW.description, ''));
END;

CREATE TRIGGER search_index_workspaces_au AFTER UPDATE ON workspaces BEGIN
    DELETE FROM search_index WHERE entity_type = 'workspace' AND entity_id = OLD.id;
    INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
    VALUES ('workspace', NEW.id, NEW.id, NEW.name, COALESCE(NEW.description, ''));
END;

CREATE TRIGGER search_index_workspaces_ad AFTER DELETE ON workspaces BEGIN
    DELETE FROM search_index WHERE entity_type = 'workspace' AND entity_id = OLD.id;
END;

-- ---------------------------------------------------------------------
-- files --> search_index
-- ---------------------------------------------------------------------
CREATE TRIGGER search_index_files_ai AFTER INSERT ON files BEGIN
    INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
    VALUES ('file', NEW.id, NEW.workspace_id, NEW.path_or_url, '');
END;

CREATE TRIGGER search_index_files_au AFTER UPDATE ON files BEGIN
    DELETE FROM search_index WHERE entity_type = 'file' AND entity_id = OLD.id;
    INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
    VALUES ('file', NEW.id, NEW.workspace_id, NEW.path_or_url, '');
END;

CREATE TRIGGER search_index_files_ad AFTER DELETE ON files BEGIN
    DELETE FROM search_index WHERE entity_type = 'file' AND entity_id = OLD.id;
END;

-- Backfill: index every workspace/file that already existed before this
-- migration ran. A freshly created database has none (the tables were
-- just created in migration 0001), but this keeps the migration correct
-- and idempotent-in-spirit for a hypothetical pre-populated database.
INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
SELECT 'workspace', id, id, name, COALESCE(description, '') FROM workspaces;

INSERT INTO search_index (entity_type, entity_id, workspace_id, title, body)
SELECT 'file', id, workspace_id, path_or_url, '' FROM files;
