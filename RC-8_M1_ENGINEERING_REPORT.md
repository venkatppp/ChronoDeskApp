# RC-8 M1 Engineering Report — Knowledge Graph Foundation

**Date:** 2026-08-02
**Branch:** `main`

---

## Summary

RC-8 M1 establishes the Knowledge Graph as a first-class, typed
structural layer over everything ChronoDesk records. Where the Phase 4
`graph_edges` table inferred co-occurrence edges between workspaces and
files, the RC-8 knowledge graph is a **typed node registry**
(`graph_nodes`) plus a **relationship table** (`graph_relationships`)
that is **constructed automatically** from all six source aggregates —
workspaces, files, planner reports, executions, memory records, and
autonomous sessions — and then queried through **traversal, search, and
context relationship discovery** APIs.

The milestone ships:

- **Migration `0024`** — `graph_nodes` (keyed on `(node_type,
  entity_id)`, with `workspace_id` FK + cascade) and `graph_relationships`
  (structural vocabulary `contains / runs_in / reports_on /
  derived_from / related_to`, unique upsert key, FK cascade to nodes).
- **`models/kg.rs`** — `GraphNodeType`, `GraphRelationshipType`,
  `KgNode`, `KgEdge`, `KgSubgraph`, `GraphPath`, `ContextDiscovery`,
  `ContextHit`, `GraphSyncSummary`, `KgStats`, `GraphSource`.
- **`repositories/kg_repository.rs`** — every SQL statement behind the
  graph: node/relationship CRUD (idempotent `INSERT ... ON CONFLICT`
  upserts), source extraction for all six aggregates, structural link
  queries, BFS neighbor lookups, search, and statistics rollups.
- **`services/kg_service.rs`** — construction orchestration
  (`sync_graph`), BFS subgraph extraction, shortest-path search,
  node search, and context relationship discovery with explainable,
  ranked hits.
- **`graph/mod.rs`** — `GraphEngine` facade extended additively:
  `with_kg_service(...)` enables the RC-8 half while the legacy
  `GraphEngine::new` and all Phase 4 methods keep working untouched.
- **7 new thin IPC commands** — `graph_sync`, `graph_search`,
  `graph_subgraph`, `graph_path`, `graph_context`, `graph_kg_stats`,
  `graph_nodes`.
- **Frontend** — the Knowledge Graph page now renders the typed RC-8
  graph: entity-type filter chips with per-type counts, global graph
  search, one-click BFS exploration from any node, a "Rebuild" sync
  action, and a context discovery panel with per-hit reasons and
  weights, backed by a new interactive SVG visualization component.
- **Tests** — 10 backend unit tests (repository + service), 1
  full-stack integration test, and 5 frontend component tests.

The RC-8 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. The new
schema is fully additive (migration `0024`); the legacy `graph_edges`
graph and its commands are untouched; `GraphEngine::new` keeps working;
every SQL statement lives in the repository layer; business logic
(construction rules, BFS, similarity scoring) lives in the service
layer; IPC handlers are one-line delegations.

---

## Architecture

### What changed (additive only)

| Component | RC-8 M1 change |
|-----------|----------------|
| **`migrations/0024_knowledge_graph.sql` (new)** | `graph_nodes` + `graph_relationships` tables, indexes, cascade FKs, unique upsert keys |
| **`models/kg.rs` (new module)** | RC-8 graph models (`KgNode`, `KgEdge`, enums, view payloads) following the codebase `*Row` + `TryFrom` decoding pattern |
| **`repositories/kg_repository.rs` (new)** | all RC-8 SQL: node/relationship upserts, source extraction, structural links, BFS neighbors, search, stats |
| **`services/kg_service.rs` (new)** | `sync_graph` construction, `subgraph`, `find_path`, `search_nodes`, `discover_context`, `stats`, `list_by_type` |
| **`graph/mod.rs`** | `GraphEngine` gains an optional `KgService` via `with_kg_service`; RC-8 facade methods (`sync_graph`, `search_graph_nodes`, `graph_subgraph`, `graph_path`, `graph_context`, `graph_stats`, `graph_nodes`) |
| **`commands/graph.rs`** | 7 new thin commands (delegation only) |
| **`lib.rs`** | wires `KgRepository` → `KgService` → `GraphEngine::with_kg_service`; runs an idempotent `sync_graph` at startup so the Graph page is populated on first launch |
| **Frontend** | `types/graph.ts` gains the RC-8 types; `graphRepository.ts` gains the new IPC methods; new `KnowledgeGraphView.tsx` (force-directed SVG with pan/zoom/focus/search, per-type colors/icons, relationship colors); `GraphPage.tsx` rebuilt around the typed graph (filters, search, exploration, context panel, rebuild) |

