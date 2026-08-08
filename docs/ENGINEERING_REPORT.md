# ChronoDesk Engineering Report

_Canonical consolidated engineering report. Synthesized from 19 interim milestone
reports (RC-2 through RC-10, 2026-08-02 → 2026-08-03), `ARCHITECTURE.md`, and
`PROJECT_STATE.md`. Where reports conflict, the conflict is noted in line or in
the "Conflicts between sources" note at the end._

## 1. Overview

ChronoDesk is a cross-platform desktop app that tracks a developer's workspace
activity: it watches folders, records a searchable timeline of workspace/file
activity, indexes everything with FTS5, and builds a knowledge graph of how
workspaces, files, plans, executions, memory, and sessions relate. Since RC-2 it
also hosts an agentic Copilot (AI provider settings, LLM tool calling,
goal-driven planning/execution, autonomous sessions) backed by a memory and
learning system, a production-grade typed knowledge graph, and production
hardening subsystems (performance, recovery, backup, security).

### Architecture summary

- **Strict layering** (enforced by convention): `commands` (thin IPC handlers)
  → `engines` (facades: `WorkspaceManager`, `TimelineEngine`, `GraphEngine`,
  `MemoryEngine`, `RecoveryManager`, …) → `services` → `repositories` (the only
  layer that writes SQL) → `database` → SQLite. `models` and `errors` are
  shared by every layer; `app_events` (`AppEventEmitter` trait) is the one
  deliberate cross-cutting exception.
- **Rules**: commands never run SQL or business logic (a "thin wrappers only"
  rule maintained across all 19 reports); repositories return typed models;
  services compose repositories; engines orchestrate services. No dependency
  cycles: `watcher` depends on `workspace`/`timeline`, never the reverse.
- **SQLite in WAL mode** with FK enforcement and versioned, additive
  migrations (`0001` → `0031`). Runtime-checked `sqlx::query` (no compile-time
  DB dependency); enum-bearing models use a private `*Row` struct +
  `TryFrom<Row>` conversion pattern.
- **No polling on the frontend**: live data flows through Tauri events
  (`useAppEvents`), later extended with typed streams (`execution:progress`,
  `autonomous:session`/`autonomous:reasoning`, `graph:updated`,
  `security:status`).

### Tech stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tauri, sqlx (SQLite, WAL), tokio, serde, uuid, chrono; `rayon` (RC-8 M4); `sysinfo` (RC-10 M1); keyring-backed `SecretStore` (in place by RC-10 M4) |
| Frontend | React + TypeScript, Vite, lucide-react, custom SVG graph visualization; Vitest + React Testing Library |
| Database | SQLite (WAL), FTS5 for search, versioned additive migrations |

## 2. Phase-by-phase engineering history

### Phase 1 — Shell & UI scaffold ✅ Complete

Tauri/React/TS project shell, routing, theming, dashboard UI, IPC scaffold.
(Per `PROJECT_STATE.md`; not covered by the RC milestone reports.)

### Phase 2 — Database layer ✅ Complete

SQLite + sqlx, migration runner, models, repositories, services, workspace
commands; migration `0001` (10 tables with indexes/FKs).

### Phase 3 — Workspace, timeline & watcher engines ✅ Complete

- Workspace Engine (`workspace/`): heuristics, detector, manager.
- Timeline Engine (`timeline/`): recorder, events, engine. Two enums by
  design: `TimelineEventType` (storage, matches DB CHECK) and
  `TimelineActivity` (domain vocabulary, many-to-one mapped exhaustively).
- File Watcher (`watcher/`): debounce, event_handler, watcher; three
  independent tokio tasks per watched root (OS watch/reconnect, intake, debounce
  + record) decoupled by mpsc channels.
- Event pipeline `notify → SQLite → frontend` (documented in
  `EVENT_PIPELINE.md`); live UI updates with no polling.
- Migrations `0002`–`0004` (workspace root path, timeline event-type CHECK
  widening, files/workspace-path index).

### Phase 4 — Search, knowledge graph & full UI ✅ Complete (release audit 2026-07-26)

- FTS5 search index with triggers (`0005`), `search_history` + `saved_searches`
  + `graph_edges` adjacency table (`0006`).
