# RC-6 M3 Engineering Report — Adaptive Learning

**Date:** 2026-08-02
**Branch:** `main`

---

## Summary

RC-6 M3 transforms ChronoDesk from **remembering executions** into
**actually learning from them**. The execution-memory learning engine is
no longer a fixed-weight ranker: recommendation weights are **learned
from store history** (success, replay, user acceptance), every
recommendation carries a **confidence score with per-factor
explanations**, memories **age** (decay, freshness, archival weighting)
instead of being treated equally, **identical memories are merged**,
goals cluster into **reusable workflow families**, and the system
**detects failure patterns** (repeated failures, unstable workflows,
low-confidence plans). All of it is exposed as **learning health** over
IPC and visualized in the expanded Memory Dashboard.

The RC-6 charter is preserved: **no architecture rewrites, no duplicate
execution paths, no breaking IPC, no breaking database schema**. All new
tables are additive (migration `0022`); the planner only *consumes*
learning, the execution engine only *records* learning events, the
autonomous runtime only *consults* learning, and all learning code lives
inside `copilot::memory` (the M1 feedback/preference engine at the crate
root is untouched except for a thin IPC forward of recommendation
acceptance).

---

## Architecture

### What changed (additive only)

| Component | RC-6 M3 change |
|-----------|----------------|
| **`memory/learning/` (new module)** | `learning.rs` (497 lines) split into a module of pure rules: `core`, `weights`, `confidence`, `aging`, `duplicates`, `clustering`, `failures`, `stats` |
| `learning::weights` | adaptive `LearningWeights` — the fixed blend constants are replaced by weights learned from success rate, replay frequency, and acceptance rate (bounded deltas, renormalized to 1) |
| `learning::confidence` | Confidence Engine — `confidence_score` = similarity + success history + replay history + freshness + usage count, archival-scaled, with per-factor `ExplanationReason`s |
| `learning::aging` | freshness decay (30-day half-life), archival weight (0.3 past 180 days) applied to learned scores and confidence, aging buckets summary |
| `learning::duplicates` | identical-memory detection (goal fingerprint + status + steps + tools) and merge planning (best record survives) |
| `learning::clustering` | greedy single-link clustering of learned workflows into families by tool overlap (≥50%) or embedding cosine (≥0.65) |
| `learning::failures` | failure pattern detection: repeated failures, unstable workflows, low-confidence plans, with severity and goal scoping |
| `learning::stats` | `LearningHealth`: confidence averages, workflow quality, success trends (14 days), memory utilization |
| `MemoryEngine` | `recommend()` now returns confidence + explanations under adaptive weights; new facade: `record_acceptance`, `learning_health`, `failure_patterns(_for_goal)`, `workflow_families`, `duplicate_groups`, `merge_duplicates`, `aging_summary`, `acceptance` |
| `MemoryRepository` | `record_acceptance`, `acceptance_map`, `delete` (SQL for `memory_acceptance` + `execution_memory`) |
| `vector/` | `MemoryVectorRepository::remove_index`, `MemoryVectorSystem::remove` (durable row + in-memory k-NN entry) |
| `models.rs` | `MemoryRecommendation` + `confidence_score`/`explanation`; `MemoryOutcome.duration_seconds`; `MemoryAcceptance` ledger type; `outcome_from_report` extended |
| ExecutionEngine | `capture_memory` computes wall-clock `duration_seconds` from the execution row and passes it to `record_execution` |
| Planner | consumes confidence: reused-plan reasoning/log cite `confidence_score` |
| AutonomousRuntime | consults `failure_patterns_for_goal` (advisory reasoning event before planning) |
| IPC | 7 new thin commands (`memory_recommendation_feedback`, `memory_learning_health`, `memory_failure_patterns`, `memory_workflow_families`, `memory_aging_summary`, `memory_duplicate_groups`, `memory_merge_duplicates`); `submit_feedback` forwards recommendation acceptance to the ledger |
| Frontend | Memory dashboard gains Learning health card (confidence stats, workflow quality, utilization, 14-day success trend chart), Memory aging card, Failure patterns card, Workflow families card (confidence bars), Duplicate memories card (merge action), and recommendation cards with confidence badges, per-factor explanations, and Accept/Reject feedback |

### Data flow (M3)

