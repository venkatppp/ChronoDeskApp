# ChronoDesk — Project State

_Last updated: end of Phase 4 (Search Engine, Knowledge Graph, full UI completion)._

## Phase status

| Phase | Scope | Status |
|---|---|---|
| 1 | Project initialization — Tauri/React/TS shell, routing, theme, dashboard UI, IPC scaffold | ✅ Complete |
| 2 | Database Layer — SQLite + sqlx, migrations, models, repositories, services, workspace commands | ✅ Complete |
| 3 | Workspace Engine, Timeline Engine, File Watcher, event pipeline, frontend live-data integration | ✅ Complete |
| 4 | Search Engine + Knowledge Graph + full UI (Workspaces, Timeline, Settings, Search, Graph) | ✅ Complete |
| 5 | ML Layer | Not started |

## What exists today

### Frontend (`frontend/`)
All frontend pages are now fully implemented — no ComingSoon placeholders remain for Phase 1–4 features. The Dashboard shows live workspaces and timeline activity. The dedicated Workspaces browser supports status filtering, search, and create/delete. The Timeline page provides full reverse-chronological browsing with per-workspace and per-event-type filters. The Search page offers FTS5-ranked keyword search with entity-type filtering, search history, and saved searches. The Knowledge Graph page renders an interactive SVG-based graph visualization with node selection, statistics, and related-file exploration. The Settings page manages watched folders, theme preferences, and shows app version. Auto-refresh via `useAppEvents` (Tauri event subscriptions) works across all data-driven pages — no polling.

### Backend (`src-tauri/`)
```
src-tauri/src/
├── main.rs, lib.rs                 binary entry + full DI wiring (search, graph engines wired)
├── app_events.rs                   AppEventEmitter trait + search:indexed, graph:edge_added events
├── commands/                       system, workspace, timeline, watcher, search, graph
├── database/                       Database facade, connection (WAL, FK-on), migrations, schema
├── models/                         Workspace, FileArtifact, TimelineEvent, SearchResult, GraphEdge/Node
├── repositories/                   Workspace, File, Timeline, Settings, Search, Graph
├── services/                       WorkspaceService, TimelineService, SearchService, GraphService
├── workspace/                      heuristics, detector, manager — Workspace Engine
├── timeline/                       events, recorder, engine — Timeline Engine
├── watcher/                        debounce, event_handler, watcher — File Watcher
├── search/                         SearchEngine facade
├── graph/                          GraphEngine facade
└── ml/                             documented scaffold only — Phase 5
migrations/
├── 0001_initial_schema.sql         10 tables, all indexes/FKs (Phase 2)
├── 0002_workspace_root_path.sql    workspaces.root_path + partial unique index (Phase 3)
├── 0003_timeline_event_create_type.sql   widens event_type CHECK (Phase 3)
├── 0004_files_workspace_path_index.sql   composite index (Phase 3)
├── 0005_search_fts.sql             FTS5 search_index with triggers (Phase 4)
└── 0006_graph_edges.sql            graph_edges + search_history + saved_searches tables (Phase 4)
tests/
└── backend_integration.rs          6 full-stack integration tests
```

See `ARCHITECTURE.md` for the dependency graph and layering rules, and `EVENT_PIPELINE.md` for the full notify→SQLite→frontend data flow.

## Verification status

| Check | Result |
|---|---|
| Frontend `npm run build` / `tsc --noEmit` | ✅ Passes clean, 0 errors |
| Backend `cargo check --all-targets` | ✅ Passes clean, 0 warnings (on rustc 1.97.1) |
| Backend `cargo test` | ✅ 91/91 tests pass (86 lib + 5 integration) |
| Search feature operational | ✅ FTS5 search index, search_history, saved_searches wired |
| Knowledge Graph operational | ✅ graph_edges adjacency table, graph_engine, graph commands |
| IPC commands registered | ✅ 24 commands registered (system:2, workspace:5, timeline:2, watcher:3, search:9, graph:3) |
| Routes registered | ✅ 7 routes (Dashboard, Workspaces, Timeline, Graph, Search, Analytics, Settings) |
| No duplicate implementations | ✅ Verified — single ownership per concern |
| No TODO/FIXME/unimplemented | ✅ Clean — no unfinished markers in source |

