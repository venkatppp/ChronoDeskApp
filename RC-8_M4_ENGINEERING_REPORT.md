# RC-8 M4 Engineering Report — Knowledge Graph Optimization & Scale

**Date:** 2026-08-03
**Branch:** `main`

---

## Summary

RC-8 M4 turns the RC-8 knowledge graph from a *correct* graph into a
*scalable, observable, self-healing* one. The milestone adds paginated /
virtualized graph loading, ranked + vector-assisted search, rayon-parallel
multi-root traversal, a persisted operational ledger (integrity issues,
maintenance runs, query metrics, benchmarks), integrity checks with repair,
orphan detection/cleanup, consistency verification, a micro-benchmark suite,
graph memory/cache statistics, and a full performance dashboard in the
frontend.

The milestone ships:

- **Migration `0027`** — four additive tables: `graph_integrity_issues`
  (persisted findings with an open → resolved lifecycle), `graph_maintenance_runs`
  (one row per integrity/repair/cleanup/consistency/benchmark pass),
  `graph_query_metrics` (append-only per-operation latency/volume ledger),
  and `graph_benchmarks` (persisted suite results). All timestamp defaults
  use the RFC3339 `strftime('%Y-%m-%dT%H:%M:%fZ','now')` convention.
- **`models/kg_opt.rs`** — the full M4 DTO surface: `NodePage`, `EdgePage`,
  `NeighborPage`/`NeighborRow`, `RankedSearchHit`, `IssueType`/`IssueSeverity`,
  `GraphIntegrityIssue`, `IntegrityCheckResult`, `RepairResult`, `OrphanSummary`,
  `OrphanCleanupResult`, `ConsistencyCheck`/`ConsistencyReport`, `QueryMetric`,
  `GraphMemoryStats`, `MaintenanceRun`, `GraphBenchmarkResult`,
  `BenchmarkSuiteResult`, `ParallelWalkResult`, `GraphDiagnostics`.
- **`repositories/kg_opt_repository.rs`** — all M4 SQL: paginated
  node/edge/neighbor loading with totals, the four integrity scans, repair
  helpers, issue persistence (dedup-aware), maintenance/benchmark/metric
  persistence. `KgLiveRepository` gains exactly three additive cache methods
  (`cache_size_bytes`, `cache_trim`, `cache_clear_expired`).
