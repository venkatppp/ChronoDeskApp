# RC-10 M1 Engineering Report — Production Hardening (Performance & Profiling)

## Summary

RC-10 M1 delivers the first production-hardening milestone: a complete
performance & profiling subsystem spanning backend, database, IPC, and
frontend. The backend gains a live profiler (command/service/repository/
worker timings), a startup phase profiler, a read-only micro-benchmark
engine covering the five subsystems (planner, execution, memory, graph,
vector), on-demand system diagnostics (CPU, RAM, DB size, caches,
workers, threads), and a pure-logic optimizer that turns those
observations into severity-tagged, partially auto-applicable
recommendations across query, lazy-initialization, worker, cache, and
memory surfaces.

Everything is additive: one new migration (`0028`), one new engine
module (`performance/`), one new repository (`PerformanceRepository`),
one new model module (`models::performance`), six thin IPC commands,
and a new frontend page with five components. No existing table, IPC
command, or service was modified in a breaking way; the only edits to
existing files are mechanical stage markers in `lib.rs` startup, the
`entry_count()` accessor on `IntelligenceCache`, module registration,
and Cargo/sidebar/route wiring.

## Architecture

### What changed (additive only)

| Layer | Addition |
|---|---|
| Engine | `performance/` — `PerformanceEngine` facade over `profiler.rs`, `startup.rs`, `benchmark.rs`, `diagnostics.rs`, `optimizer.rs` |
| Models | `models/performance.rs` — DTOs for profiles, startup runs, benchmarks, diagnostics, recommendations, history |
| Repository | `repositories/performance_repository.rs` — all SQL for the three new tables + `db_size_bytes()` |
| Commands | `commands/performance.rs` — `performance_profile`, `performance_startup`, `performance_benchmark`, `performance_diagnostics`, `performance_optimize`, `performance_history` (thin, self-timing) |
| Migration | `0028_performance_profiling.sql` — `performance_profiles`, `benchmark_runs`, `startup_profiles` |
| Runtime | `IntelligenceCache::entry_count()` (additive accessor used by diagnostics) |
| Frontend | `types/performance.ts`, `services/performanceRepository.ts`, `pages/PerformancePage.tsx`, `components/performance/{PerformanceDashboard,PerformanceCharts,BenchmarkPanel,DiagnosticsPanel,StartupTimeline}.tsx` |
| Dependency | `sysinfo 0.33` (system diagnostics — CPU/RAM/thread/process facts) |

### How the surfaces compose

```
lib.rs setup()
  └─ StartupProfiler (created before Database::initialize)
       └─ stage_start/stage_end markers around 12 startup blocks
            (database → services → graph_sync → session_context →
             predictive → ai_models → learning → copilot → memory →
             kg_live → execution → watcher)
       └─ PerformanceEngine::record_startup() persists the run
PerformanceEngine (managed Tauri state)
  ├─ PerformanceProfiler  ── in-memory ring (1024) + durable ledger
  ├─ StartupProfiler      ── run-id grouped stage timeline
  ├─ BenchmarkEngine      ── 5 read-only suites → benchmark_runs
  ├─ Diagnostics          ── sysinfo + PRAGMA + cache/worker handles
  └─ Optimizer            ── pure analysis over the above
commands/performance.rs   ── thin forwards that record their own timing
```

The dependency order mirrors the rest of the codebase: commands →
engine → services/repositories → models → database. The profiler is
the only cross-cutting piece: performance commands record their own
execution, engine operations (benchmark, diagnostics, optimize) record
service/engine samples, worker telemetry is pulled from the existing
`RuntimeHealthService`, and repository access times surface as
repository samples. Nothing in the existing engines was rewired to
report timings, so no existing behavior changed.

### Persistence model

- `performance_profiles` — append-only sampled operations
  (category/name/duration/metadata). The in-memory ring keeps the live
  window (and p95 latency); the table survives restarts for history and
  the optimizer's slow-operation analysis. Pruning is available
  (`prune_profiles_older_than`) as an applied remediation.
