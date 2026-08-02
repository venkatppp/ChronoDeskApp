# RC-5 M5 Engineering Report: Live Execution Progress Streaming & Dashboard

> Milestone: RC-5 M5 — Stream live `execution:progress` snapshots from the
> execution engine to a new frontend Execution Dashboard (DAG progress, current
> running task, completed/failed/skipped tasks, planner report, controls,
> timeline), reusing the existing `app_events`/event infrastructure.

## Executive Summary

Through M4 the backend recorded execution progress durably *to the database*
(events, checkpoints), but nothing reached the user in real time: the frontend
had no way to watch a run besides polling `execution_get_progress`, and the
planner's final `PlannerReport` (what completed, what was replaced by replanning,
how many replans ran) was never surfaced anywhere in the UI.

M5 connects the two with the event pipeline that already existed. The
`ExecutionEngine` now carries an `AppEventEmitter` (the same trait `ToolExecutor`
already emits tool progress through) and, after every state change — start,
step-started, step-completed/skipped/failed, pause, resume, cancel, terminal —
builds a full `ExecutionProgress` snapshot and streams it to the frontend as an
`execution:progress` event. The `planner_report` travels inside that same
snapshot, persisted in a new durable `plan_execution_reports` table so a later
reconnect/restart still shows the run summary. A new `execution_list_recent`
IPC lets the dashboard re-attach on reload. The frontend gains a typed
execution domain (types, repository, `useExecutionStream` hook) and a full
Execution Dashboard, wired into routing and the sidebar.

Responsibilities remain frozen from the RC-5 charter: Planner plans/replans and
reports; ExecutionEngine drives lifecycle/scheduling/progress publication;
ExecutionContext resolves variables; ToolExecutor handles single invocations.
The only planner change is that its final report is attached to the engine's
stream (report serialization — no new planning logic).

## Architecture Change

| Role | Held by | M5 change |
|------|---------|-----------|
| Planning, replanning, DAG gates | `Planner` | attaches its final `PlannerReport` to the engine's stream (report serialization only) |
| Scheduling, lifecycle, cancellation, context, progress publication | `ExecutionEngine` | `event_emitter` + `publish_progress` on every state change; `attach_planner_report` persists+streams the report |
| Tool dispatch | `ToolExecutor` | none |
| Persistence | `ExecutionRepository` | `save_planner_report`/`get_planner_report`/`list_recent_executions` |
| Frontend live view | Execution Dashboard (new) | stream subscription + reconnect restore + controls |

No polling. No duplicate event system. Reconnect = `execution_get_progress`
(fetch current snapshot) then re-subscribe to `execution:progress` — the exact
pattern desired: on reload the dashboard restores state from a server snapshot,
then goes live on the stream.

## Streaming Architecture

The engine already had the gather machinery:

```
get_progress(execution_id)  → step table + events(50) → ExecutionProgress
```

M5 adds `publish_progress(execution_id)`, which calls `get_progress` and, only
when an emitter is attached, does `emit(EVENT_EXECUTION_PROGRESS, &progress)`
via the existing `app_events::emit` helper (best-effort; serialization failure
logs, never fails the run). Emit points:

| Point | Why |
|-------|-----|
| `start_execution` | dashboard sees status flip to `running` immediately |
| StepStarted event | current running task becomes visible |
| StepCompleted (success / skip branches) | progress % advances, DAG flips to completed |
| `pause_execution` / `resume_execution` | controls state stays in sync |
| `cancel_execution` / `complete_execution` / `fail_execution` | terminal snapshot emitted just before active state is dropped, so the DAG is visible through the final state |

The emitter is an `Option<Arc<dyn AppEventEmitter>>` on the engine (mirroring
`ToolExecutor`), wired in `lib.rs` with the real `AppHandle`. Tests inject a
`RecordingEmitter` instead. `publish_progress` is a no-op without an emitter,
so headless/embedded use is unaffected.

## Planner Report Streaming

`ExecutionProgress` gains a new optional `planner_report: Option<PlannerReport>`
field (plus a new optional `plan: Option<ExecutionPlan>` so the frontend can
render the DAG even when querying a completed run). `PlannerReport` already
derived `Serialize`/`Deserialize`; no schema drift.

When a planner-driven run finishes, `Planner::execute_plan` builds its final
report and calls `engine.attach_planner_report(execution_id, report)`. The
engine:

1. persists the report (`save_planner_report`) so reconnect/restart can restore, then
2. re-runs `publish_progress`, streaming a snapshot that carries the report.

`get_progress` reads reports from the in-memory store first, then falls back to
the durable `plan_execution_reports` table (post-restart), so a reconnect to a
previously-completed run still shows the summary.

## Execution Session + Reconnect

The frontend `useExecutionStream(executionId)` hook:

1. On mount, `execution_get_progress` restores current state (reconnect).
2. Subscribes to `execution:progress`, filtering payloads by `execution_id`.
3. Returns a `refresh` that re-fetches — used by the dashboard's pause/resume/
   cancel buttons (IPC action, then restore).

`execution_list_recent(limit)` (new IPC) returns up to `limit` `ExecutionProgress`
snapshots for the most recently-updated executions, so the Execution page can
offer a picker of in-flight/last-completed runs after a reload.

## Database Migration

- `src-tauri/migrations/0019_planner_reports.sql`: new `plan_execution_reports`
  table keyed on execution_id (UPSERT semantics — a replan run keeps only its
  last report), `report TEXT` JSON, `created_at`, FK cascade to
  `plan_executions`. Index on `created_at DESC`.
- `CURRENT_SCHEMA_VERSION` 18 → 19.
- Additive only; no existing table touched; the events CHECK constraint needs
  no widening because we introduce no new event variants.

## Evaluating Responsibility Constraints

- `Planner`: unchanged planning/replanning logic. The sole addition is handing
  its already-built report to the engine for publication — report serialization
  plumbing, not a planning rule change.
- `ExecutionEngine`: owns scheduling/lifecycle + progress publication. It adds
  storage/publication only, no planner logic.
- `ExecutionContext`/`ToolExecutor`: untouched.

## Security Review

- The new `plan_execution_reports` table persists only planner summary JSON and
  `execution_id`. No secrets/credentials (plans carry application-level tool
  arguments already).
- `execution:progress` payloads are snapshots of the existing persisted
  execution data; no new data surface or privilege boundary.
- Migration is additive with FK cascade; nothing broadened.

## Production Readiness

- Stream publication is best-effort (`app_events::emit` design) and gated by
  an optional emitter: no emission path can fail a run.
- Snapshots reuse the same `get_progress` reader as the IPC so UI and IPC can
  never disagree.
- Planner reports are durable; reload/reconnect shows the same summary the
  live stream showed.
- The engine keeps emitting through `execute_until_complete`'s normal loop
  (paused states still publish), so pause mid-run is visible live.
- Frontend: subscription is `execution_id`-filtered so unrelated runs don't
  clobber the dashboard; controls are disabled by terminal status.

## Remaining Roadmap

- M6: External tool integration as planning sources; telemetry/analytics sweep
  on top of the durable checkpoint backbone; frontend auto-connect to the most
  recent in-flight run on app launch.