# RC-6 M1 Engineering Report — Memory & Learning System

**Date:** 2026-08-02
**Branch:** `main`

---

## Summary

RC-6 M1 delivers the **Memory & Learning System**: ChronoDesk now learns from every previous execution. Plan executions, planner reports, and autonomous sessions are captured into a durable **Execution Memory Store**; the **semantic retrieval** engine finds similar goals/plans; the **Learning Engine** ranks history, recommends successful workflows, and flags failed strategies to avoid; the **Planner** reuses learned workflows before building a new plan; and the **AutonomousRuntime** consults memory while reasoning and records every terminal session.

The implementation strictly preserves the RC-6 charter: **no architecture rewrites**, **no duplicate execution paths**, **no breaking IPC changes**, and **no business logic in IPC commands**. Memory is a read/write-only store — it never schedules, plans, or drives a session.

---

## Architecture

### Responsibility Split (unchanged + one new store)

| Component | Responsibility | RC-6 M1 change |
|-----------|----------------|----------------|
| `Planner` | planning, replanning, DAG generation | consults memory before planning; reuses learned workflows |
| `ExecutionEngine` | execution, scheduling, checkpoints, lifecycle, streaming | captures terminal runs into memory; records planner reports |
| `ToolExecutor` | only execution path for a single tool | unchanged |
| `AutonomousRuntime` | session budgets, retries, timeouts, approvals, reasoning loop | consults memory during recovery; captures terminal sessions |
| **`MemoryEngine` (new)** | **persistence + retrieval + learning only** | new (RC-6 M1) |

### Data flow

```
ExecutionEngine ──terminal state──► MemoryEngine ──► execution_memory (SQLite)
Planner          ──attach report──► MemoryEngine ──► planner_report row
AutonomousRuntime──terminal session► MemoryEngine ──► autonomous_session row
                                         │
Planner.plan() ◄──recommend()───────────┘   (reuse successful workflow)
AutonomousRuntime.recover_failure() ◄──avoid()──┘  (avoid failed strategies)
Frontend Memory page ◄──search/stats/learned_workflows──┘
```

---

## Deliverables

### Backend (src-tauri/)

| File | Lines | Description |
|------|-------|-------------|
| `migrations/0020_execution_memory.sql` | 35 | `execution_memory` table (kind/source_id unique, embeddings BLOB, replay counters) |
| `copilot/memory/mod.rs` | 30 | Public re-exports |
| `copilot/memory/models.rs` | 290 | Pure data types: `ExecutionMemoryRecord`, `MemoryKind`, `MemoryStatus`, `MemoryOutcome`, `MemoryHit`, `MemoryRecommendation`, `AvoidedStrategy`, `LearnedWorkflow`, `MemoryStats`, `MemorySearchRequest`, goal fingerprinting |
| `copilot/memory/repository.rs` | 430 | SQLite persistence (upsert keyed on kind+source_id, filters, counts, replays) |
| `copilot/memory/retrieval.rs` | 230 | Semantic similarity (zero-centered cosine + token overlap), ranking, filtering |
| `copilot/memory/learning.rs` | 420 | Learning engine: learned score blend, recommendations, avoid list, workflow aggregation, stats |
| `copilot/memory/engine.rs` | 470 | `MemoryEngine` facade: capture executions/reports/sessions, search, recommend, avoid, stats |
| `commands/memory.rs` | 88 | Tauri IPC handlers (`memory_search`, `memory_recommend`, `memory_avoid`, `memory_learned_workflows`, `memory_stats`) |

### Frontend (frontend/src/)

| File | Description |
|------|-------------|
| `types/memory.ts` | TypeScript mirrors of the memory models |
| `services/memoryRepository.ts` | IPC bindings (search, recommend, avoid, learnedWorkflows, stats) |
| `features/memory/MemoryDashboard.tsx` | Memory dashboard: stats cards, semantic search, workflow recommendations, avoid list, recent memories, learned workflows |
| `features/memory/MemoryDashboard.test.tsx` | 4 component tests |
| `pages/MemoryPage.tsx` | Route page (`/memory`) |
| `App.tsx` / `navigation/Sidebar.tsx` | Route + "Memory" nav item |

---

## Key Features Implemented

### 1. Execution Memory Store (`execution_memory` table)
- **Successful executions** captured at `ExecutionEngine::complete_execution`
- **Failed executions** captured at `fail_execution` (with the error and failed steps)
- **Cancelled executions** captured at `cancel_execution`
- **Planner reports** captured at `attach_planner_report` (outcome accounting: completed/replaced/replans)
- **Autonomous sessions** captured at every terminal state of the reason–act–observe loop (completed/failed/cancelled/rejected/budget-exhausted), including the reasoning stream
- Upsert keyed on `(kind, source_id)` so re-recording never duplicates history
- All captures are **best-effort**: a memory failure is logged and never affects the execution lifecycle

