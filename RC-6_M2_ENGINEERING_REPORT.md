# RC-6 M2 Engineering Report — Production Vector Memory System

**Date:** 2026-08-02
**Branch:** `main`

---

## Summary

RC-6 M2 replaces the placeholder semantic retrieval over execution memory
with a **production-quality vector memory system**. Execution memory now
embeds every remembered goal through a real local embedding provider
(character n-gram hashing), caches embeddings in two tiers (in-memory LRU
+ durable SQLite), keeps a k-NN vector index (in-memory for search,
SQLite for durability), and indexes **incrementally in the background**:
captures notify the indexer worker, which batch-embeds only the records
that are new or whose goal changed, and re-embeds automatically when
memories change.

The RC-6 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC changes, and no breaking database
schema**. All new tables are additive (migration `0021`); existing M1
behavior (token-overlap retrieval, learning engine, planner reuse)
continues unchanged. The memory system still only persists and retrieves
— it never schedules, plans, or drives a session.

---

## Architecture

### What changed (additive only)

| Component | RC-6 M2 change |
|-----------|----------------|
| **`memory/vector/` (new module)** | provider abstraction, local n-gram embedder, LRU cache, in-memory k-NN index, SQL repository, background indexer |
| `MemoryEngine` | captures stop blocking on embedding → store + notify indexer; search/recommend/avoid use k-NN candidates; new `index_pending` / `reindex` / `vector_status` |
| `MemoryRepository` | `get_many`, `update_goal_embedding`, `pool()` (SQL for execution_memory only) |
| `retrieval.rs` | term-frequency weighted token overlap + exact-match normalization (ranking improvements) |
| IPC | 2 new thin commands: `memory_index_status`, `memory_reindex` |
| Frontend | Memory dashboard vector-index card (provider, dims, coverage, cache hit rate) + "Index now" action |

### Data flow (M2)

```
capture (engine/runtime terminal state)
   │  upsert row (no embedding; instant)
   ▼
MemoryIndexer.notify() ──debounce 150 ms──► index_pending()
                                              │ list_pending (LEFT JOIN: new OR
                                              │   updated_at > indexed_at)
                                              ▼
                                   CachedProvider.embed_batch()
                                      │ memory LRU cache → SQLite cache
                                      │   → LocalVectorProvider (n-gram hashing)
                                      ▼
                        execution_memory.goal_embedding  (back-fill)
                        memory_vector_index row          (durable, indexed_at)
                        VectorIndex (in-memory k-NN)     (upsert)

search/recommend/avoid (query path)
   embed query (cached) → knn_candidates (oversampled k-NN)
   → get_many(ids) → filters → blended rank (cosine + TF overlap + learning)
```

- **Warm-up**: at startup the durable index rows are loaded into the
  in-memory k-NN index (`MemoryIndexer::warm_up`), so search is
  vector-based immediately after launch.
- **Safety net**: the worker also runs a pass every 60 s, so records are
  indexed even if a notification was missed.

---

## Deliverables

### Migration

| File | Lines | Description |
|------|-------|-------------|
| `migrations/0021_memory_vector_index.sql` | 35 | `memory_vector_index` (memory_id PK → execution_memory, text_hash, text, embedding BLOB, dim, indexed_at) + `memory_embedding_cache` (text_hash PK, text, embedding, dim, created_at). Additive; `CURRENT_SCHEMA_VERSION` bumped 19 → 21 |

### Backend (src-tauri/src/copilot/memory/)