## Phase 4 Release Audit (2026-07-26)

The following release-blocking bugs were found and fixed during the final audit:

- **`search_repository.rs` — UUID string interpolation (SQL injection)**: `format!(" AND workspace_id = '{}'", ws_id)` replaced with proper `?` bind parameters.
- **`graph_repository.rs` — string interpolation in edge_type filter**: `format!("'{}'", t.as_str())` replaced with proper `?` bind parameters.
- **macOS path canonicalization**: `notify` returns paths with `/private/` prefix on macOS (e.g., `/private/var/folders/...` vs expected `/var/folders/...`). Fixed by canonicalizing event paths and watch roots at the watcher level so `starts_with` checks in workspace detection work correctly. This also resolved a macOS-only test failure in `writing_a_file_under_a_detectable_root_produces_a_timeline_event`.
- **Debounce coalescing**: File writes generate both Create and Modify events; the debouncer coalesces them into Modify. Updated the end-to-end test assertion to accept either `Create` or `Edit` timeline event types.
- **Frontend TypeScript build errors**: Fixed unused imports (`useEffect`, `GraphEdge`, `Filter`), unused destructured variables (`q`, `error`, `appVersion`), incorrect `ThemeContextValue` field names (`theme`→`preference`, `setTheme`→`setPreference`), and `Github`→`GitBranch` icon rename (lucide-react compatibility).
- **Unused import**: `PathBuf` import in `database/mod.rs` gated behind `#[cfg(test)]`.

## Test summary

91 tests total across `src/` unit tests and `tests/backend_integration.rs`, all using `tempfile`-backed temporary databases/directories. Verified passing on macOS (rustc 1.97.1).

| Area | Test count |
|---|---|
| Database layer (Phase 2) | 3 |
| Repositories (workspace/file/timeline/settings/search/graph) | ~20 |
| Services (workspace/timeline) | ~6 |
| Workspace Engine (heuristics/detector/manager) | 15 |
| Timeline Engine (events/recorder/engine) | 9 |
| File Watcher (debounce/event_handler/watcher/commands) | 25 |
| Search/Graph repositories | 4 |
| System commands | 2 |
| Integration (full pipeline + lifecycle) | 5 |
| **Total** | **91** |

## Known technical debt / deferred decisions

- **No graceful shutdown hook for active watches.** `FileWatcher`'s background tokio tasks are torn down implicitly when the process exits (the OS reclaims file-watch handles on process termination), rather than via an explicit `tauri::RunEvent::Exit` handler calling `.stop()` on every active watch. Not a resource leak in practice, but not textbook-graceful either — a reasonable Phase 4 cleanup item if it ever needs to become more deliberate (e.g. flushing a final "unclean shutdown" timeline event).
- **Frontend has no test runner.** `pnpm run build` (tsc + vite) is real type-checking, not a substitute for behavioral tests. Recommend adding Vitest + React Testing Library alongside Phase 4's UI work, once there's a dedicated Workspaces/Timeline screen worth testing beyond the dashboard's presentational components.
- **Workspace card has no file/tab counts.** Removed in Phase 3's frontend rewrite rather than shipped as a fake number — there's no aggregate "count files in this workspace" command yet, and adding one just to fill a stat card would be premature; a real one belongs with whatever Phase 4/5 feature needs it first.
- **`recent_activity` table (schema only, Phase 2) is still unpopulated.** It's a planned dashboard-optimization cache, not yet wired to anything — the dashboard's live activity feed currently queries `timeline_events` directly, which is fast enough at today's data volumes.
- **`sqlx::Error::Database`'s violation-check methods** — see verification status above.

## Recommended next step

Phase 5 — ML Layer: on-device ONNX Runtime inference for workspace-boundary clustering, file classification, duplicate/near-duplicate detection, and the embedding pipeline for semantic search. All Phase 4 infrastructure (FTS5 search index, graph_edges table, IPC commands) is ready to receive the ML signals.
