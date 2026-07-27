-- Migration 0003: add 'create' to timeline_events.event_type
--
-- Phase 3's Timeline Engine distinguishes "a file was created" from "a
-- file was modified" (blueprint's explicit event list: File Created,
-- File Modified, File Deleted), but migration 0001's CHECK constraint on
-- `event_type` has no 'create' value.
--
-- SQLite has no `ALTER TABLE ... ALTER COLUMN` / `DROP CONSTRAINT`, so
-- widening a CHECK constraint requires the standard "recreate table"
-- pattern: build the new table shape, copy every existing row across,
-- drop the old table, rename the new one into place. No data is lost;
-- every existing row's `event_type` is already one of the still-valid
-- values, so the copy cannot violate the new (superset) constraint.

CREATE TABLE timeline_events_new (
    id              TEXT PRIMARY KEY NOT NULL,
    workspace_id    TEXT NOT NULL
                        REFERENCES workspaces (id) ON DELETE CASCADE,
    file_id         TEXT
                        REFERENCES files (id) ON DELETE SET NULL,
    event_type      TEXT NOT NULL
                        CHECK (event_type IN (
                            'create', 'open', 'close', 'edit', 'move', 'delete',
                            'commit', 'visit', 'screenshot', 'workspace_switch'
                        )),
    occurred_at     TEXT NOT NULL,
    metadata        TEXT,
    created_at      TEXT NOT NULL
);

INSERT INTO timeline_events_new (id, workspace_id, file_id, event_type, occurred_at, metadata, created_at)
SELECT id, workspace_id, file_id, event_type, occurred_at, metadata, created_at
FROM timeline_events;

DROP TABLE timeline_events;
ALTER TABLE timeline_events_new RENAME TO timeline_events;

CREATE INDEX idx_timeline_events_workspace_occurred
    ON timeline_events (workspace_id, occurred_at DESC);
CREATE INDEX idx_timeline_events_file_id ON timeline_events (file_id);