- All Phase 1–4 pages implemented (Dashboard, Workspaces, Timeline, Search,
  Graph, Analytics, Settings); 24 IPC commands, 7 routes.
- Release audit fixed: SQL-injection risk in `search_repository.rs` and
  `graph_repository.rs` (string interpolation → `?` bind parameters), macOS
  `/private/` path canonicalization in the watcher, debounce Create/Modify
  coalescing, frontend TypeScript build errors.

### Phase 5 — Learning / ML + agentic Copilot (RC-2 → RC-10)

`PROJECT_STATE.md` (written at end of Phase 4) marks Phase 5 as "Not started";
the RC-2–RC-10 reports record its delivery in six arcs:

1. **Copilot foundations (RC-2)** — AI settings UI (provider selection, live
   connection test, secure key input), the plan execution engine
   (async multi-step execution, pause/resume/cancel, progress, audit trail),
   and conversation management (rename/delete/pin/export JSON+Markdown).
   Migration `0017`; 10 new IPC commands (89 total).
2. **LLM-native tool calling (RC-4)** — provider-native `tool_calls` wire
   format, tool-schema advertising, a `ToolCallLoop` that iterates model →
   tool execution → feedback until a plain answer or the iteration cap, with
   permission policy and failure feedback returned to the model.
3. **Planning & durable execution (RC-5 M2–M4)** — goal → dependency-aware DAG
   `Planner` (bounded replanning) handing `ExecutionPlan`s to a DAG-scheduling
   `ExecutionEngine`; per-run in-memory `ExecutionContext` with `{{...}}`
   template binding (M3); durable checkpoints that survive app restart with
   resume/rollback semantics and zero re-runs (M4, migration `0018`).
4. **Live progress & autonomous runtime (RC-5 M5–M6)** — `execution:progress`
   streaming to a frontend Execution Dashboard with durable planner reports
   (M5, migration `0019`); an `AutonomousRuntime` with budgets, retry/timeout
   policies, approval checkpoints, and reasoning-event streaming (M6).
5. **Memory & learning (RC-6 M1–M4)** — execution-memory store, semantic
   retrieval, and a learning engine (M1, migration `0020`); production vector
   memory with n-gram embeddings, two-tier cache, k-NN index and a background
   indexer (M2, `0021`); adaptive learning (learned weights, confidence
   explanations, aging, duplicate merging, workflow families, failure
   patterns; M3, `0022`); memory lifecycle — retention policies, cleanup
   worker, compression, versioning/lineage, export/import, snapshots
   (M4, `0023`).
6. **Knowledge graph 2.0 (RC-8 M1–M4)** — typed node/relationship registry
   built from all six aggregates (M1, `0024`); live incremental sync, semantic
   `related_to` edges with confidence and decay, analytics (M2, `0025`);
   context intelligence (inference, workspace similarity, goal clusters,
   snapshots/timeline, fusion, explanations; M3, `0026`); scale/health —
   pagination, ranked + vector search, rayon parallel traversal, integrity
   checks/repair, benchmarks (M4, `0027`).
7. **Production hardening (RC-10 M1–M4)** — performance & profiling (M1,
   `0028`); reliability & recovery — checksummed journal, crash detection,
   watchdog, 0–100 health monitor, self-healing (M2, `0029`); data integrity &
   backup — `VACUUM INTO` snapshots, staged restores, integrity battery,
   maintenance (M3, `0030`); security hardening — 0–100 scoring across six
   categories, ledgers, recommendations (M4, `0031`).

## 3. Key subsystems: current state and evolution

### Workspace Engine

- Current state: complete (Phase 3) and stable — untouched by all RC reports.
- Evolution: Phase 3 `heuristics`/`detector`/`manager`; the workspace aggregate
  later became a source for the knowledge graph (`workspace` nodes + `contains`
  edges) and a target of `WorkspaceManager`-side detection. No workspace work
  appears in RC-2–RC-10 reports.

### Timeline Engine

- Current state: complete (Phase 3); `TimelineRecorder` records events from the
  watcher pipeline; the timeline feeds the Dashboard/Timeline pages.
- Evolution: unchanged by the RC reports (the RC-5+ `TimelinePage` file only
  reappears as a pre-existing lint-error file). Timeline semantics were
  exercised in the Phase 4 audit (Create vs Edit coalescing).

### File Watcher / event pipeline