```
capture (engine / runtime terminal state)
   │  record_execution(…, duration_seconds)     ← completion time tracked
   ▼
execution_memory rows + memory_acceptance ledger  ← user acceptance
   │                                                (Accept/Reject in UI,
   │                                                 submit_feedback forward)
   ▼
learning rules (pure, no DB):
   learn_weights(history) ──► rank_historical ──► score (archival-scaled)
   confidence_score(5 factors + explanations) ──► confidence_score
   aging buckets ──► aging_summary
   duplicate_groups ──► merge_plan ──► repository.delete + vectors.remove
   workflow_families / failure_patterns / learning_health
   ▼
IPC (thin) ──► Memory Dashboard (confidence charts, aging viz, health)
```

---

## Deliverables

### Migration

| File | Lines | Description |
|------|-------|-------------|
| `migrations/0022_memory_learning.sql` | 15 | `memory_acceptance` (memory_id PK → execution_memory ON DELETE CASCADE, accepted_count, rejected_count, first/last_feedback_at). Additive; `CURRENT_SCHEMA_VERSION` bumped 21 → 22 |

### Backend (src-tauri/src/copilot/memory/)

| File | Lines | Description |
|------|-------|-------------|
| `learning/mod.rs` | 46 | module map + re-exports; tests moved to `tests.rs` |
| `learning/core.rs` | 238 | thresholds, fingerprint success rate, replay/duration factors, `rank_historical` / `learned_score` (weights + acceptance + archival), `learned_workflows`, `avoid_strategies`, `compute_stats` |
| `learning/weights.rs` | 246 | `LearningWeights` (7 factors), `default_weights`, `learn_weights` (bounded adaptive shifts + renormalization) + 4 tests |
| `learning/confidence.rs` | 230 | Confidence Engine: 5-factor blend, archival scaling, per-factor `ExplanationReason`s + 3 tests |
| `learning/aging.rs` | 196 | `freshness`, `archival_weight`, `aging_factor`, `aging_summary` (fresh/aging/archived buckets) + 3 tests |
| `learning/duplicates.rs` | 230 | identical-memory grouping, best-record keeper, `merge_plan`, `MergeResult` + 3 tests |
| `learning/clustering.rs` | 294 | greedy single-link families over tool overlap / embedding cosine, family aggregates + 2 tests |
| `learning/failures.rs` | 420 | repeated-failure / unstable-workflow / low-confidence-plan detection with severity + goal-scoped variant + 6 tests |
| `learning/stats.rs` | 376 | `LearningHealth`, workflow quality, 14-day success trends, memory utilization + 4 tests |
| `learning/tests.rs` | 422 | adapted M1/M2 core suite (new signatures) + 4 new core tests |
| `engine.rs` | ~640 | capture signatures, confidence-bearing `recommend`, 8 new facade methods |
| `engine_tests.rs` | ~690 | +6 M3 facade tests (acceptance adapts recommendations, confidence explanations + duration, learning health, duplicate merge compaction, failure patterns + aging + families) |
| `repository.rs` | ~740 | `record_acceptance`, `acceptance_map`, `delete` + acceptance/delete test |
| `models.rs` | ~420 | `MemoryAcceptance`, recommendation confidence fields, `duration_seconds` |
| `vector/repository.rs` | ~480 | `remove_index` + cascade test |
| `vector/mod.rs` | ~190 | `MemoryVectorSystem::remove` |

(Engine/repository files exceed 500 lines — they carry the pre-existing
M1/M2 methods; every *new* M3 file is well under the guideline.)

### IPC (src-tauri/src/commands/ + lib.rs)

- `memory_recommendation_feedback(memory_id, accepted)` — writes the acceptance ledger.
- `memory_learning_health()` → `LearningHealth` (confidence averages, workflow quality, success trends, utilization).
- `memory_failure_patterns()` → `Vec<FailurePattern>`.
- `memory_workflow_families()` → `Vec<WorkflowFamily>`.
- `memory_aging_summary()` → `MemoryAgingSummary`.
- `memory_duplicate_groups()` → `Vec<DuplicateGroup>`.
- `memory_merge_duplicates()` → `MergeResult`.
- `submit_feedback` (M1 command) now also takes `Arc<MemoryEngine>` and forwards `Accepted`/`Helpful` on `Recommendation` targets to the acceptance ledger — thin wiring, no business logic in IPC.

