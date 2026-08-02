# RC-5 M4 Engineering Report: Durable Execution

> Milestone: RC-5 M4 — Durable execution with persistent checkpoints, context
> serialization, and pause/resume that survives application restart.

## Executive Summary

RC-5's execution engine previously kept the working state of an in-flight run
entirely in memory: a `HashMap<Uuid, ActiveExecutionState>` held the plan, the
paused steps, and the resolved `ExecutionContext`. A pause simply flipped the
row's status bit, and a restart orphaned the run — resuming a paused execution
after restarting the app was impossible, and completed steps could not be
distinguished from the scheduler's own bookkeeping after the engine process went
away.

M4 makes execution durable. After every completed (or skipped) step the engine
persists a checkpoint row containing the full `ExecutionPlan` (DAG,
dependencies, gates), the resolved `ExecutionContext` (shared variables +
step outputs), the current `ExecutionStatus`, and the completed/skipped/failed
step-number lists. A pause writes a checkpoint too. Terminal states
(`Completes/Cancelled/Failed`) delete the checkpoint. A brand-new engine can
then rebuild run state from the persisted checkpoint and resume exactly where
the old process left off — never re-running a step that already completed.

## Architecture Change

Responsibilities stay frozen from the RC-5 charter:

| Role | Held by | M4 change |
|------|---------|-----------|
| Planning, replanning, DAG gates | `Planner` | none |
| Scheduling, lifecycle, cancellation, context, tool invocation | `ExecutionEngine` | checkpoint save/load/delete, resume reconstruction |
| Tool dispatch (validation, permissions, retries) | `ToolExecutor` | none |
| Persistence | `ExecutionRepository` | `save_checkpoint`/`get_checkpoint`/`delete_checkpoint` |

No callbacks, no event bus, no actor system, no new execution paths were
introduced.

## Checkpoint Design

One row per execution in the new `plan_execution_checkpoints` table, UPSERTed
keyed on `execution_id` so at most one checkpoint survives per run.

```sql
CREATE TABLE IF NOT EXISTS plan_execution_checkpoints (
    execution_id    TEXT NOT NULL PRIMARY KEY,
    plan            TEXT NOT NULL,           -- serialized ExecutionPlan
    context         TEXT NOT NULL,           -- serialized ExecutionContext
    status          TEXT NOT NULL,           -- execution status at save time
    completed_steps TEXT NOT NULL,           -- JSON array of step numbers
    skipped_steps   TEXT NOT NULL,           -- JSON array of step numbers
    failed_steps    TEXT NOT NULL,           -- JSON array of step numbers
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (execution_id) REFERENCES plan_executions(id) ON DELETE CASCADE
);
```

| Field | Why it is required |
|-------|--------------------|
| `plan` | Persisting DB execution rows alone is insufficient for scheduler reconstruction — the engine needs the full DAG, dependency ordering, gates, and tool arguments to keep scheduling correctly after a restart. |
| `context` | Serialized `ExecutionContext` carries shared variables (`goal`, `workspace`, caller vars) and every consumed step output. Restoring a partial context would break `{{steps...}}` bindings and correctness. |
| `completed/skipped/failed` | Step-number lists match what the scheduler actually recorded, so resume never re-runs completed work. |
| `status` | Lets a restarted engine distinguish "was paused mid-run" from other recoverable states. |

Design decisions:

- **Write every step.** The checkpoint is the primary durability mechanism, and a
  single UPSERT per step is intentionally cheap (one row, JSON columns). A
  crash at any point still leaves the last completed step intact.
- **Checkpoint never carries a stale context**: the context object is frozen in
  the loop just before/after the step that mutates it, so the JSON written is
  always consistent with the persisted step table.
- **Terminal states delete the checkpoint.** Ephemeral once the run is done;
  the engine never keeps cruft rows around.
- Same-table the events CHECK constraint widened (below in Migration).

## Execution Context Serialization

`ExecutionContext` (and its `StepOutput`) now derive `Serialize`/`Deserialize`.
The round-trip is lossless for JSON numbers/strings/booleans/arrays/nested
objects, and a round-trip test asserts a nested object with an integer, a float,
a string, a bool, and an array survives byte-for-byte identical (no `1` →
`1.0`-type drift, no `true` → `"true"` coercion).

`ExecutionContext::new` is unaffected (still in-memory), so serialization is a
pure add-on layered between the engine and the repository.

## Resume Flow

```
resume_execution(execution_id):
  if active state exists  -> reuse it (not restart, fast path)
  else:
    checkpoint = repository.get_checkpoint(execution_id)?
    if none -> error (nothing durable to resume)
    rebuild ActiveExecutionState:
      plan    = checkpoint.plan
      tasks   = checkpoint.plan.tasks
      context = checkpoint.context
      token   = CancellationToken::new()
    record CheckpointLoaded event  (payload: completed/skipped/failed)
    update_execution_status(Running)
  record resumed event (+ audit trail), return Ok
```