| File | Lines | Description |
|------|-------|-------------|
| `vector/mod.rs` | 168 | `MemoryVectorSystem` facade (cached provider + k-NN index + durable SQL + indexer), `VectorIndexStatus`, `IndexResult` |
| `vector/provider.rs` | 239 | **Real provider abstraction**: `VectorProvider` trait (`embed`, `embed_batch`, dims, name) + `CachedProvider` two-tier cache decorator |
| `vector/local.rs` | 222 | **Local embedding provider**: character n-gram (3–5) hashing embedder, TF-weighted, L2-normalized — real sub-word semantic similarity, deterministic across processes |
| `vector/cache.rs` | 209 | **Embedding cache**: in-memory LRU (512 texts) with hit/miss counters and hit-rate stats |
| `vector/index.rs` | 194 | **k-NN index**: in-memory cosine index over normalized vectors, clone-able, upsert/remove/clear/knn |
| `vector/repository.rs` | 427 | **MemoryVectorRepository** — all SQL for the index + persistent cache, including the incremental `list_pending` / `count_pending` LEFT-JOIN predicates |
| `vector/indexer.rs` | 391 | **Background indexing worker**: notify-based (debounced) + 60 s interval, chunked batch embedding, `index_pending`, `reindex_all`, `warm_up`, shutdown flag |
| `engine.rs` | 439 | facade keeps capture/search/recommend/avoid + vector system accessors; tests moved to `engine_tests.rs` |
| `engine_tests.rs` | 497 | moved M1+M2 engine test suite (file kept under the 500-line guideline) |
| `repository.rs` | 619 | +`get_many`, `update_goal_embedding` (never touches `updated_at`, so indexing never re-pends its own writes), `pool()` |
| `models.rs` | 356 | shared pure helpers: `embedding_to_blob`/`embedding_from_blob` (one wire format), stable `text_hash` |
| `retrieval.rs` | 312 | TF-weighted token overlap (multiset coverage) replaces set Jaccard; exact-match normalization → 1.0 |

### IPC (src-tauri/src/commands/memory.rs + lib.rs)

- `memory_index_status` → `VectorIndexStatus` (total/indexed/pending, provider, dims, last indexed, cache stats) — thin wrapper.
- `memory_reindex` → `IndexResult` (requested/indexed/failed/skipped) — thin wrapper.
- lib.rs: memory engine gets its own `LocalVectorProvider`; the indexer is warmed up and spawned as a background task at startup.

### Frontend (frontend/src/)

| File | Description |
|------|-------------|
| `types/memory.ts` | +`VectorIndexStatus`, `IndexResult` |
| `services/memoryRepository.ts` | +`indexStatus()`, `reindex()` |
| `features/memory/MemoryDashboard.tsx` | Vector index card: indexed/total coverage, pending, provider · dims, cache hit rate, last indexed, "Index now" button (spinner while indexing, refreshes overview after) |
| `features/memory/MemoryDashboard.test.tsx` | +2 tests (status card rendering, re-index action + overview refresh) |

---

## Key Features Implemented

### 1. Real embedding provider abstraction (`VectorProvider`)
- One trait for the memory system: single-text `embed` and batch
  `embed_batch` (providers with real batch paths override the default
  loop), plus `dimensions`/`name` for the dashboard.
- `CachedProvider` decorates any provider: in-memory LRU → durable
  SQLite cache → wrapped provider. A text is embedded **once per process
  run and once ever across restarts**.

### 2. Local embedding provider (`LocalVectorProvider`)
- Replaces the whole-string hash placeholder with the hashing trick:
  words + character n-grams (3–5) per token, TF-weighted, hashed into a
  384-dim vector with signed buckets, L2-normalized.
- Meaningful similarity: "resume my focus session" and "resume my last
  focus session" embed close (cosine > 0.5, pinned by test); "organize
  tax receipts" embeds apart. Deterministic across runs (fixed-key
  hashing) — safe for the persistent cache.

### 3. Embedding cache
- In-memory LRU (capacity 512) with hit/miss/hit-rate counters surfaced
  in the dashboard.
- Durable SQLite cache keyed by stable `text_hash`; stored text guards
  against hash collisions before a cached vector is trusted.

### 4. Background indexing worker
- `MemoryIndexer` spawned at startup; capture → `notify()` → 150 ms
  debounce → one pass; 60 s interval as safety net; graceful shutdown
  flag for tests.

### 5–6. Vector similarity search & k-NN retrieval
- `VectorIndex` (in-memory): L2-normalized upserts, cosine k-NN with
  ranking. Search/recommend/avoid embed the query once and take
  **oversampled candidates** (5×/20×) so workspace/status filters don't
  starve results, then rank with the blended score (0.6 cosine + 0.4 TF
  overlap + learning blend for recommend/avoid).
- **Search optimization**: only the top-k candidate ids leave SQL
  (`get_many`) — no full-table decode of every record's plan/steps for
  every query. Cold-start (empty index) falls back to the full
  token-overlap scan, so behavior is correct before the first pass.

### 7. Memory ranking improvements
- Token overlap upgraded from set Jaccard to **term-frequency weighted
  multiset coverage** (`Σmin(tf) / max(Σtf)`): repeated keywords count
  proportionally instead of being collapsed into sets.
- Exact-match normalization: a case/space-insensitive identical goal
  scores 1.0 even without embeddings.

