# ChronoDesk RC-5 Milestone 2 Engineering Report

**Release:** RC-5 (Release Candidate 5) — Milestone 2 — Planner → ExecutionEngine Handoff
**Milestone 1 Commit:** 7206a4a (Autonomous Planning Engine — `Planner`)
**Date:** 2026-08-02
**Build Status:** ✅ All checks passing (238 unit + 5 integration + 1 doc-test)
**Scope:** 3 files, +462 / -147 lines, zero breaking changes, no schema/IPC/provider changes

---

## Executive Summary

Milestone 1 introduced the standalone `Planner` (goal → dependency-aware DAG
plan with conditional gates, bounded replanning). Milestone 2 closes the
loop: the `Planner` now **creates** the `ExecutionPlan` and **hands it to the
`ExecutionEngine`**, which schedules and executes the DAG step-by-step with
lifecycle, cancellation, persistence, and streaming progress events. The
planner keeps every planning/replanning decision; the engine never generates
plans.

The duplicated planner-private execution path (`invoke_task`,
`tool_is_denied`, `empty_result`, `bind_arguments`) is deleted. Planning and
execution are now cleanly separated — one DAG scheduler, one tool-invocation
pipeline, one set of persisted steps.

---

## Architecture Changes

### Planner (`copilot/planner.rs`)

- `Planner` gained an optional `Arc<ExecutionEngine>`:
  - `with_execution_engine(engine)` builder attach.
  - `engine()` accessor returning a clear error when unattached.
- `execute_goal` now loops:
  1. `plan(workspace_id, token, goal)` — build the DAG.
  2. `engine.start_execution(&plan, None)` — persist plan + steps, get `execution_id`.
  3. `engine.execute_until_complete(execution_id)` — drive the scheduler to completion.
  4. `engine.get_progress(execution_id)` — read step outcomes + events.
  5. On `Completed`: collect completed/skipped task ids from progress.
     On `Cancelled`: return `PlannerError::Cancelled`. Otherwise find the first
     `StepStatus::Failed` step, record it in `replaced`, `replan_count += 1`,
     and `replan_after_failure` (bounded by `MAX_REPLAN_ATTEMPTS`).
- `PlannerReport` gained `execution_id: Option<Uuid>` (final execution id).
- Duplicate invocation infrastructure removed: `invoke_task`, `tool_is_denied`,
  `empty_result`, `bind_arguments` are gone. The plan's step arguments are now
  derived from the tool registry via `bind_plan_arguments`: tools declaring a
  `workspace_id` parameter receive the planner's workspace context; other tools
  run with no arguments (`{}`).
- Plan DAG semantics are unchanged (dependencies + `PlanGate`).

### ExecutionEngine (`copilot/execution_engine.rs`)

- `ActiveExecutionState` now stores `tasks: Option<Vec<PlanTask>>` populated
  from `plan.tasks` in `start_execution` (aligned with persisted steps by index).
- All execution logic goes through the shared `ToolExecutor` + permission
  pipeline; **no plan generation exists in the engine**.
- `execute_next_step_impl` is now DAG-driven: see `next_runnable_step_index`
  below.
- Missing-argument steps with a `tool_name` are invoked with `{}` instead of
  being silently skipped (previously `arguments: None` caused a skip).

---

## Scheduler Design (`next_runnable_step_index`)

The scheduler is a DAG-walker over the persisted steps aligned with the
plan's tasks by index. For each index whose step is still `Pending` it checks:

1. **Dependency gate** — every referenced task id is `Completed` or `Skipped`.
2. **Conditional gate** — `PlanGate`:
   - `AfterSuccess(predecessor)` requires the predecessor's outcome to be
     `Success`.
   - `AfterFailure(predecessor)` requires `Failed`.
3. Returns the first index satisfying both, i.e. the *next runnable step*.

If nothing is runnable:
- Any `Pending` step left → `fail_execution("no runnable step: unsatisfied
  dependencies or conditional gates")` — the planner then observes it and
  replans.
- No steps pending → `complete_execution`.

When no plan is attached (`steps` created outside the planner, `tasks: None`),
the scheduler falls back to the previous sequential order
(`execution.current_step`), preserving backward compatibility with all
existing callers.

## Planner ↔ Engine Contract

| Concern | Owner |
|---|---|
| Goal → DAG + gates + estimated duration | `Planner` |
| Replanning after a failed step (bounded) | `Planner` |
| Step scheduling (dependencies + gates) | `ExecutionEngine` |
| Lifecycle (running/completed/cancelled/failed) | `ExecutionEngine` |
| Persistence of plans/steps/executions/events | `ExecutionEngine` (via `ExecutionRepository`) |
| Cancellation + streaming progress events | `ExecutionEngine` |
| Tool invocation/validation/permission | shared `ToolExecutor` + `ToolPermissionService` |

No callbacks, no `Replanner` traits, no circular ownership, no second
invocation path.

---

## Validation

| Check | Result |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 238 unit + 5 integration + 1 doc-test |
| `cd frontend && npm run build` | ✅ |
| `cd frontend && npx tsc --noEmit` | ✅ |

### New Tests (`copilot/planner.rs`)

| Test | Verifies |
|---|---|
| `planner_hands_plan_to_engine_and_completes` | end-to-end: goal → plan → engine → completed plan report, all tasks completed, no replaced, `ExecutionStatus::Completed` |
| `engine_executes_tasks_in_dependency_order` | engine runs the DAG (each task after its references), all steps `Completed` |
| `engine_cancellation_propagates` | cancelling the execution transitions to `Cancelled` |
| `execution_progress_events_are_recorded` | `Start`ed, `StepStart`ed, `Completed` progress events exist |
| `failure_triggers_replan_against_engine` | denying a tool (runtime policy) fails the plan, planner replans (≥1 replan) |

---

## Security Review

- Reuses the exact `ToolExecutor` + persistent `ToolPermissionService`.
- Planner no longer has its own invocation/is-permitted checks; the engine
  enforces registry + runtime policy once.
- No new secrets, no network exposure changes, no widened permissions.
- No schema / IPC / provider-trait changes — surface minimized.

---

## Production Readiness

- Zero breaking changes; every `#[tauri::command]` handler, `LLMProvider`
  trait, and DB schema untouched.
- Deterministic termination: all-gray `Completed`, `Cancelled` on cancel,
  bounded replan attempts on failure.
- Backward compatible: `next_runnable_step_index` falls back to sequential
  order when executing a plan without tasks.
- Verified full CI-equivalent locally across all six gates.

---

## Remaining RC-5 Roadmap

- Plan step argument binding beyond workspace context (e.g. step-output →
  downstream-argument interpolation in `bind_plan_arguments`).
- Frontend surfacing of `PlannerReport`/execution progress (UX follow-up).
- Streaming progress emission for automatically executed plans (engine already
  records events; exposing live over IPC is an integration step).

---

## Final Acceptance

- ✅ No new abstractions / no callback framework / no circular ownership
- ✅ Engine does NOT generate plans; planner does NOT run steps
- ✅ Shared `ToolExecutor` + permission services reused — no duplicate path
- ✅ 5 new tests; entire suite green (238 + 5 + 1)
- ✅ fmt / clippy / build / test / frontend build / tsc — all six gates green