- Current state: complete (Phase 3 + Phase 4 fixes); full flow documented in
  `EVENT_PIPELINE.md`.
- Evolution: per-root three-task decoupled pipeline (OS watch + reconnect,
  intake/normalize, debounce-drain + record); macOS canonicalization fix;
  debounce coalescing of Create+Modify. RC reports only touch the shared
  `AppEventEmitter` trait the pipeline emits through — the same infrastructure
  later carries `execution:progress`, `autonomous:*`, `graph:updated`, and
  `security:status` events.

### Search Engine (FTS5)

- Current state: complete (Phase 4) — FTS5 `search_index` with triggers
  (migration `0005`), `search_history` + `saved_searches` (`0006`),
  entity-type filtering, ranked keyword search; 9 search commands.
- Evolution: RC-2 lists "no conversation search/filter in UI" as deferred
  enhancement; RC-8 M4 adds ranked + vector-assisted search for graph nodes;
  the search frontend module (`searchRepository`) remains a lint-clean surface.
  Search is otherwise untouched by the RC reports.

### Knowledge Graph

- **Phase 4 legacy**: `graph_edges` co-occurrence adjacency table + `GraphEngine`
  (`graph:edge_added` events). Still present and untouched.
- **RC-8 M1 (foundation)**: typed `graph_nodes` (`(node_type, entity_id)` key)
  + `graph_relationships` (structural vocabulary: `contains`, `runs_in`,
  `reports_on`, `derived_from`; `related_to` reserved for computed edges),
  auto-constructed from six aggregates (workspaces, files, planner reports,
  executions, memory records, autonomous sessions) via idempotent upserts;
  BFS subgraph/path, node search, explainable context discovery; 7 IPC
  commands; interactive SVG visualization page.
- **RC-8 M2 (live)**: per-aggregate watermark incremental sync (5-min worker),
  semantic `related_to` edges via embeddings (threshold 0.45, capped at 500
  nodes) with per-edge confidence, exponential confidence decay
  (`0.92^age_days`, prune < 0.10; structural edges exempt), cached graph
  analytics (eigenvector centrality, components, workspace importance),
  multi-hop context + recommendations, `graph:updated` live events.
- **RC-8 M3 (context intelligence)**: per-signal confidence inference,
  workspace similarity + cross-workspace relationship discovery, goal
  clustering, knowledge summaries, persisted context snapshots with diffed
  timeline, memory+KG fusion, graph-assisted planner retrieval, and a
  why-are-these-related explanation engine (BFS path, shared-topic fallback).
- **RC-8 M4 (scale & health)**: paginated/virtualized loading, ranked search
  (title-prefix > title > summary) + vector search, rayon-parallel multi-root
  BFS, persisted integrity/maintenance/query-metric/benchmark ledgers, four
  integrity scans with repair (user-triggered), orphan cleanup, consistency
  probes, 8-benchmark suite, performance dashboard page.
- Evolution: co-occurrence inference (Phase 4) → deterministic structural
  construction → live self-maintaining layer → context intelligence → scale
  and self-healing. Computed/learned edges, incremental event-driven sync, and
  cross-workspace relatedness were explicitly deferred from M1 and delivered in
  M2/M3.

### Copilot: tool calling, planning & execution

- **Tool calling (RC-4)**: provider-native wire format (`LLMToolCall`,
  streaming delta reconstruction), `ToolCallLoop` (default 8 iterations) that
  executes only through the shared `ToolExecutor` + permission pipeline —
  denials and tool errors are fed back to the model, not fatal.
- **Planning (RC-5 M1–M3)**: `Planner` owns goal → DAG planning (dependencies
  + `PlanGate` conditional semantics) and bounded replanning; `ExecutionEngine`
  owns scheduling (`next_runnable_step_index`), lifecycle, cancellation,
  persistence, and progress. `ExecutionContext` resolves `{{steps.<name>.<path>}}`,
  `{{workspace.id}}`, `{{goal}}` templates before invocation; unresolved
  variables fail fast with a structured `PlannerError::UnresolvedVariable`.
- **Durability (RC-5 M4)**: per-step checkpoint rows (`plan_execution_checkpoints`)
  carrying plan + context + status; resume after restart never re-runs
  completed steps; terminal states delete checkpoints; `checkpoint_saved` /
  `checkpoint_loaded` event types.