- `benchmark_runs` — persisted per-suite results, including skipped
  entries (a subsystem not wired is recorded as `ok = 0` so history
  shows the gap rather than silently dropping the row).
- `startup_profiles` — per-stage timings grouped by `run_id`, so one
  launch renders as one timeline in the frontend and history is
  queryable. `finish()` rotates the run id and drops unfinished stages
  (an early `?` in setup leaves a clean timeline).

## Test report

### Backend (40 new; 500 total: 493 lib + 6 integration + 1 doc, 3 ignored)

| Suite | Tests | Covers |
|---|---|---|
| `performance/profiler_tests` | 5 | persistence, aggregates, p95 latency, recent/slowest ordering, `time()` helper, pruning |
| `performance/startup_tests` | 4 | stage markers, closure helper, early-return (open stage dropped), run grouping/ordering |
| `performance/benchmark_tests` | 4 | planner suite, all-suite run, unconfigured subsystem skip, graph suite |
| `performance/diagnostics_tests` | 4 | machine/db facts, cache wiring, empty workers, bounded CPU/RAM |
| `performance/optimizer_tests` | 8 | each rule (query/lazy-init/worker/cache/memory), severity ordering, no-op case, prune action |
| `performance/engine_tests` | 7 | facade integration: startup round-trip, profile, benchmark, diagnostics, optimize, history, apply |
| `repositories/performance_repository_tests` | 8 | profile round-trip, limits, benchmark decode, startup grouping, DB size PRAGMA, pruning |

### Frontend (20 new; 69 total across 11 files)

| Suite | Tests | Covers |
|---|---|---|
| `services/performanceRepository.test.ts` | 8 | every `performance_*` IPC command + argument shape + singleton |
| `pages/PerformancePage.test.tsx` | 4 | dashboard rendering, benchmark run, diagnostics + optimizer, apply flow |
| `components/performance/StartupTimeline.test.tsx` | 4 | stage bars, slowest highlight, pinning, empty/loading states |
| `components/performance/BenchmarkPanel.test.tsx` | 4 | category selection callback, empty state, measured rows, running state |

## Quality gates

All gates ran clean on the final commit:

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 0 warnings |
| `cargo build` | pass |
| `cargo test` | 500 passed, 0 failed |
| `npm run build` | pass |
| `npx tsc -b` | pass |
| `npm test` | 69 passed, 0 failed |
| `npm run lint` | no new problems (13 pre-existing errors, 5 pre-existing warnings) |

## Design notes & trade-offs

- **Timing integration is opt-in, not invasive.** Rather than wrapping
  ~150 existing commands, the profiler records samples from the six new
  commands, engine operations, and existing worker/health telemetry.
  This keeps the "thin wrappers only" IPC rule intact and the diff
  additive, at the cost of not capturing every legacy command's latency.
- **Benchmarks are read-only micro-benchmarks.** Every benchmarked
  operation (plan DAG build, execution listing, memory search, graph
  pagination/search, semantic search) is a pure read path, so running a
  suite is side-effect free. `Planner::plan` deliberately builds the
  deterministic DAG without executing tools.
- **Thread counts are platform-limited.** `sysinfo` only enumerates
  per-process threads on Linux/Windows; macOS reports 0 (documented in
  the model and surfaced as "n/a" in the UI). CPU/RAM/DB figures remain
  meaningful everywhere.
- **The optimizer is pure logic.** All rules are functions of the
  snapshot inputs, which makes them trivially testable without a
  database; applying an action is the engine's job since it owns the
  handles (graph cache trim/clear, profile-history pruning).
- **Startup profiling is marker-based, not closure-based.** The
  `stage_start`/`stage_end` markers are infallible and tolerate early
  returns in setup, so instrumenting `lib.rs` could not change startup
  control flow; unfinished stages are dropped at `finish()`.