### Construction data flow

```
workspaces ─┐
files ──────┤   ┌────────────────────────────┐
planner reports ─┤  KgService::sync_graph    │   graph_nodes
(reports keyed ──┤  (idempotent upserts)     ├──► 6 node kinds
on execution id) │  ┌────────────────────┐   │
executions ──────┼─►│ node upserts (6×)  │   │
memory records ──┤  └────────────────────┘   │
autonomous ──────┘  ┌────────────────────┐   ├──► graph_relationships
sessions            │ structural edges:  │   │    contains / runs_in /
                    │ contains, runs_in, │   │    reports_on / derived_from
                    │ reports_on,        │   └
                    │ derived_from       │
                    └────────────────────┘
```

Construction is a sequence of idempotent `INSERT ... ON CONFLICT DO
UPDATE` upserts: all six node kinds first (so every edge's endpoints
exist), then the structural edges. Edges whose endpoints vanished
mid-sync (e.g. a workspace deleted concurrently, cascading its nodes
away) are skipped with a debug log instead of failing the whole pass.
Re-running `sync_graph` is a no-op-change pass (all `updated_*` counters,
zero `created_*`).

### Node/edge model

| Node type | Identity | Title | Metadata |
|-----------|----------|-------|----------|
| `workspace` | workspace id | name | status |
| `file` | file id | file name (path as summary) | artifact_type |
| `planner_report` | execution id (report is per-execution) | "Planner Report \<id\>" | execution_id, created_at; report body (truncated) as summary |
| `execution` | execution id | plan goal (joined via `copilot_plans`) | status, started/completed at |
| `memory_record` | memory id | goal | kind, status |
| `autonomous_session` | session id (memory `source_id`) | "Session: \<goal\>" | status |

| Relationship | Direction | Source |
|--------------|-----------|--------|
| `contains` | workspace → file | `files.workspace_id` |
| `runs_in` | execution → workspace | `copilot_conversations.workspace_id` |
| `runs_in` | memory_record → workspace | `execution_memory.workspace_id` |
| `runs_in` | autonomous_session → workspace | `execution_memory.workspace_id` |
| `reports_on` | planner_report → execution | `plan_execution_reports.execution_id` |
| `derived_from` | memory_record → execution | `execution_memory.source_id` (kind `execution`) |

### Traversal, search & context discovery

- **`subgraph`** — BFS from a root node up to `depth` (default 2, cap 4),
  collecting nodes and the edges whose *both* endpoints were collected.
- **`find_path`** — unweighted BFS shortest path (default max depth 6,
  cap 10) with edge resolution along the walked path.
- **`search_nodes`** — case-insensitive `LIKE` over node titles and
  summaries, with optional node-type filtering.
- **`discover_context`** — ranked, explainable hits for one entity:
  1. entities sharing its workspace (files/executions at 0.8,
     memory/sessions at 0.6) — strongest ties;
  2. memory records anywhere whose goal token-Jaccard with the source
     title is ≥ 0.25 (cross-workspace learned context);
  3. nodes already connected by a persisted relationship.
  Hits are deduplicated (stronger evidence upgrades a weaker hit in
  place — e.g. a persisted `derived_from` edge upgrades a workspace
  co-member hit and attaches its relationship type), sorted by weight,
  capped at `limit` (default 100, cap 200).

### IPC (thin)

`graph_sync` / `graph_search` / `graph_subgraph` / `graph_path` /
`graph_context` / `graph_kg_stats` / `graph_nodes` — each a single
delegation to the engine. `lib.rs` runs `sync_graph` once at startup
and logs the summary.

---

## Frontend

The Knowledge Graph page (`pages/GraphPage.tsx`) now renders the typed
RC-8 graph:

