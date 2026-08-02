# ChronoDesk RC-5 Milestone 3 Engineering Report

**Release:** RC-5 (Release Candidate 5) — Milestone 3 — Execution Context & Variable Binding
**Milestone 2 Commit:** f3e02c0 (Planner → ExecutionEngine handoff)
**Date:** 2026-08-02
**Build Status:** ✅ All checks passing (250 unit + 5 integration + 1 doc-test)
**Scope:** 4 files, +261 / -12 lines (plus new `execution_context.rs` module), zero breaking changes, no schema/IPC/provider changes

---

## Executive Summary

Milestones 1 and 2 delivered goal → dependency-aware DAG planning and
Planner → ExecutionEngine handoff. Milestone 3 wires the execution-time data
flow between steps: a per-execution, in-memory `ExecutionContext` that stores
each completed step's **structured tool output**, resolves `{{...}}` template
references (`{{steps.<name>.<path>}}`, `{{workspace.id}}`, `{{goal}}`,
`{{results[0]}}`) **before a downstream tool is invoked**, and stores the
invocation result back so later steps can bind against it.

The engine alone owns this context — the planner still only plans and replans.
If a referenced output genuinely cannot exist, the engine fails fast and the
planner returns a **structured** `PlannerError::UnresolvedVariable` instead of
silently passing a bad value or spinning on an unrecoverable replan.

---

## Architecture Changes

### New: ExecutionContext (`copilot/execution_context.rs`)

The execution-scoped variable store, kept in
`ExecutionEngine::active_executions` and dropped when the execution finishes.
In-memory only — no schema, no migrations, no persistence.

- `new(workspace_id: Option<Uuid>, goal: String)` — seeds the shared
  `goal` and (when present) `workspace` (`{ id, workspace_id }`) variables.
- `set_variable(name, value)` — caller-scoped shared variables.
- `set_step_output(step: usize, name: Option<&str>, output: Value)` — records a
  completed step's result under its tool name **and** its step index, so
  `{{steps.<name>.<path>}}` works by name or by number.
- `resolve(&Value) -> Result<Value, ContextError>` — recursive substitution
  over the whole JSON argument object (scalars, arrays, nested objects).
- `resolve_string(&str)` — whole-template references return the bound value as
  JSON/scalar; embedded `{{...}}` in surrounding text interpolate the rendered
  text (`to_text`).
- `split_path` + `lookup` — dotted-path walking that expands `results[0]`
  array accesses into ordered segments.

**Error contract** — `ContextError` is structured and, crucially,
classifiable:

| Variant | Meaning |
|---|---|
| `Unresolved` | root variable (e.g. `{{unknown}}`) has no value |
| `MissingStep` | referenced step (`{{steps.<name>...}}`) never stored output |
| `MissingField { template, field }` | a field/index in a bound value does not exist |
| `Malformed` | template text is syntactically invalid (e.g. empty body) |

`ContextError.is_unresolved()` is true for the first three; `UNRESOLVED_VARIABLE_MARKER = "unresolved variable"` prefixes engine-failure messages so the planner can map them structurally.

### ExecutionEngine (`copilot/execution_engine.rs`)

- `ActiveExecutionState` gained `context: ExecutionContext`,
  seeded in `start_execution` from the plan's `workspace_id` + `goal`.
- `execute_next_step_impl` now, before invoking a tool:
  1. Snapshot the running execution's context.
  2. `context.resolve(...)` the step's arguments (default `{}`).
  3. **On `ContextError`** — mark the step `Failed` with the classified error
     message, record a `StepFailed` event, `fail_execution`, return early
     (no tool call is attempted with a half-bound value).
  4. **On `Ok`** — invoke the shared `ToolExecutor` with the resolved arguments.
- On `ToolInvocationSuccess` the structured `invocation.result` is stored via
  `set_step_output(step.step_number, step.tool_name.as_deref(), result)` so it
  is visible to downstream steps by name and index.
- The engine never generates plans and never loads external context; it only
  binds and stores during a run.