- **Autonomous runtime (RC-5 M6)**: session budgets (steps/plans/replans/
  duration), retry + timeout policies, approval gates (Automatic/OnRisk/Manual),
  reasoning-event streaming, pause/resume/cancel/approve/reject; 8 IPC commands.

### Learning engine

- Current state: adaptive learning over execution memory (RC-6 M3), consumed
  only (never scheduling/planning): `Planner` reuses learned workflows
  (score ≥ 0.6, replay-counted), `AutonomousRuntime` consults avoid/failure
  signals; all advisory.
- Evolution: fixed-weight ranker (M1: 0.5·similarity + 0.3·fingerprint success +
  0.2·recency) → learned weights from success/replay/acceptance history (M3,
  bounded ±0.08 shifts, neutral until ≥3 records) with confidence scores and
  per-factor explanations, memory aging (30-day half-life, 0.3 archival weight
  past 180 days), identical-memory merging, workflow-family clustering,
  failure-pattern detection (repeated failures, unstable workflows,
  low-confidence plans), and learning-health stats over IPC.

### Predictive intelligence

- State: referenced but not covered by the RC reports. `predictive.ts` appears
  among pre-existing frontend lint errors (RC-8 M2/M3/M4) and a `predictive`
  startup stage exists in the RC-10 M1 startup profile; `PROJECT_STATE.md`
  scopes the Phase 5 ML layer (ONNX, clustering, classification, embedding
  pipeline for semantic search) as the recommended next step. The vector-memory
  (RC-6 M2) and graph-embedding surfaces (RC-8 M2–M4) are the embedding
  infrastructure it would build on.

### Runtime health

- Current state: RC-10 M2 delivers the fault-tolerance subsystem — append-only
  checksummed journal (SHA-256), crash detection + resume/rollback on launch,
  30-s watchdog for worker liveness (stalled/recovered transitions), a 0–100
  health monitor, and a self-healing service (restart monitoring, checkpoint
  verification, bounded journal pruning past 10k entries). Pre-existing
  `RuntimeHealthService` (referenced in RC-10 M1) supplies worker telemetry.
- Evolution: RC-10 M4 adds a parallel security monitor (0–100 score, six
  categories, non-fatal, `security:status` events). Watchdog monitoring is
  opt-in via `register_worker`/`heartbeat`.

### Memory

- Current state: managed execution memory with a production vector backend.
- Evolution: durable `execution_memory` store with best-effort captures at
  engine/runtime terminal states (M1) → real embeddings (character n-gram
  3–5 hashing, 384-dim, TF-weighted, L2-normalized), two-tier cache (512-entry
  LRU + durable SQLite), in-memory k-NN index warmed at startup, background
  indexer (150 ms debounce + 60 s sweep, chunks of 64, `update_goal_embedding`
  never re-pends) (M2) → adaptive learning weights/aging/merging/clustering
  (M3) → lifecycle management: retention policies (permanent/temporary/
  archived/expired), 15-min cleanup worker, compression of oversized histories
  (≥80 events or ≥150 steps, restorable), versioning with lineage edges,
  versioned JSON export/import, 6-hourly snapshots (pruned to 10) with restore
  (M4).

### Dashboard

- Current state: multi-page frontend with live event-driven updates, no
  polling. Pages added over time: Dashboard, Workspaces, Timeline, Search,
  Graph, Analytics, Settings (Phase 4) + Execution Dashboard (RC-5 M5), Memory
  page (RC-6 M1), rebuilt Knowledge Graph page with interactive SVG (RC-8 M1)
  + analytics/inspector (M2) + context panel (M3), Graph Performance page
  (RC-8 M4), Performance page (RC-10 M1), Recovery page (M2), Maintenance page
  (M3). Frontend test count grew 0 → 116 alongside.
- Evolution: presentational dashboard → per-subsystem operational dashboards
  (execution, autonomous, memory, graph, performance, recovery, maintenance),
  each with reconnect-on-reload snapshots + live streams.

## 4. Known limitations and deferred work (consolidated)

### Copilot / execution

- No rate limiting on LLM requests (RC-2; still open) and no LLM request cost
  tracking.
- No parallel step execution (serial DAG scheduler); no rollback of failed
  executions (steps idempotent by design) (RC-2).