- **Header stats** — node/edge totals plus the delta from the last
  `graph_sync`.
- **Entity filter chips** — All / Workspaces / Files / Planner Reports /
  Executions / Memory / Sessions, each with a live per-type count from
  `graph_kg_stats`; switching filters re-queries `graph_nodes`.
- **Global search** — `graph_search` with a dropdown of hits; selecting
  a hit explores its subgraph.
- **Interactive visualization** (`KnowledgeGraphView.tsx`) — custom
  SVG force-directed layout (repulsion/attraction/collision), pan,
  momentum scrolling, zoom, minimap-free but zoomable, node focus mode,
  keyboard search, hover-highlighted relationship curves, per-type
  node colors/icons and per-relationship edge colors, selection
  inspector card with relationship breakdown.
- **Exploration** — clicking a node runs `graph_subgraph` (depth 2) and
  `graph_context` in parallel; the view re-centers on the subgraph and
  the right-hand panel lists ranked context hits with reasons, weights,
  and relationship badges; clicking a hit navigates the graph further.
- **Rebuild** — `graph_sync` with a spinner and refreshed stats.

---

## Tests

### Backend (11 new)

| Area | Tests |
|------|-------|
| `KgRepository` | idempotent node upsert (created vs updated), list/search filtering, direction-agnostic neighbors, relationship round-trip, delete-cascade, source extraction covering all six aggregates, stats rollups |
| `KgService` | full `sync_graph` build (6 node kinds, 6 structural edges, idempotent re-run), subgraph extraction per node kind, shortest path (found + disconnected), context discovery ranking (workspace members, goal similarity, persisted relationship upgrade) |
| Integration (`backend_integration.rs`) | end-to-end sync against a fully migrated DB: nodes/edges from all six sources, traversal errors for unknown nodes, path absence, context discovery, idempotent re-sync |

### Frontend (5 new, 36 total)

`GraphPage.test.tsx` — stats + node loading on mount, entity-type
filtering, subgraph exploration + context panel, global search,
rebuild via `graph_sync`.

---

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt` / `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 419 tests (412 lib + 6 integration + 1 doc) |
| `npm run build` | ✅ |
| `npx tsc -b` | ✅ |
| `npm test` | ✅ 36 tests (6 files) |

Notes: `npm run lint` still reports 13 pre-existing errors in files
untouched by this milestone (`workspaceRepository.ts`, `predictive.ts`,
`TimelinePage.tsx`, `WorkspacesPage.tsx`, ...); this milestone's files
are lint-clean (the one hooks-order violation introduced in the new
visualization component was fixed during the gate run).

---

## Design notes & trade-offs

- **Typed registry over adjacency list.** The Phase 4 `graph_edges`
  table stays for the legacy workspace/file co-occurrence view. The RC-8
  graph is deliberately *structural* — edges are constructed from
  real foreign keys rather than inferred — which keeps construction
  deterministic and the vocabulary small. Computed/learned edges remain
  a future milestone's job.
- **`(node_type, entity_id)` as the node key.** Aggregates key on their
  own UUIDs, so the natural key is the pair — this is why `graph_nodes`
  uses a composite PK and relationships carry both columns.
- **Idempotent sync.** Construction is pure upsert, safe to run on every
  launch and from the UI; the `created_at == updated_at` trick in the
  `RETURNING *` row cleanly separates "created" from "updated" for
  reporting without a second query.
- **sqlx `Uuid` is a BLOB in SQLite.** All tests bind `Uuid` values
  directly (never `.to_string()`), matching how the rest of the
  codebase stores ids; a string-bound id silently violates the FK.
  Also, `substr(pe.id, 1, 8)` on a BLOB id returns a BLOB — the
  execution fallback title uses `substr(lower(hex(pe.id)), 1, 8)`
  instead.
- **FK violations during sync are tolerated.** A workspace deleted
  concurrently cascades its nodes; the corresponding edge upsert fails
  its FK and is skipped with a debug log rather than aborting the pass.

## Out of scope (later RC-8 milestones)

Semantic/embedding-based edges between arbitrary entities, incremental
event-driven sync (watcher/timeline hooks feeding the graph live),
path/centrality analytics on the graph, and cross-workspace relatedness
persistence (`related_to` edges are computed at discovery time only).
