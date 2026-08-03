# RC-8 M3 Engineering Report — Context Intelligence

**Date:** 2026-08-03
**Branch:** `main`

---

## Summary

RC-8 M3 adds a **context intelligence layer** on top of the M1 knowledge
graph and M2 live graph: an inference engine that ranks an entity's
neighbors with a per-signal confidence breakdown, graph-derived workspace
similarity and cross-workspace relationship discovery, goal-similarity
clustering, knowledge summaries, persisted graph-context snapshots with a
diffed timeline, memory + KG context fusion with graph-assisted planner
retrieval, and a why-are-these-related explanation engine.

The milestone ships:

- **Migration `0026`** — three additive tables: persisted cross-workspace
  relationships (`context_intel_workspace_relations`), graph context
  snapshots (`context_intel_snapshots`), and goal clusters
  (`context_intel_clusters`). All timestamp defaults use RFC3339
  `strftime('%Y-%m-%dT%H:%M:%fZ','now')` — `datetime('now')` is not RFC3339.
- **`models/kg_context.rs`** — the M3 DTO surface: `ContextSignalType`,
  `ConfidenceBreakdown`, `ContextHit`, `ContextInference`, `SignalEvidence`,
  `WorkspaceSimilarity(Result)`, `ClusterMember`, `GoalCluster`,
  `SummaryPoint`, `KnowledgeSummary`, `ContextIntelSnapshot`,
  `ContextTimelineEntry`, `FusedHit(Source)`, `FusedContext`, `PlannerContext`,
  `ExplanationLink`, `ContextExplanation`.
- **`repositories/context_intel_repository.rs`** — all M3 SQL: similarity
  upsert/list (both directions of an unordered pair), snapshot insert/list,
  cluster replace/list, with RFC3339 timestamps written explicitly so
  already-migrated databases parse correctly.
- **`services/context_intel_service.rs`** — the M3 business logic: cache-backed
  `infer_context`, `workspace_similarity` /
  `discover_cross_workspace_relationships`, `goal_clusters`,
  `knowledge_summary`, `context_snapshot_create/list` + `context_timeline`,
  `fused_context`, `planner_context`, and `explain` (BFS shortest path with a
  shared-vocabulary fallback). All scoring/similarity/clustering policy lives
  here; all SQL lives in the repository; results share the M2 query cache.
- **`graph/mod.rs`** — `GraphEngine` gains an optional `ContextIntelService`
  via `with_context_intel_service` plus 11 additive facade methods.
- **11 new thin IPC commands** — `graph_infer_context`,
  `graph_workspace_similarity`, `graph_discover_cross_workspace_relationships`,
  `graph_goal_clusters`, `graph_knowledge_summary`, `graph_snapshot_create`,
  `graph_snapshot_list`, `graph_context_timeline`, `graph_fused_context`,
  `graph_planner_context`, `graph_explain`.
- **Runtime wiring in `lib.rs`** — the context intelligence service is
  constructed with the real `MemoryVectorSystem` as its embedder (same
  embedding surface M2 uses for semantic edges) and attached to the engine.
- **Frontend** — `types/contextIntel.ts` (all M3 DTOs, kept separate from
  `types/graph.ts` because the M3 `ContextHit` shape differs from the M1 type),
  11 new repository methods, and a `ContextIntelPanel` in the graph inspector
  showing the knowledge summary, confidence breakdown, top inferred hits, and —
  for workspace nodes — related workspaces (with recompute), goal clusters,
  and context snapshots (with capture).
- **Tests** — 14 new backend tests (4 repository + 10 service) and 3 new
  frontend component tests. The remaining M3 surfaces (`fused_context`,
  `planner_context`, `explain`, `context_timeline`, `graph_planner_context`)
  are covered backend-side; they are planner/service-facing rather than
  inspector-facing.

The RC-8 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. The schema is
additive (migration `0026`); `GraphEngine::new` and every M1/M2 command and
method keep working; all SQL stays in the repository layer; all scoring stays
in the service layer; IPC handlers are one-line delegations.

---

## Architecture

### What changed (additive only)