- Export has no pagination (large conversations OOM risk); no execution-history
  retention policy for audit rows (RC-2).
- Tool-call rounds are not persisted as enriched `Message` rows — intermediate
  tool rounds are transient (RC-4); frontend does not render intermediate tool
  steps in the transcript (RC-4, RC-5 M2).
- `bind_plan_arguments` only binds workspace context; step-output →
  downstream-argument interpolation is a roadmap item (RC-5 M2 — partially
  addressed by RC-5 M3 `{{steps...}}` binding).
- No in-process rehydration of in-flight runs at app start; resume is
  on-demand (RC-5 M4, post-RC-5 item).
- Autonomous sessions are not persisted across restart (RC-5 M6); no
  multi-workspace session coordination; approval UX is basic (no diff view /
  risk badges); no session-outcome observability hooks (RC-5 M6).

### Memory / learning

- Embeddings use the deterministic local n-gram provider (a real model-backed
  embedder remains an option) (RC-6 M2).
- All learning signals are advisory — no enforcement beyond planner reuse
  scoring (RC-6 M3, by design).

### Knowledge graph

- `related_to` edges computed at discovery time only; cross-workspace
  relatedness persistence was deferred (RC-8 M1) and later delivered as
  context-intel workspace relations (RC-8 M3).
- Out of scope at RC-8 M4: periodic re-embedding maintenance jobs for changed
  nodes, per-relationship evidence in the UI, user-confirmed edges, graph
  export, cross-workspace analytics comparison views.

### Hardening

- Profiler samples new commands/engines/workers only — ~150 legacy command
  latencies are not captured (RC-10 M1). Thread counts are platform-limited
  (macOS reports 0) (RC-10 M1).
- Watchdog loop is not wired to the frontend (no live events; page reflects
  state on load/action) (RC-10 M2); worker monitoring is opt-in (RC-10 M2).
- Backups are not auto-rotated (retention/rotation deferred) (RC-10 M3);
  restores require an app relaunch (by design) (RC-10 M3).
- Security hardening has no frontend page yet (backend/IPC only; RC-10 M5
  surface work) (RC-10 M4); recommendations never auto-apply by default
  (RC-10 M4).

### Phase 1–4 leftovers (PROJECT_STATE.md)

- No graceful shutdown hook for active file watches (the RC-10 M2 `RunEvent::Exit`
  hook covers the recovery journal, not watcher task teardown).
- `recent_activity` table (schema-only, Phase 2) still unpopulated; the
  dashboard queries `timeline_events` directly.
- No workspace file/tab counts on the workspace card.

## 5. Milestone / release chronology

| Milestone | Date | Commit | One-line deliverable | Backend tests | Frontend tests |
|-----------|------|--------|----------------------|---------------|----------------|
| RC-2 | 2026-08-02 | `ee432c0` | AI settings UI; plan execution engine; conversation management (migration 0017) | 206 | — |
| RC-4 | 2026-08-02 | `9708c18` | LLM-native tool calling loop (wire format, schemas, feedback) | 234 | — |
| RC-5 M2 | 2026-08-02 | (M1: `7206a4a`) | Planner → ExecutionEngine handoff; DAG scheduler | 244 | — |
| RC-5 M3 | 2026-08-02 | (M2: `f3e02c0`) | Execution context + `{{...}}` variable binding | 256 | — |
| RC-5 M4 | — | — | Durable execution checkpoints; pause/resume across restart | 254¹ | — |
| RC-5 M5 | — | — | Live `execution:progress` streaming + Execution Dashboard | (not stated) | — |
| RC-5 M6 | 2026-08-02 | (pending) | Autonomous agent runtime (budgets, retries, approvals, reasoning stream) | 271 | 19 |
| RC-6 M1 | 2026-08-02 | — | Memory & learning system (execution memory, retrieval, learning) | 305 | 23 |
| RC-6 M2 | 2026-08-02 | — | Production vector memory (n-gram embeddings, k-NN, background indexer) | 346 | 25 |
| RC-6 M3 | 2026-08-02 | — | Adaptive learning (weights, confidence, aging, clustering, failures) | 382 | 31 |
| RC-6 M4 | 2026-08-02 | — | Memory lifecycle (retention, compression, versioning, snapshots) | 407 | 31 |
| RC-8 M1 | 2026-08-02 | — | Typed knowledge graph foundation (6 aggregates) | 419 | 36 |
| RC-8 M2 | 2026-08-03 | — | Live graph (watermark sync, semantic edges + confidence decay, analytics) | 432 | 39 |
| RC-8 M3 | 2026-08-03 | — | Context intelligence (inference, similarity, clusters, snapshots, explain) | 446 | 42 |
| RC-8 M4 | 2026-08-03 | — | Graph scale & health (pagination, vector search, rayon, integrity, benchmarks) | 468 | 49 |
| RC-10 M1 | — | — | Performance & profiling subsystem (profiler, benchmarks, optimizer) | 500 | 69 |
| RC-10 M2 | — | — | Reliability & recovery (journal, crash recovery, watchdog, self-healing) | 565 | 91 |
| RC-10 M3 | — | — | Data integrity & backup (snapshots, staged restore, integrity battery) | 594 | 116 |
| RC-10 M4 | — | — | Security hardening (0–100 scoring, ledgers, recommendations) | 638 | 116 |

