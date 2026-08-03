# RC-8 M2 Engineering Report — Live Knowledge Graph

**Date:** 2026-08-03
**Branch:** `main`

---

## Summary

RC-8 M2 turns the RC-8 M1 knowledge graph from a snapshot constructed
on launch into a **live, self-maintaining layer**: incremental
watermark-driven syncs, semantic `related_to` edges with per-edge
confidence, exponential confidence decay with pruning, cached graph
analytics, multi-hop context expansion, related-work recommendations,
and a per-node relationship inspector — with a frontend dashboard that
updates itself from backend `graph:updated` events.

The milestone ships:

- **Migration `0025`** — additive columns/table: `confidence` on
  `graph_relationships`, `graph_sync_state` (per-aggregate watermark),
  `graph_query_cache` (scoped JSON payloads with TTL).
- **`models/kg_live.rs`** — the M2 DTO surface: `EntitySyncResult`,
  `SemanticEdgeResult`, `EdgeDecaySummary`, `DecayCandidate`,
  `DegreeBucket`, `NodeCentrality`, `GraphComponent`,
  `WorkspaceImportance`, `GraphAnalytics`, `MultiHopHit`,
  `MultiHopContext`, `GraphRecommendation`, `RelationshipDetail`,
  `RelationshipDetails`, `QueryCacheStats`, plus the `GraphEmbedder`
  trait and the `TypeCount` re-export.
- **`repositories/kg_live_repository.rs`** — all M2 SQL: entity sync
  (sources + structural links), semantic edge upsert/prune, decay
  candidate selection + confidence write-back, query cache, analytics
  fetches (degree distribution, eigenvector centrality inputs,
  components, workspace importance, node summaries).
- **`services/kg_live_service.rs`** — the M2 business logic:
  `sync_entity`, `sync_incremental` (watermarks), `rebuild_semantic_edges`
  (embedding → cosine → threshold), `apply_edge_decay` (exponential
  policy), `graph_analytics` (power-iteration centrality, components,
  workspace importance), `multi_hop_walk` (level-relaxation DP with hop
  decay), `recommendations`, `relationship_details`, `expand_context`,
  `query_cache` orchestration.
- **`graph/mod.rs`** — `GraphEngine` gains an optional `KgLiveService`
  via `with_kg_live_service` plus 9 additive facade methods.
- **9 new thin IPC commands** — `graph_incremental_sync`,
  `graph_sync_entity`, `graph_rebuild_semantic_edges`,
  `graph_apply_edge_decay`, `graph_analytics`, `graph_expand_context`,
  `graph_recommendations`, `graph_relationship_details`,
  `graph_cache_stats`.
- **Runtime wiring in `lib.rs`** — the live service is constructed with
  the real `MemoryVectorSystem` as its embedder, one startup incremental
  sync establishes the watermark, and a background worker syncs every 5
  minutes and applies decay every 6 hours, emitting `graph:updated`
  after each sync.
- **Frontend** — `KgEdge.confidence` + all M2 DTOs in `types/graph.ts`,
  9 new repository methods, an analytics panel (density, average degree,
  components, top central nodes, workspace importance), a relationship
  inspector with per-edge confidence, a live `graph:updated` subscription,
  Sync / Semantic / Decay maintenance controls, and confidence-scaled
  semantic edge rendering in the visualization.
- **Tests** — 13 new backend unit tests (5 repository + 8 service) and 3
  new frontend component tests.

The RC-8 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. The
schema is additive (migration `0025`); `GraphEngine::new` and every M1
command/method keep working; all SQL stays in the repository layer; all
scoring stays in the service layer; IPC handlers are one-line
delegations.

---

## Architecture

### What changed (additive only)

