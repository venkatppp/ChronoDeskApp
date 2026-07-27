# ChronoDesk Roadmap

This tracks scope at the phase level. For current implementation status,
file-by-file, see `PROJECT_STATE.md`. For system design, see
`ARCHITECTURE.md` and `EVENT_PIPELINE.md`.

---

## Phase 1 — Foundation ✅

Tauri 2 + React 19 + TypeScript shell. Tailwind v4 dark-mode-first design
system, routing, sidebar/topbar layout, dashboard UI against mock data,
minimal Rust command scaffold proving the IPC path works end to end.

## Phase 2 — Database Layer ✅

SQLite + sqlx: connection pool (WAL mode, foreign keys on), versioned
migrations, typed models, the Repository Pattern for `workspaces`,
`files`, and `timeline_events`, `WorkspaceService`, and the first five
real `commands::workspace::*` IPC commands.

## Phase 3 — Workspace Engine, Timeline Engine, File Watcher ✅

- **Workspace Engine**: heuristic-based boundary detection (git repo,
  language manifests, README) finding/creating the workspace a file
  belongs to; `workspaces.root_path` added to the schema.
- **Timeline Engine**: a domain vocabulary (`TimelineActivity`) mapped
  onto the storage-level `TimelineEventType`, automatic `files` row
  resolution on record, `create` added to the timeline event vocabulary.
- **File Watcher**: `notify`-based recursive watching, per-path
  debouncing, ignore rules (`.git`, `node_modules`, `target`, OS/editor
  temp files), automatic reconnect on watch failure, persisted watch
  paths restored on launch.
- **Event pipeline**: notify → debounce → normalize → workspace
  detection → timeline recording → SQLite → `AppEventEmitter` → frontend,
  fully wired with no manual refresh required.
- **Frontend**: mock repository removed entirely; live workspaces, live
  recent activity, working create flow, error handling, auto-refresh via
  Tauri events.

## Phase 4 — Search Engine + Knowledge Graph (proposed)

See the Phase 4 Design Proposal for full detail. Summary: Tantivy
keyword + embedded vector (HNSW) hybrid search; a `graph_edges`
adjacency-table Knowledge Graph with co-occurrence/semantic-similarity
edge inference; the dedicated Workspaces and Timeline screens (full
browsing, filtering, and a React Flow graph visualization) the Phase 3
placeholders are still waiting on; a Settings screen for managing
watched folders through the UI instead of only via IPC commands.

## Phase 5 — ML Layer (not started)

On-device ONNX Runtime inference: workspace-boundary clustering (as a
second signal alongside — not a replacement for — Phase 3's heuristics),
file classification, duplicate/near-duplicate detection, and the
embedding pipeline Phase 4's semantic search depends on.

## Startup roadmap (post-MVP)

Team workspaces, RBAC, plugin marketplace, Enterprise admin dashboard,
compliance mode — per the original product blueprint's §14.

## Enterprise roadmap (post-MVP)

SSO/SAML, org-wide analytics, Neo4j-backed shared knowledge graph,
on-prem deployment — per the original product blueprint's §14.