¹ RC-5 M4 reports 254 backend tests versus M3's 256 — see conflicts below.

## 6. Verification practices summary

- **Six-gate validation** on every milestone: `cargo fmt --check`, `cargo clippy
  --all-targets -- -D warnings` (0 warnings), `cargo build`, `cargo test`,
  `cd frontend && npm run build` (Vite + tsc), `npx tsc -b` / `--noEmit`.
  From RC-5 M6 onward also `npm test` (Vitest) and, from RC-6 M1, `npm run lint`
  (no new errors; a fixed set of 13 pre-existing frontend lint errors /
  warnings reported consistently from RC-8 onward).
- **Backend tests**: pure-logic rules unit-tested without a database;
  repository tests against `tempfile`-backed SQLite; full-stack integration
  tests (`tests/backend_integration.rs`, grown to 6) against fully migrated
  databases; deterministic fake embedders (`alpha → [1,0]` etc.) for
  embedding-dependent services; flaky-test root causes hunted down (e.g. the
  millisecond-precision RFC3339 prune boundary in RC-10 M4).
- **Frontend tests**: Vitest + React Testing Library; per-service IPC contract
  tests, page wiring/action-flow tests, and component render/state tests;
  suites registered per-module (two were found unregistered and dead in RC-10
  M2 and fixed).
- **Migration safety**: every schema change is additive, forward-only, and
  version-gated (`CURRENT_SCHEMA_VERSION`); CHECK constraints widened via table
  rebuilds where SQLite requires it (0018); RFC3339 timestamp defaults used from
  migration `0026` onward (`datetime('now')` is not RFC3339).
- **Security review** is a standing section of each report: no new secrets, no
  widened permissions, no duplicated execution paths, best-effort
  audit/telemetry writes that never fail the primary operation.

## Conflicts between sources

- **PROJECT_STATE.md is stale relative to the RC reports**: it is dated "end of
  Phase 4" (audit 2026-07-26) and reports 24 IPC commands, 6 migrations, 91
  tests, and Phase 5 "Not started" — while RC-2 (2026-08-02) already reports 89
  IPC commands, 17 migrations, and 206 tests. Treat PROJECT_STATE as the Phase
  1–4 snapshot; the RC reports supersede it.
- **Test-count drift in RC-5**: M3 reports 256 backend tests (250 unit + 5
  integration + 1 doc); M4's report states "254 passed (was idem)" without a
  unit breakdown — a 2-test discrepancy not explained in either file.
- **RC-4 header labels its *previous* commit as "RC-4"**: "Previous Commit:
  `279dff0` (RC-4 — Persistent Tool Permission System)" — by series numbering
  this should be RC-3 (RC-2's header likewise cites RC-1 as its previous).
- **`CURRENT_SCHEMA_VERSION` fell out of sync**: RC-10 M3 found the constant
  stale at 22 while migrations had advanced through 28/29, and corrected it to
  30 (then 31 in M4). RC-6 M2's "bumped 19 → 21" also skips a bump for
  migration `0020` — the constant is informational and migrations are
  authoritative.
- **Dates**: RC-10 M1–M4 and RC-5 M4/M5 headers carry no dates; the chronology
  table marks these as "—".