| Component | RC-8 M2 change |
|-----------|----------------|
| **`migrations/0025_graph_live.sql` (new)** | `confidence` on `graph_relationships`, `graph_sync_state`, `graph_query_cache` with TTL, indexes |
| **`models/kg_live.rs` (new)** | M2 DTOs, `GraphEmbedder` trait, `DecayCandidate` |
| **`repositories/kg_live_repository.rs` (new)** | all M2 SQL: entity sync sources, structural links, semantic upsert/prune, decay candidates + confidence write-back, query cache, analytics |
| **`services/kg_live_service.rs` (new)** | `sync_entity`, `sync_incremental`, `rebuild_semantic_edges`, `apply_edge_decay`, `graph_analytics`, `multi_hop_walk`, `expand_context`, `recommendations`, `relationship_details`, query cache |
| **`graph/mod.rs`** | `GraphEngine` gains `Option<KgLiveService>` via `with_kg_live_service`; 9 additive facade methods |
| **`commands/graph.rs`** | 9 new thin commands (delegation only) |
| **`lib.rs`** | wires `KgLiveRepository` → `KgLiveService` (with real embedder) → engine; startup watermark sync; background sync/decay worker emitting `graph:updated` |
| **`app_events.rs`** | `EVENT_GRAPH_UPDATED = "graph:updated"` |
| **Frontend** | `types/graph.ts` M2 DTOs + `KgEdge.confidence`; `graphRepository.ts` + 9 methods; `GraphPage.tsx` analytics panel, relationship inspector, live updates, Sync/Semantic/Decay controls; `KnowledgeGraphView.tsx` confidence rendering; `GraphPage.test.tsx` + 3 tests |

### Incremental sync model

`sync_incremental` keeps one **watermark per aggregate** in
`graph_sync_state` (key `(source, aggregate)`). Each pass processes one
aggregate at a time:

1. Read the saved watermark (first run falls back to `UNIX_EPOCH`).
2. Load rows whose `updated_at` (workspaces, files, plan executions,
   execution memory) or `created_at` (planner reports) is newer than the
   watermark.
3. Upsert each changed node, then upsert its **structural links**
   (`links_for`), skipping FK failures when an endpoint was deleted
   concurrently.
4. Advance the watermark to `now` and re-run if new source rows appeared
   mid-pass (loop guard cap 3).

**Aggregate order matters:** execution is processed before planner
reports so the `reports_on` edge's endpoints exist; incremental sync
builds one kind at a time, so `Execution` must precede `PlannerReport`
in the KINDS list (full `sync_graph` builds all nodes first and has no
such constraint). `sync_entity` is the single-entity version used by the
new IPC command and by unit tests.

### Semantic edges & confidence

`rebuild_semantic_edges` (capped at `MAX_SEMANTIC_NODES` = 500):

1. For every eligible node, build a text blob (title + summary +
   metadata fields) and embed it with the `GraphEmbedder` (the real
   `MemoryVectorSystem` in production, a deterministic fake in tests).
2. Compare each pair whose combined embedding length is ≥ 2 with cosine
   similarity; keep pairs ≥ `SEMANTIC_THRESHOLD` (0.45) as candidates.
3. Upsert a `related_to` edge per candidate with `weight = confidence =
   similarity`, and **prune** persisted `related_to` edges whose
   similarity dropped below the threshold.

Every graph write clears the query cache, so `graph_analytics` and
`graph_expand_context` never serve stale data after a maintenance pass.

### Decay policy

SQLite has no `POWER()`, `exp()` or `ln()`, so decay is a **two-layer
policy**:

- **Repository** reports the SQL-expressible facts:
  `decay_candidates(now, min_age_days)` selects all `related_to` edges
  older than `DECAY_FRESH_MIN_AGE_DAYS` (0.5 days) — deliberately *not*
  filtered on confidence, so a fresh semantic edge at 1.0 still ages
  (the original M1-era `confidence < 1.0` guard would have exempted it
  forever) — and `update_edge_confidence` writes the new value back.
- **Service** applies the exponential policy in Rust:
  `new = (confidence * 0.92.powf(age_days)).clamp(0.0, 1.0)`, rounded to
  4 dp, then prunes edges below `MIN_CONFIDENCE` (0.10). Structural
  edges are exempt by construction (excluded by relationship type, never
  decayed).