### Frontend (frontend/src/)

| File | Description |
|------|-------------|
| `types/memory.ts` | `MemoryRecommendation.confidence_score`/`explanation`, `MemoryOutcome.duration_seconds`, + `SuccessTrend`, `WorkflowQuality`, `MemoryUtilization`, `LearningHealth`, `FailurePattern(Type)`, `WorkflowFamily`, `MemoryAgingSummary`, `DuplicateGroup`, `MergeResult`, `RecommendationExplanation` |
| `services/memoryRepository.ts` | +7 IPC bindings |
| `features/memory/components/LearningHealthCard.tsx` | confidence stats, workflow quality grid, utilization bars, 14-day success trend chart |
| `features/memory/components/MemoryAgingCard.tsx` | fresh/aging/archived bucket bar + counts |
| `features/memory/components/FailurePatternsCard.tsx` | typed pattern list with severity + occurrences |
| `features/memory/components/WorkflowFamiliesCard.tsx` | families with shared tools + confidence bars |
| `features/memory/components/DuplicateGroupsCard.tsx` | duplicate groups + merge action |
| `features/memory/MemoryDashboard.tsx` | composes all cards; recommendation cards show confidence badge, per-factor explanations, Accept/Reject feedback |
| `features/memory/MemoryDashboard.test.tsx` | +6 tests (health, aging, failure patterns, families, feedback, merge) |

---

## Key Features Implemented

### 1. Workflow success learning
`ExecutionMemoryRecord` outcomes now carry `duration_seconds` (execution
engine computes it from the run's `started_at`/`completed_at`;
autonomous sessions from `created_at`/`updated_at`), and the acceptance
ledger records how often each recommended workflow was **accepted by the
user**. Success rate, retry counts (`outcome.retries_used`), completion
time, and replay frequency (`replay_count`) feed every downstream rule.

### 2. Adaptive recommendation weighting
`learn_weights` reads the store: a high acceptance rate raises the
acceptance weight; frequent replays raise replay weight and lower
recency weight; a shaky success rate raises the failure weight; a
strong success rate raises the success weight. Each shift is bounded
(≤0.08 per signal) and the blend renormalizes to 1, so early history
stays neutral and later history adapts safely.

### 3. User preference learning
Recommendation acceptance/rejection is captured per memory record
(`memory_acceptance`), surfaced through Accept/Reject in the dashboard
and through the M1 `submit_feedback` pipeline (preferred tools/
workflows/workspaces already flow through the M1 preference engine).

### 4. Confidence Engine
Every `MemoryRecommendation` exposes `confidence_score` (0..1) built
from similarity, fingerprint success history, replay history,
freshness, and usage count, scaled by archival weight, with a
`Vec<ExplanationReason>` — the dashboard renders each factor and its
impact (▲/▼).

### 5. Memory aging
`freshness` (30-day half-life exponential decay) replaces the fixed
recency factor; `archival_weight` (0.3 past 180 days) scales learned
scores and confidence so aged knowledge fades from recommendations
without being deleted; `aging_summary` buckets the store for the
dashboard.

### 6. Duplicate memory detection
Identical memories (same normalized goal, status, step sequence, tool
list) are grouped; the best record (most completed steps, then most
replays, then newest) survives; `merge_duplicates` deletes the rest
including their vector-index entries (durable + in-memory, cascading
FKs for the SQL rows).

### 7. Workflow clustering
Greedy single-link clustering groups learned workflows into families
when they share ≥50% of tools (or embed within 0.65 cosine when no
tools are recorded), with per-family shared tools, success/failure
totals, average duration, and average plan confidence.

### 8. Failure pattern detection
`failure_patterns` surfaces repeated failures (≥2 failures, more than
successes, recent), unstable workflows (≥3 samples, success rate <
0.5), and low-confidence plans (plans under 0.4 confidence, repeated);
`failure_patterns_for_goal` gives the autonomous runtime an advisory
signal before it trusts a remembered plan.

### 9. Learning statistics
`memory_learning_health` exposes confidence averages (overall and for
successful memories), acceptance rate, workflow quality (count, avg
success rate, avg plan confidence, avg duration, replay adoption,
replays per run), 14-day success trends, and memory utilization
(active share, avg freshness, workflows per record).