### Planner (`copilot/planner.rs`)

- New `PlannerError::UnresolvedVariable(String)` (`variable resolution failed: {0}`).
- Extracted the engine-drive loop into a public `execute_plan(&ExecutionPlan, token)`;
  `execute_goal` now builds the plan then delegates to it.
- On a failed step whose error starts with `UNRESOLVED_VARIABLE_MARKER` or
  `"invalid template"`, return `PlannerError::UnresolvedVariable` **instead of** replanning — a missing output can never appear in a new plan, so replanning would be a doomed loop.
- `bind_plan_arguments` now binds `{{workspace.id}}` (a template) for tools
  declaring a `workspace_id` parameter; the engine resolves it per-run.

Contract unchanged: planner plans/replans, engine schedules/invokes/persists,
shared `ToolExecutor` + permission pipeline reused — no second invocation path.

---

## Validation

| Check | Result |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ 250 unit + 5 integration + 1 doc-test |
| `cd frontend && npm run build` | ✅ |
| `cd frontend && npx tsc --noEmit` | ✅ |

### New Tests

`copilot/execution_context.rs` (7 unit):

| Test | Verifies |
|---|---|
| `stores_and_reads_step_outputs` | outputs stored by tool name and by index |
| `variable_substitution_returns_scalar` | `{{goal}}`, `{{workspace.id}}` bind |
| `nested_json_lookup_resolves` | `{{steps.search.results[0].path}}` walks JSON |
| `array_index_into_step_output` | `workspaces[0].id` expands correctly |
| `missing_variable_errors_are_structured` | missing step/field → classified `ContextError` |
| `embedded_template_substitutes_text` | `goal is {{goal}} today` interpolates |
| `resolves_whole_argument_objects` | full argument objects resolve field-by-field |

`copilot/planner.rs` (5 integration):

| Test | Verifies |
|---|---|
| `execution_context_stores_outputs` | end-to-end: list_workspaces → get_workspace binds upstream output; every step completes |
| `downstream_task_receives_resolved_arguments` | a later step receives a value bound from the earlier step |
| `missing_variable_returns_structured_planner_error` | unresolvable reference fails the step and surfaces as `UnresolvedVariable` |
| `malformed_template_returns_structured_planner_error` | recognized-but-invalid template body fails with a structured error, not a silent pass-through |
| `cancellation_still_propagates_with_context_binding` | cancelling a context-bound execution still transitions to `Cancelled` |

---

## Security Review

- Reuses the exact `ToolExecutor` + persistent `ToolPermissionService`; no new
  invocation paths, no widened permissions.
- Substitution is a pure data transform over stored tool outputs — no shell/JS
  `eval`, no untrusted code path.
- No secrets stored in context beyond what a tool already returns; context is
  in-memory, dropped at execution end.
- No schema / IPC / provider-trait changes — surface minimized.

---

## Production Readiness

- Zero breaking changes; every `#[tauri::command]` handler, `LLMProvider`
  trait, and DB schema untouched.
- Deterministic failure: missing/malformed references fail fast with classified
  errors instead of partial substitutions or infinite replan loops.
- Backward compatible: engine falls back to raw arguments when a step carries
  no templates, and to sequential order when executing a plans task-less.
- Verified full CI-equivalent locally across all six gates.

---

## Remaining RC-5 Roadmap

- Frontend surfacing of `PlannerReport`/execution progress (UX follow-up).
- Streaming progress emission for automatically executed plans (engine already
  records events; exposing live over IPC is an integration step).

---

## Final Acceptance

- ✅ No new abstractions / no callback framework / no circular ownership
- ✅ Engine does NOT generate plans; planner does NOT run steps
- ✅ Context is in-memory only — no schema/migrations; engine resolves before
  invoking, stores after completing
- ✅ Structured `PlannerError::UnresolvedVariable` on resolution failure
- ✅ 12 new tests (7 unit + 5 integration); entire suite green (250 + 5 + 1)
- ✅ fmt / clippy / build / test / frontend build / tsc — all six gates green