### 2. Semantic Retrieval (`retrieval.rs`)
- **Similar goals**: blended score = 0.6 × zero-centered embedding cosine + 0.4 × token overlap
- **Similar plans**: every row stores the executed `ExecutionPlan`, so a match retrieves the full workflow
- **Successful workflows**: retrieval filters on `status == success` and ranks by the learned blend
- Zero-centering is applied before cosine because the placeholder hash provider emits all-positive vectors (uncentered cosine of any two unrelated vectors ≈ 0.75, which would make all goals look similar)

### 3. Learning Engine (`learning.rs`)
- **Ranking**: `learned_score = 0.5·similarity + 0.3·goal-fingerprint success rate + 0.2·recency`
- **Recommendations**: `MemoryEngine::recommend()` returns the top successful workflows with replay counts
- **Avoid failed strategies**: `MemoryEngine::avoid()` surfaces failed/cancelled runs relevant to a goal with the failure reason
- **Learned workflows**: aggregates repeated goals into fingerprints with success/failure history and the best remembered plan

### 4. Planner Integration (`planner.rs`)
- `Planner::with_memory()` attaches the store (optional — `None` keeps the planner deterministic and backward compatible)
- `plan()` consults memory first: when a previous run achieved a sufficiently similar goal (`score ≥ 0.6`), the remembered workflow is **reused** — re-keyed (fresh ids), reset to pending, and annotated with `"Reused successful workflow from execution memory (score …)"`
- The reused memory row's `replay_count` is incremented so the learning engine weights frequently-reused workflows
- Memory failures degrade gracefully: planning falls back to the deterministic chain

### 5. AutonomousRuntime Integration (`autonomous/runtime.rs`)
- **Consult on reasoning**: before the first plan, the runtime reports how many similar runs memory holds (or that none exist); on every recovery it surfaces `"Memory: avoiding a previously failed strategy — …"`
- **Reuse successful workflows**: the planner (memory-attached) performs the actual reuse
- **Improve replanning**: the avoid consultation feeds the Replanning phase reasoning stream
- **Record sessions**: terminal sessions (completed, failed, cancelled, rejected, budget-exhausted) are captured via `capture_session()`

### 6. IPC + Frontend
- Five new commands: `memory_search`, `memory_recommend`, `memory_avoid`, `memory_learned_workflows`, `memory_stats` — thin wrappers, no business logic
- New `Memory` page (`/memory`) with stats, semantic search, recommendations, avoid list, recent memories, and learned workflows

---

## Backward Compatibility

- **No breaking IPC changes**: all new commands are additive; existing commands unchanged
- **Planner/engine/runtime changes are opt-in**: memory is an `Option` field; without it, behavior is byte-identical to RC-5 M6
- **Single new migration** (`0020`), additive table with `CREATE TABLE IF NOT EXISTS` semantics via sqlx's migration runner; existing data untouched
- Embeddings live in the memory table itself (BLOB), so no dependency on the `semantic_documents` schema
- Existing deterministic-plan tests still pass unchanged (empty memory ⇒ deterministic chain)

---

## Tests

### Backend (all passing)

| File | Tests | Status |
|------|-------|--------|
| `memory/models.rs` | 3 | ✅ |
| `memory/retrieval.rs` | 6 | ✅ |
| `memory/learning.rs` | 8 | ✅ |
| `memory/repository.rs` | 6 | ✅ |
| `memory/engine.rs` | 8 | ✅ |
| `planner.rs` (+4 integration) | plan reuses remembered workflow, weak matches ignored, completed/failed executions captured | ✅ |
| `autonomous/runtime.rs` (+1 integration) | completed session captured into memory | ✅ |

### Frontend (all passing)

| File | Tests | Status |
|------|-------|--------|
| `MemoryDashboard.test.tsx` | 4 (stats load, search IPC + ranked hits, recommend + avoid render, learned workflows) | ✅ |

**Totals: 36 new backend tests + 4 new frontend tests — all passing.**

---

## Gates

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | ✅ clean |
| `cargo clippy --all-targets` | ✅ 0 warnings |
| `cargo build` | ✅ |
| `cargo test` | ✅ 300 passed / 0 failed (+5 integration) |
| `npx tsc -b` | ✅ |
| `npm run build` | ✅ |
| `npm run test` (vitest) | ✅ 23 passed |
| `npm run lint` | ✅ no new errors (18 pre-existing, verified identical on the base commit) |

---

## Engineering Notes

- **Zero-centering discovery**: the placeholder `LocalEmbeddingProvider` emits all-positive vectors, so naive cosine similarity is ~0.75 for *any* two goals. Retrieval applies zero-centering before cosine — identical goals score 1.0, unrelated goals ~0.05 — and falls back to pure token overlap when no embedding is stored.
- **Single-writer discipline**: all memory writes happen at their component's natural lifecycle boundary (engine terminal states, report attach, runtime terminal states), so there is exactly one capture path per artifact.
- **No duplicated logic**: ranking, filtering, and similarity are pure functions over records, unit-tested without a database; the repository is the only SQL.
- Files kept under the 500-line guideline per project rule (largest new file: 470 lines).