### Multi-hop context & recommendations

`multi_hop_walk` is a level-relaxation dynamic program over hop depth:
`current`/`next` maps of `(score, relationship_type, via)` per node,
advancing one hop at a time with `hop_decay = 0.5^(depth-1)`, keeping
the strongest `(score, depth, kind, via)` in a `best` map, and dropping
the source itself. Accumulated score = product of edge `weight ×
confidence`, × hop decay.

- `expand_context` walks up to `MAX_HOPS` (4), scores each reached node,
  sorts by weight descending, and caps at `limit`.
- `recommendations` walks up to `MAX_RECOMMENDATION_HOPS` (3), **skips
  direct neighbors** (hop 1), boosts 2-hop hits whose `via` is a
  `planner_report` (reports explain why work got done), combines the
  path score with cosine similarity between the source's and hit's
  embeddings, and caps at `MAX_RECOMMENDATIONS` (20).

### Analytics

`graph_analytics` (cached per scope, `ANALYTICS_TTL_SECONDS` = 60):
node/edge counts, average degree, density, degree distribution
histogram, **power-iteration eigenvector centrality**
(`CENTRALITY_ITERATIONS` = 12, normalized), connected components
(largest-first, per-type counts, up to 5 sample titles), and workspace
importance in global scope (eigenvector mass + confidence-weighted edge
strength). `workspace_importance` is only populated for the global
scope; workspace-scoped analytics return an empty list.

### IPC (thin) & runtime

`graph_incremental_sync` / `graph_sync_entity` / `graph_rebuild_semantic_edges`
/ `graph_apply_edge_decay` / `graph_analytics(workspace_id, cached)` /
`graph_expand_context(node_type, entity_id, hops, limit, cached)` /
`graph_recommendations` / `graph_relationship_details` /
`graph_cache_stats` — each a single delegation. `lib.rs` runs one
startup `incremental_sync` (logs the summary, establishes watermarks)
and spawns a background worker using `tokio::select!` over a 300-second
sync interval (emits `graph:updated` with the summary) and a 6-hour
decay interval.

---

## Frontend

- **Types** (`types/graph.ts`) — `KgEdge` gains `confidence: number`
  (0.0–1.0); all 15 M2 DTOs mirror the backend `serde(camelCase)` fields
  exactly (e.g. `GraphAnalytics`, `MultiHopContext`, `RelationshipDetails`).
- **Repository** (`graphRepository.ts`) — 9 new methods invoke the exact
  backend command names, all `cached`-flag-aware.
- **GraphPage** — header shows density + average degree, plus the last
  semantic (`+created · ~updated · -pruned`) and decay (`decayed ·
  pruned`) pass results; three maintenance controls (Sync =
  `graph_incremental_sync`, Semantic = `graph_rebuild_semantic_edges`,
  Decay = `graph_apply_edge_decay`); the right-hand panel now shows a
  **Graph analytics** view by default (scope/cache badge, stat grid,
  top central nodes by eigenvector, workspace importance) and, when a
  node is selected, a **relationship inspector** listing each
  relationship with its neighbor, type, weight, and (for `related_to`)
  confidence; the page subscribes to `graph:updated` in addition to the
  M1 events, so the backend's 5-minute sync refreshes the page live.
- **Visualization** (`KnowledgeGraphView.tsx`) — `related_to` edge
  stroke opacity is scaled by `edge.confidence`
  (`0.2 + confidence * 0.8`), so stale edges visually fade; hovering a
  semantic edge shows its confidence in a native tooltip.

---

## Tests

### Backend (13 new)