- **`services/kg_opt_service.rs`** — paginated loading, virtualized page
  totals, ranked search (title-prefix > title > summary, recency-boosted),
  vector-assisted search (cosine over embedded titles via the memory
  system's embedder), rayon-parallel multi-root BFS, cache trimming/expiry,
  memory statistics, and the shared query-metrics ledger.
- **`services/graph_health_service.rs`** — the four integrity scans with
  persisted, deduplicated findings; repair; orphan summary + cleanup;
  five-probe consistency verification; maintenance-run history; the
  micro-benchmark suite (8 benchmarks over the real service surfaces);
  and the combined diagnostics bundle.
- **`graph/mod.rs`** — `GraphEngine` gains `Option<KgOptService>` +
  `Option<GraphHealthService>` via `with_kg_opt_service` /
  `with_graph_health_service` plus 16 additive facade methods.
- **19 new thin IPC commands** in `commands/graph_opt.rs` (delegation only).
- **Runtime wiring in `lib.rs`** — both services constructed with the real
  `MemoryVectorSystem` embedder, attached to the engine, managed as state,
  plus a background maintenance worker: an hourly TTL-sweep of the query
  cache and a 6-hourly integrity check (recording only — repair stays a
  user-triggered action).
- **`Cargo.toml`** — `rayon = "1.10"` (parallel traversal).
- **Frontend** — `types/graphOptimization.ts` (all M4 DTOs), a new
  `graphOptimizationRepository.ts` (20 methods), `VirtualizedNodeList.tsx`
  (windowing + progressive loading), `GraphPerformancePage.tsx` with a
  performance dashboard (memory/cache stats, integrity panel, orphans +
  consistency, benchmark viewer, query metrics, maintenance history,
  virtualized node browser), a nav entry + route (`/graph/performance`), a
  Performance link on `GraphPage`, and a progressive-loading "load more"
  pill on `KnowledgeGraphView`.

The RC-8 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. The schema
is additive (migration `0027`); every M1–M3 command and method keeps working;
all SQL stays in repositories; all ranking/repair/benchmark policy stays in
services; IPC handlers are one-line delegations.

---

## Architecture

### What changed (additive only)

| Component | RC-8 M4 change |
|-----------|----------------|
| **`migrations/0027_graph_optimization.sql` (new)** | `graph_integrity_issues`, `graph_maintenance_runs`, `graph_query_metrics`, `graph_benchmarks`; RFC3339 `strftime` timestamp defaults |
| **`models/kg_opt.rs` (new)** | all M4 DTOs (pagination, integrity, repair, orphans, consistency, metrics, memory, maintenance, benchmarks, traversal, diagnostics) |
| **`repositories/kg_opt_repository.rs` (new)** | paginated node/edge/neighbor pages + totals; orphan/dangling/malformed/confidence scans; repair helpers; issue/maintenance/benchmark/metric persistence |
| **`repositories/kg_live_repository.rs`** | +3 additive cache methods: `cache_size_bytes`, `cache_trim`, `cache_clear_expired` (the only M2-file change) |
| **`services/kg_opt_service.rs` (new)** | pagination, ranked + vector search, rayon parallel traversal, cache trim/expiry, memory stats, metrics ledger |
| **`services/graph_health_service.rs` (new)** | integrity checks + issue persistence, repair, orphans, consistency, maintenance runs, benchmark suite, diagnostics |
| **`graph/mod.rs`** | `GraphEngine` gains `Option<KgOptService>`/`Option<GraphHealthService>`; 16 additive facade methods |
| **`commands/graph_opt.rs` (new)** | 19 new thin commands (delegation only) |
| **`lib.rs`** | wires `KgOptRepository` → `KgOptService` → `GraphHealthService` → engine; background maintenance worker; state + command registration |
| **`Cargo.toml`** | `rayon = "1.10"` |
| **Frontend** | `types/graphOptimization.ts`; `graphOptimizationRepository.ts`; `VirtualizedNodeList.tsx`; `GraphPerformancePage.tsx` + test; `GraphPage` Performance link; `KnowledgeGraphView` load-more pill; sidebar nav + route |

### How the surfaces compose

1. **Pagination** (`nodes_page`/`edges_page`/`neighbors_page`) is
   repository-owned SQL (typed pages + a `COUNT(*)` total, `has_more` from
   `offset + len < total`); the service times the call and records a
   `paginate_*` metric. Neighbor resolution batches all page-neighbor node
   fetches into a single `node_type = ? AND entity_id = ? OR …` query — no
   N+1.
2. **Ranked search** re-scores the M1 keyword hits by match quality
   (title prefix 1.0 > title contains 0.85 > summary contains 0.6 > indexed
   0.3) with a +0.05 recency bonus for nodes touched in the last 7 days, and
   explains the rank per hit.
3. **Vector-assisted search** embeds the query and candidate titles through
   the same `GraphEmbedder` M2 uses for semantic edges (the memory vector
   system), keeps pairs at cosine ≥ 0.20, and reports the similarity as the
   score.
4. **Parallel multi-root traversal** snapshots the full node/edge registry
   once, builds an in-memory adjacency map, then `rayon::par_iter` walks BFS
   from each root (depth ≤ 6, per-root budget) and merges the deduplicated
   union of reached nodes/edges.
5. **Integrity checks** run the four scans — orphan edges (left-join probe),
   dangling workspace nodes, malformed nodes (empty title/summary or unknown
   type), out-of-range confidence — persist findings deduplicated against
   open issues, and record a maintenance run. Because the current schema
   enforces FKs + CHECKs, these scans are a corruption/legacy safety net;
   tests simulate legacy rows through an FK-off connection.
6. **Repair** deletes orphan edges and dangling nodes, fixes (or drops)
   malformed rows, clamps out-of-range confidence, resolves the affected
   open issues, and invalidates the query cache. **Orphan cleanup** is the
   same destructive pass scoped to edges + dangling nodes.
7. **Consistency verification** answers five pass/fail probes (node
   uniqueness, forward references, workspace references, well-formedness,
   confidence bounds) for the diagnostics panel.
8. **Benchmark suite** times the real service surfaces (node/edge/neighbor
   pages, ranked search, vector search when an embedder exists, memory and
   cache stats, parallel traversal), persists each result, and records a
   `benchmark` maintenance run.
9. **Diagnostics** bundles a fresh integrity pass + consistency report +
   memory stats + recent maintenance/benchmarks/metrics into one payload the
   performance page renders on load.

### Persistence model

All four tables are **derived data** written only by the services through
the repository. Issues carry `(issue_type, entity_id)` dedup keys so repeated
scans never duplicate rows; repair/cleanup resolve issues by type + entity id.
`graph_query_metrics` is append-only; `graph_maintenance_runs` is written
once per pass. Entity ids in `graph_integrity_issues` are stored as text
(stringified UUIDs) so the ledger never depends on SQLite's dynamic BLOB/TEXT
handling of uuid columns.

---

## Test report

### Backend (22 new; 468 total: 461 lib + 6 integration + 1 doc, 3 ignored)

`kg_opt_repository_tests.rs` (8 tests) — pagination reports totals,
newest-first ordering, and `has_more` across pages (nodes filtered by type;
edges and neighbors over a seeded `contains` edge); orphan-edge and
dangling-workspace scans find rows seeded through a connection with FK
enforcement off (the only way such rows can exist under the current schema)
and the delete helpers remove them; the malformed scan catches an empty-title
node and `fix_malformed_node` restores `(untitled)`; the confidence scan is
empty on CHECK-enforced data and `clamp_edge_values` is harmless on a valid
row; issues round-trip through insert → open list → per-type counts →
resolve → recent (with resolved timestamps); maintenance runs, benchmarks,
and metrics round-trip with stable newest-first ordering (`occurred_at DESC,
id DESC` tiebreak).

`kg_opt_service_tests.rs` (7 tests) — pagination across nodes/edges/neighbors
with totals; ranked search ranks the title-prefix hit above the contains hit
and respects type scoping; vector search ranks the semantically similar title
first (deterministic `FakeEmbedder`); parallel traversal reaches a
workspace + both files (deduped union), merges a disjoint second root, and
handles empty roots; cache expiry drops the old-TTL row while keeping the
fresh one, trim removes oldest-first; memory stats aggregate registry +
cache footprint; tracked operations appear in the metrics ledger.

`graph_health_service_tests.rs` (7 tests) — integrity checks persist orphan +
dangling findings with per-type counts and **do not duplicate them on a
repeat pass** (three maintenance runs recorded for clean + two scan passes);
repair removes the legacy problems and resolves ≥ 2 issues, leaving a clean
re-scan; orphan summary/cleanup round-trip with resolved issues; consistency
passes on a clean graph (5 checks) and flips "Forward references" on a seeded
orphan; the benchmark suite runs on a real database (even an empty graph),
persists results, records `benchmark` + metrics; diagnostics aggregates
every ledger (fresh integrity run included).

### Frontend (7 new page tests; 49 total across 7 files)

`GraphPerformancePage.test.tsx` — renders memory/cache statistics,
consistency checks, query metrics, and maintenance history from one
`graph_diagnostics` call; runs `graph_integrity_check` + `graph_repair` and
shows the repair summary; detects and cleans up orphans
(`graph_orphan_summary` / `graph_orphan_cleanup`); verifies consistency;
runs the benchmark suite (`graph_benchmark_suite`) and lists suite results;
trims (n=50) and sweeps the query cache; loads the virtualized node browser
progressively — scrolling near the bottom of the windowed list issues a
second `graph_nodes_page` call at the new offset.

---

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt` / `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 468 tests (461 lib + 6 integration + 1 doc, 3 ignored) |
| `npm run build` | ✅ |
| `npx tsc -b` | ✅ |
| `npm test` | ✅ 49 tests (7 files) |

Notes: `npm run lint` still reports the pre-existing error set in files this
milestone did not touch (the same set M2/M3 reported); every file M4 touched
is lint-clean and clippy-clean. The `GraphEngine` no longer derives `Debug`
(its new optional services hold `Arc<dyn GraphEmbedder>`, which is not
`Debug`); nothing in the codebase relied on it.

---

## Design notes & trade-offs

- **Integrity scans are defensive, not reactive.** The current schema already
  enforces the four invariants (FKs with `ON DELETE CASCADE`, `CHECK` on
  node types and confidence bounds), so orphan/dangling/malformed/confidence
  rows can only exist in legacy or corrupted data. The scans, repair pass,
  and persisted issue ledger exist precisely to detect and clean that state,
  and the tests simulate it by seeding through an FK-off connection.
- **UUIDs are BLOB-encoded by sqlx.** The uuid columns (`graph_nodes`,
  `graph_relationships`) are stored as 16-byte BLOBs when written through
  sqlx's `Uuid` binding, so tests seeding "legacy" rows through raw SQL must
  bind `Uuid` values (not `to_string()`) or the scans' `Uuid` decode fails.
  The integrity ledger is the exception: `graph_integrity_issues.entity_id`
  is TEXT and stores stringified UUIDs so issue bookkeeping is
  representation-independent.
- **Repair is user-triggered; the background worker only records.**
  A scheduled pass (hourly TTL sweep, 6-hourly integrity check) surfaces
  findings but never mutates the graph — destructive repair stays a
  deliberate, visible user action on the performance page.
- **Metrics are best-effort.** A failed metric insert must never fail the
  operation that produced it, so `record_metric` swallows the error after
  logging; the benchmark suite measures the *service* surfaces, so the
  measured calls also appear in the metrics ledger.
- **Parallel traversal is bounded.** Snapshot-once + rayon BFS is only safe
  because depth and per-root budgets cap the walk; the union is deduplicated
  by node key so overlapping root neighborhoods are reported once.
- **Pagination replaces whole-graph loads where it matters.** The virtualized
  browser and the `KnowledgeGraphView` load-more pill page through the node
  registry instead of one capped `list_nodes` fetch, so multi-thousand-node
  graphs render incrementally without blocking the UI thread.
