//! Schema reference.
//!
//! The migration files under `migrations/` are the actual source of truth
//! for the schema's shape (columns, constraints, indexes) — this module
//! does not duplicate that SQL. It exists so that table names used
//! outside of query strings (logging, diagnostics, future admin/export
//! tooling) come from one constant rather than being re-typed as string
//! literals in multiple places.

/// The schema version this build of ChronoDesk expects, i.e. the number
/// of applied migrations. Bump this alongside adding a new
/// `NNNN_description.sql` file under `migrations/`.
///
/// History: 1 = initial schema (Phase 2). 2 = `workspaces.root_path`
/// (Phase 3, Workspace Engine). 3 = `timeline_events.event_type` gains
/// `'create'` (Phase 3, Timeline Engine). 4 = composite index on
/// `files (workspace_id, path_or_url)` (Phase 3, Timeline Recorder).
/// 5 = `search_index` FTS5 virtual table (Phase 4 Search Engine).
/// 6 = `graph_edges`, `search_history`, `saved_searches` tables
///     (Phase 4 Knowledge Graph).
/// 7 = unique index on `graph_edges` (6 identifying columns).
///     (Milestone 2 Backend Reliability).
/// 8 = `embeddings` table for Phase 5 ML Layer vector storage.
/// 17 = plan execution tables and conversation management metadata.
/// 18 = `plan_execution_checkpoints` table and checkpoint lifecycle
///     event types (durable pause/resume).
/// 19 = `plan_execution_reports` table (execution progress streaming;
///     durable planner reports).
/// 20 = `execution_memory` table (RC-6 M1 execution memory store).
/// 21 = `memory_vector_index` + `memory_embedding_cache` tables
///     (RC-6 M2 production vector memory system).
/// 22 = `memory_acceptance` table (RC-6 M3 adaptive learning: the
///     recommendation acceptance ledger that adapts weights + confidence).
/// 28 = performance profiling tables (RC-10 M1 — `performance_profiles`,
///     `benchmark_runs`, `startup_profiles`).
/// 29 = reliability & recovery ledger (RC-10 M2 — `recovery_journal`,
///     `crash_reports`, `worker_health`, `recovery_history`).
/// 30 = data integrity & backup ledger (RC-10 M3 — `backup_runs`).
/// 31 = security hardening ledgers (RC-10 M4 — `security_audit_log`,
///     `security_config`, `security_findings`, `security_recommendations`).
pub const CURRENT_SCHEMA_VERSION: i64 = 31;

/// Table name constants. Repository query strings use literal SQL for
/// readability (and because `sqlx::query_as` takes a plain `&str`, not a
/// query builder), but anything that needs to *refer* to a table name in
/// Rust — logs, diagnostics, future export tooling — should use these
/// instead of re-typing the string.
pub mod tables {
    pub const WORKSPACES: &str = "workspaces";
    pub const FILES: &str = "files";
    pub const TIMELINE_EVENTS: &str = "timeline_events";
    pub const TAGS: &str = "tags";
    pub const WORKSPACE_TAGS: &str = "workspace_tags";
    pub const WORKSPACE_RELATIONSHIPS: &str = "workspace_relationships";
    pub const SETTINGS: &str = "settings";
    pub const SEARCH_INDEX_METADATA: &str = "search_index_metadata";
    pub const ML_METADATA: &str = "ml_metadata";
    pub const RECENT_ACTIVITY: &str = "recent_activity";
    pub const GRAPH_EDGES: &str = "graph_edges";
    pub const SEARCH_HISTORY: &str = "search_history";
    pub const SAVED_SEARCHES: &str = "saved_searches";
    pub const EMBEDDINGS: &str = "embeddings";
}
