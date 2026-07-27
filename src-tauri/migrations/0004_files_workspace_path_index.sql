-- Migration 0004: composite index for file lookup by workspace + path
--
-- `TimelineRecorder` (Phase 3) resolves a `files` row for every
-- file-level timeline activity via
-- `FileRepository::find_by_workspace_and_path`, which runs on every
-- watcher event — a query pattern migration 0001's indexes
-- (`workspace_id` alone, `content_hash` alone) don't cover efficiently.
-- A composite index on exactly the two columns that query filters on
-- keeps that lookup an index seek instead of a per-workspace scan as a
-- workspace's file count grows.

CREATE INDEX idx_files_workspace_id_path ON files (workspace_id, path_or_url);