### 10. Frontend
The Memory Dashboard now shows learning health with confidence stats
and a success-trend chart, memory aging visualization, failure
patterns, workflow families with confidence bars, duplicate memories
with a merge action, and recommendation explanations with Accept/Reject
feedback that re-ranks live.

---

## Backward Compatibility

- **No breaking IPC**: 7 additive commands; `submit_feedback` gains a
  managed-state argument only (frontend callers unchanged).
- **Additive schema**: migration `0022`; existing tables untouched.
- **Existing tests pass unchanged**: M1/M2 planner/runtime/engine tests
  still pass (they were adapted only for the new `record_execution`
  duration parameter, which is `Option<u64>`); without acceptance or
  history, weights stay at defaults and behavior matches M2.
- Deterministic-plan tests remain green — memory stays optional and
  advisory (`Option<Arc<MemoryEngine>>`).

---

## Tests

### Backend (all passing)

| File | New tests | Highlights |
|------|-----------|------------|
| `learning/weights.rs` | 4 | default normalization, thin-history neutrality, acceptance raises acceptance weight, replay raises replay / lowers recency, failure history raises failure weight |
| `learning/confidence.rs` | 3 | 5-factor blend + explanations, replay rewarded / age punished + archival factor, clean vs failing history |
| `learning/aging.rs` | 3 | exponential decay, archival flip at 180 days, bucket summary |
| `learning/duplicates.rs` | 3 | identical grouping / single runs excluded, best record survives, empty plan |
| `learning/clustering.rs` | 2 | tool-based families, empty store |
| `learning/failures.rs` | 6 | repeated failures, old failures excluded, unstable workflow samples, low-confidence plans (avg), goal scoping, pattern counts |
| `learning/stats.rs` | 4 | confidence averages, workflow quality, 14-day trends, empty store |
| `learning/tests.rs` | 11 (4 new) | adaptive weights in blend, archival scaling, acceptance ledger ranking, replay-vs-freshness balance |
| `engine_tests.rs` | +6 | acceptance adapts recommendations, confidence explanations + duration, learning health, duplicate merge compaction, failure patterns + aging + families |
| `repository.rs` | +1 | acceptance ledger tallies + cascade delete |
| `vector/repository.rs` | +1 | `remove_index` drops only the requested memory |

**Totals: 382 passing (376 lib + 5 integration + 1 doc), 0 failed** —
M2 was 346; **+36 new tests**.

### Frontend (all passing)

| File | Tests | Status |
|------|-------|--------|
| `MemoryDashboard.test.tsx` | 13 (6 new: learning health, aging, failure patterns, families, feedback, duplicate merge) | ✅ |

**Totals: 31 frontend tests, all passing (25 M1/M2 + 6 new).**

---

## Gates

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ clean |
| `cargo clippy --all-targets -- -D warnings` | ✅ 0 warnings |
| `cargo build` | ✅ |
| `cargo test` | ✅ 382 passed / 0 failed |
| `npx tsc -b` | ✅ |
| `npm run build` | ✅ |
| `npm test` (vitest) | ✅ 31 passed |

---

## Engineering Notes

- **Learning code stays inside MemoryEngine**: every new rule lives in
  `copilot::memory::learning` as pure functions over
  `ExecutionMemoryRecord`s — the planner only consumes, the execution
  engine only records, the autonomous runtime only consults.
- **Repository owns SQL**: acceptance and deletion SQL live in
  `MemoryRepository` / `MemoryVectorRepository`; the learning rules
  never touch the database.
- **No duplicated logic**: decay math lives once in `aging` (the old
  `recency_factor` moved there), blob/text helpers stay shared in
  `models.rs`, explanation reasons reuse the M1 `ExplanationReason`
  type, and `compute_stats`/`learned_workflows` are reused by the new
  stats and clustering rules instead of being re-implemented.
- **Single writer discipline preserved**: the indexer still owns
  embeddings; the engine owns rows; `merge_duplicates` removes the
  durable index row + in-memory k-NN entry through one path
  (`MemoryVectorSystem::remove`), and FKs cascade the rest.
- **Adaptive but stable**: weight shifts are bounded (±0.08/signal) and
  neutral until ≥3 records exist, so early stores behave like M2 while
  mature stores genuinely adapt.
- **Advisory only**: failure-pattern warnings and confidence scores
  never block planning or execution — they inform reasoning and the
  dashboard.