| Area | Tests |
|------|-------|
| `KgLiveRepository` (5) | query cache round-trips with TTL and clears on graph writes; node fetches respect workspace scope; semantic edge upsert is idempotent; low-confidence edges are pruned; decay candidates select only aged `related_to` edges and write-back updates confidence |
| `KgLiveService` (8) | incremental sync builds then advances watermarks; `sync_entity` is idempotent and drops missing sources; semantic rebuild persists confident pairs and prunes stale ones; decay ages semantic edges only (structural untouched); multi-hop expansion walks hops with decay and caches; recommendations skip direct neighbors and explain via planner reports; analytics payload round-trips through the query cache; relationship details surface incident edges with resolved neighbors |

The service tests use a deterministic `FakeEmbedder` (alpha → `[1,0]`,
beta → `[0,1]`, else `[1,1]`) and seed workspaces/files/executions +
`copilot_conversations` + `plan_execution_reports` rows so FKs hold.

### Frontend (3 new, 39 total)

`GraphPage.test.tsx` — analytics loaded on mount with density shown;
incremental sync / semantic rebuild / edge decay each invoke the right
command and surface their summaries; the relationship inspector shows
per-edge confidence (`conf 62%`) for a selected node.

---

## Quality gates

| Gate | Result |
|------|--------|
| `cargo fmt` / `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 432 tests (425 lib + 6 integration + 1 doc, 3 ignored) |
| `npm run build` | ✅ |
| `npx tsc -b` | ✅ |
| `npm test` | ✅ 39 tests (6 files) |

Notes: `npm run lint` still reports pre-existing errors in files
untouched by this milestone (`workspaceRepository.ts`, `predictive.ts`,
`TimelinePage.tsx`, `WorkspacesPage.tsx`, `MarkdownRenderer.tsx`,
`GraphView.tsx`, ...); every file this milestone touched is lint-clean.

---

## Design notes & trade-offs

- **SQLite has no `POWER`/`exp`/`ln`.** The decay policy therefore lives
  in the service (`0.92.powf(age_days)`), with the repository only
  selecting candidates and writing results back. This also fixed a
  latent M1-era bug: a `confidence < 1.0` SQL guard would have exempted
  fresh semantic edges at 1.0 from decay forever. Structural edges are
  excluded by relationship type, not confidence.
- **`confidence` belongs on the edge, not the node.** Semantic similarity
  is pairwise; centrality and importance are computed from
  confidence-weighted edge strengths rather than raw counts.
- **Watermarks over timestamps-only.** Storing the last-seen `updated_at`
  per aggregate makes incremental sync a cheap diff instead of a full
  rescan, and the mid-pass re-run loop keeps it correct when rows land
  during the pass.
- **sqlx `Uuid` is a BLOB in SQLite.** Tests bind `Uuid` values directly
  and `SELECT ... ORDER BY rowid DESC LIMIT 1` to recover seeded edge
  ids — a string-bound id silently breaks FK/equality comparisons.
- **`#[async_trait::async_trait]` on the `GraphEmbedder` impl.** The
  impl for `MemoryVectorSystem` lives at the bottom of
  `kg_live_service.rs` (not in `models`, to keep the `copilot` module
  out of models) and needs the explicit attribute for the lifetime-safe
  boxed future (E0195 otherwise). `KgLiveService` carries a manual
  `Debug` impl because it holds `Arc<dyn GraphEmbedder>`.
- **MSRV is 1.77.** Clippy rejected `Option::is_none_or` (stable 1.82);
  `map_or(true, ...)` is used in `multi_hop_walk` instead. Clippy also
  required `sort_by_key(Reverse(...))` and `unwrap_or` for constant
  defaults.
- **Frontend tests click buttons, not handlers.** Waiting for the
  maintenance buttons to re-enable between actions avoids clicking a
  still-disabled button; the semantic command assertion passes
  `expect.anything()` for the `{ maxNodes: undefined }` args object.

## Out of scope (later RC-8 milestones)

Embedding index/maintenance jobs (periodic re-embedding of changed
nodes), per-relationship evidence surfacing in the UI, user-confirmed
edges (human feedback into `related_to`), cross-workspace analytics
comparison views, and graph export.
