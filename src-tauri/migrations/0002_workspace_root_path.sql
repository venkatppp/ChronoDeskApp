-- Migration 0002: workspace filesystem root
--
-- Phase 3's Workspace Engine detects workspace boundaries from the file
-- system (a git repo root, a directory containing package.json, etc.) and
-- needs to answer "does a workspace already exist for this directory?"
-- without scanning every row's files. `root_path` is that lookup key.
--
-- Nullable: a workspace created manually from the UI (no filesystem
-- association) is still valid — `root_path` is only ever set for
-- workspaces the detector created or matched.
--
-- A plain `UNIQUE` constraint can't be used here because SQLite treats
-- multiple `NULL`s in a `UNIQUE` column as distinct (which is what we
-- want — many manually-created workspaces with no root_path must all be
-- allowed), so a *partial* unique index expresses "unique only when
-- non-null" explicitly.

ALTER TABLE workspaces ADD COLUMN root_path TEXT;

CREATE UNIQUE INDEX idx_workspaces_root_path
    ON workspaces (root_path)
    WHERE root_path IS NOT NULL;