| Component | RC-8 M3 change |
|-----------|----------------|
| **`migrations/0026_context_intelligence.sql` (new)** | `context_intel_workspace_relations`, `context_intel_snapshots`, `context_intel_clusters`; RFC3339 `strftime` timestamp defaults |
| **`models/kg_context.rs` (new)** | all M3 DTOs (inference, similarity, clusters, summaries, snapshots, fusion, planner, explanations) |
| **`repositories/context_intel_repository.rs` (new)** | similarity upsert (canonical direction) + two-sided list with similarity floor, snapshot insert/list (newest-first), cluster replace-on-write/list per scope |
| **`services/context_intel_service.rs` (new)** | `infer_context`, `workspace_similarity`/`discover`, `goal_clusters`, `knowledge_summary`, snapshots + timeline, `fused_context`, `planner_context`, `explain`; `build_breakdown`/`combine_signals`/`cluster_goals`/`shortest_path` policies |
| **`graph/mod.rs`** | `GraphEngine` gains `Option<ContextIntelService>` via `with_context_intel_service`; 11 additive facade methods |
| **`commands/graph.rs`** | 12 new thin commands (delegation only) |
| **`lib.rs`** | wires `ContextIntelRepository` → `ContextIntelService` (with the memory system's embedder) → engine |
| **`services/kg_live_service.rs`** | two read accessors (`graph_nodes`/`graph_edges`) exposing the whole graph to context scoring |
| **Frontend** | `types/contextIntel.ts` DTOs; `graphRepository.ts` + 10 methods; `features/graph/ContextIntelPanel.tsx` (new); `GraphPage.tsx` mounts the panel in the inspector; `GraphPage.test.tsx` + 3 tests |

### How the surfaces compose

All services sit strictly behind the M2 `graph_query_cache`:
`cached_put` writes a short-TTL (60 s) JSON payload keyed by the call's
arguments and `cached_get` reads it back; any graph **write** (sync, semantic
rebuild, decay) clears the whole cache, so context results are always
derived from a consistent graph snapshot.

1. **Context inference** (`infer_context`) reads one entity's incident edges
   via `relationship_details`, classifies each neighbor as a **structural**
   or **semantic** signal, recency-boosts fresh neighbors (+0.1 blended at
   0.8×), clamps to `[0, 1]`, and reports the per-signal mean confidence plus
   the weighted total.
2. **Workspace similarity** (`compute_workspace_similarity`): per-workspace
   goal vocabulary (Jaccard), cross-workspace edges (confidence×weight per
   pair), and — when an embedder is configured — cosine similarity of the
   profile text. Signals are combined with fixed weights
   (`0.45 / 0.30 / 0.25`), strong pairs survive a `0.18` floor and are
   persisted once per unordered pair under a canonical direction.
3. **Goal clustering** (`cluster_goals`): agglomerative — each goal-bearing
   node joins the centroid-cluster whose centroid-Jaccard is ≥ `0.30`,
   else it seeds a new cluster; confidence is mean membership cohesion.
4. **Snapshots & timeline**: a workspace snapshot stores node/edge counts,
   a knowledge summary, and a node-type histogram; the timeline diffs each
   snapshot against its predecessor.
5. **Fusion & planner retrieval** (`fused_context`, `planner_context`):
   multi-hop expansion keyed on the source, with memory records separated,
   embedder-boosted, and merged back into one ranked `fused` list;
   `planner_context` anchors a goal on its best graph match via
   `search_nodes`.
6. **Explanation** (`explain`): undirected BFS shortest path within 4 hops
   returns a hop chain with per-hop relationship + confidence; unreachable
   pairs fall back to shared-topic overlap as a weak, still-scored
   explanation.

### Persistence model

Each of the three tables is **derived data** written only by the service
through the repository — nothing rewrites an existing table, and two
relationships (`workspaces` FKs, the M1/M2 graph tables) are untouched.
Clusters and snapshots are replace-on-write per scope; relationships are
upserted in one canonical direction and read from either side.

---

## Test report

### Backend (14 new, 439 lib tests)

`context_intel_repository_tests.rs` (4 tests) — similarity round-trips both
directions of an ordered pair, upsert is idempotent and the similarity floor
is applied to **both** sides of the `OR` (a real SQL operator-precedence bug
caught here: `a OR b AND floor` binds the floor to `b` only, so the query is
parenthesized), snapshots list newest-first, clusters replace-on-write per
scope + clear.

`context_intel_service_tests.rs` (10 tests) — inference ranks semantic +
structural hits strongest-first against a deterministic `FakeEmbedder`
(alpha → `[1,0]`, beta → `[0,1]`, else `[1,1]`); workspace similarity detects
goal overlap, persists the pair, serves the second call from cache with the
`cached` flag set, and stays below floor for unrelated scopes; discover
recomputes and rewrites; goal clusters group "fix login bug"/"fix login
issue" and keep dissimilar goals singleton; knowledge summary counts
connections; snapshots + timeline report correct workspace-scoped counts;
fusion interleaves memory + KG hits behind correct labels; planner anchors on
the goal and degrades gracefully on a miss; explanations return real 2-hop
paths and fall back on isolated pairs; and any graph write invalidates the
shared context cache.

**Workspace scoping note:** snapshots count *workspace-scoped* nodes. The
planner-report node is intentionally workspace-less (`workspace_id = NULL` in
`planner_report_sources`), so workspace-scoped snaps exclude it — and the
`reports_on` edge whose report endpoint has no workspace is likewise
excluded from the workspace edge count. The test asserts the corrected,
excluded counts.

### Frontend (3 new, 42 total)

`GraphPage.test.tsx` — selecting a node loads the knowledge summary and
inference breakdown (`graph_knowledge_summary` / `graph_infer_context`) and
shows "Confidence breakdown" with signal chips; selecting a workspace shows
related workspaces (with a working Recompute →
`graph_discover_cross_workspace_relationships`), goal clusters, and snapshots with
a Capture → `graph_snapshot_create`; a non-workspace node (file) still loads
a node-scoped summary and never shows the workspace-only blocks
("Related workspaces", "Context snapshots").

---

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt` / `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 446 tests (439 lib + 6 integration + 1 doc, 3 ignored) |
| `npm run build` | ✅ |
| `npx tsc -b` | ✅ |
| `npm test` | ✅ 42 tests (6 files) |

Notes: `npm run lint` still reports pre-existing errors in files untouched by
this milestone (the same set M2 reported); every file this milestone touched
is lint-clean and clippy-clean. `context_intel_service.rs` is large but
consistent with the existing service convention (`kg_service.rs` is 1175 LOC
and `kg_live_service.rs` 950); splitting it was deliberately avoided to keep
the milestone additive.

---

## Design notes & trade-offs

- **Only databases running migration `0026` (or later) parse cleanly.** SQLite's
  `datetime('now')` emits `YYYY-MM-DD HH:MM:SS` — not RFC3339 — so
  `DateTime::parse_from_rfc3339` fails on it. The migration defaults are now
  `strftime('%Y-%m-%dT%H:%M:%fZ','now')`; the repository *also* writes
  explicit `Utc::now().to_rfc3339()` values on insert/upsert, so
  already-migrated databases (whose column defaults were baked in at
  migration time) still produce parseable rows. The M2 `graph_query_cache`
  era code was already RFC3339-clean; M3 inherits it.
- **The `cached` flag is set by the service on cache hits when calling
  `workspace_similarity`, mirroring `analytics`.** A cache round trip returns
  the stored payload (written with `cached: false`), so the caller-side flag
  is flipped before returning — the second call reports `cached: true`.
- **Empty signal vectors → 0.0, never NaN.** `build_breakdown` computes a mean
  only over non-empty signals and `clamp01` guards `NaN` before clamping; this
  keeps `serde_json` from silently serializing `NaN → null` in confidence
  payloads (an inference on an empty graph must report `total: 0`).
- **`planner_context` cache keys are content-addressed** (`len(goal)` +
  FNV-1a hash) rather than raw-goal-keyed, so long goals never blow up the
  cache-key space.
- **Context results are a derived view of the graph**, so they share the M2
  query cache and are cleared by any graph write for free — the same
  invalidation the dashboard already trusts.