`execute_until_complete` then re-runs its normal loop, which now begins each
iteration by re-reading the persisted status and returns `Ok(())` immediately
on `Completed/Cancelled/Paused/Failed`. `execute_next_step` keeps its existing
guards. Since the plan's DAG seeds scheduling and `completed` flags come back
with the checkpoint, resume simply continues where the old engine left short,
forward-only, never re-running a completed step.

## Scheduler Changes (`execute_until_complete`)

- Loop reads execution status before scheduling each step; terminal states
  short-circuit with `Ok(())` (pausing mid-run does not surface a lifecycle
  error).
- Active engine state now carries the `plan` (`plan: Option<ExecutionPlan>`) so
  the scheduler can consult dependencies/gates after reconstruction.
- Successful and skipped steps end with `save_checkpoint`.

## Database Migration

- `src-tauri/migrations/0018_execution_checkpoints.sql`: creates the
  checkpoints table + index; rebuilds `plan_execution_events` as
  `plan_execution_events_v18` with a widened CHECK constraint accepting the two
  new event types (`checkpoint_saved`, `checkpoint_loaded`), copies rows, drops
  the old table, and re-creates the indexes. (SQLite cannot ALTER a CHECK.)
- `CURRENT_SCHEMA_VERSION` 17 → 18.
- Existing columns, foreign keys, and the previous migrations are untouched;
  upgrading applies the delta and backfills nothing (new table starts empty).

## Event Changes

- Only new variants: `ExecutionEventType::CheckpointSaved` and
  `ExecutionEventType::CheckpointLoaded`, both reported in `Display`.
- The existing `paused` / `resumed` variants were already present in M5
  (pre-M-4 model) and are left untouched — no repurposing.

## Changes to Complete `RC-5` Remaining Roadmap

Users can now pause a run, quit the app, relaunch, and resume from where the
scheduler left off, with no repeated work. This is the durability backbone the
remaining Roadmap (M5 external indexing, M6 telemetry/analytics, ...) can
build on.

## Tests

Backend suite: 254 passed (was idem), 0 failed:

- `copilot::execution_checkpoint::tests::checkpoint_serializes_and_deserializes`
  — plan DAG + gates survive a JSON round-trip.
- `...::context_survives_checkpoint_round_trip_without_type_loss` — nested JSON
  values (int/float/string/bool/array) survive without type drift.
- `copilot::planner::tests::paused_execution_resumes_on_fresh_engine_without_repeating` —
  drives step 1, pauses, rebuilds a wholly new `ExecutionEngine` over the same
  pool (empty in-memory map = restarted app), resumes, and asserts both steps
  completed exactly once (2 steps total — no re-runs).
- `copilot::planner::tests::checkpoint_is_removed_after_terminal_state` — a
  checkpoint row exists mid-run and is deleted after completion.
- All prior RC-5 M4-relevant tests (events, binding, cancellation, gates, replan)
  continue to pass; the `get_progress` recent-events window was widened 10 → 50
  so the newer `checkpoint_saved`/`checkpoint_loaded`/`Started` events all appear.

## Security Review

- Checkpoints persist only `execution_id` + serialized plan/context JSON; no
  credentials or secrets are stored. Tool argument values in plans are already
  application data.
- Terminal cleanup removes checkpoint rows; no sensitive run state lingers
  after `Completed`/`Cancelled`/`Failed`.
- No new privilege boundary or tool surface. `get_checkpoint`/`save_checkpoint`
  are repository-internal and not exposed to the frontend's IPC surface.

## Production Readiness

- Checkpoints are idempotent (single row per execution, UPSERT semantics) and
  cheap (one row + one index); per-step saves scale linearly with steps already
  persisted for status anyway.
- Resume "no-op" branch: an open-run resume hit no checkpoint and no active
  state returns a clear error; a resume mid-run reuses the live state.
- Schema migration is forward-only and additive; version gate enforces a single
  upgrade path consistent with the previous milestones.
- Serialization of context/plan is explicit (serde JSON) — no version-coupled
  magic; a future M5 can add an explicit serialized version field if needed.

## Remaining Roadmap

- M5: Proactive resilience (replan on mid-run DAG drift, health-checked re-schedules).
- M6: Background worker sweep (google-cloud audit, test-gap analysis, map) on top of the checkpoint backbone.
- Post-RC-5: In-memory active-run rehydration from checkpoints at process start
  (currently resume-on-demand; an app-start sweep could list and re-enable
  paused runs automatically).