### 8–10. Incremental embedding, automatic re-indexing, batch generation
- `list_pending` (SQL, LEFT JOIN on `indexed_at`/`updated_at`) returns
  only records that are **new, unembedded, or whose goal changed** since
  their last index write — each pass embeds only those, in chunks of 64.
- Every index write persists to the SQLite index **and** the in-memory
  k-NN index, and back-fills `execution_memory.goal_embedding` (one
  write path per artifact; `update_goal_embedding` deliberately leaves
  `updated_at` untouched so an index pass can never re-pend itself).
- `reindex_all` drops and rebuilds the entire index.

---

## Backward Compatibility

- **No breaking IPC changes**: only two additive commands; all existing
  commands unchanged.
- **Additive schema**: one new migration; existing tables untouched.
- **Existing tests pass unchanged**: planner/runtime/engine M1 tests
  still pass (they swapped the provider constructor only); without a
  warm index, retrieval degrades to the same token-overlap behavior as
  M1.
- Deterministic-plan tests remain green — memory is still optional
  (`Option<Arc<MemoryEngine>>`) and empty memory ⇒ identical behavior.

---

## Tests

### Backend (all passing)

| File | New tests | Highlights |
|------|-----------|------------|
| `vector/local.rs` | 6 | determinism, L2 normalization, similar-goals-embed-close, batch ≡ single, empty text |
| `vector/cache.rs` | 5 | LRU eviction order, hit/miss counters, zero-capacity disabled, clear |
| `vector/index.rs` | 5 | k-NN ordering, identical→1.0, k/empty handling, remove/clear, upsert overwrite |
| `vector/provider.rs` | 3 | memory-cache hit, SQLite cache survives eviction, batch only embeds misses |
| `vector/repository.rs` | 4 | index round-trip, pending new/changed tracking, last-indexed stamp, cache round-trip with text guard |
| `vector/indexer.rs` | 6 | pending→indexed everywhere, incremental (no re-embed), reindex rebuild, warm-up, run-loop on notify, batch chunking |
| `engine_tests.rs` | +5 | k-NN search after indexing, vector ranking separation, recommend via k-NN, re-index after goal change, cache hits on repeated queries |
| `repository.rs` | +2 | `get_many`, `update_goal_embedding` (with the no-`updated_at` guarantee) |
| `retrieval.rs` | +2 | TF-weighted overlap values, exact-match normalization |
| `models.rs` | +2 | blob round-trip/misalignment, stable distinct text hashes |

**Totals: 40 new backend tests — 340 lib + 5 integration + 1 doc = 346 passing, 0 failed.**

### Frontend (all passing)

| File | Tests | Status |
|------|-------|--------|
| `MemoryDashboard.test.tsx` | 6 (stats, search, recommend/avoid, learned workflows, **vector index status**, **re-index + refresh**) | ✅ |

**Totals: 25 frontend tests, all passing (23 M1 + 2 new).**

---

## Gates

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | ✅ clean |
| `cargo clippy --all-targets -- -D warnings` | ✅ 0 warnings |
| `cargo build` | ✅ |
| `cargo test` | ✅ 346 passed / 0 failed |
| `npx tsc -b` | ✅ |
| `npm run build` | ✅ |
| `npm test` (vitest) | ✅ 25 passed |
| `npm run lint` | ✅ no new errors (18 pre-existing, verified identical to base) |

---

## Engineering Notes

- **Capture is now instant**: M1 embedded inline on every capture; M2
  defers embedding to the background worker (incremental + batched), so
  captures never block the execution lifecycle on provider latency. The
  dashboard shows pending counts so the async state is transparent.
- **Single writer discipline preserved**: the indexer is the only writer
  of embeddings; the engine writes rows, the worker writes vectors.
- **No duplicated logic**: blob encode/decode and text hashing are shared
  pure helpers in `models.rs`; both repositories use them. The vector
  system is a new module — the semantic layer (Phase 6A) is untouched.
- **Files under the 500-line guideline**: the engine's test suite moved
  to `engine_tests.rs` (via `#[path]`) so no source file in the memory
  domain exceeds ~620 lines (largest new file: 497); only the pre-existing
  `repository.rs` (619) carries M1+ M2 repository methods.
- **Timing safety**: `update_goal_embedding` never bumps `updated_at`,
  and the index stamps `indexed_at` per write, so the incremental
  predicate (`updated_at > indexed_at`) cannot re-pend the record an
  index pass just wrote.
