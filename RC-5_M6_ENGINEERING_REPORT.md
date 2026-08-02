# RC-5 M6 Engineering Report — Autonomous Agent Runtime

**Date:** 2026-08-02  
**Commit:** (pending)  
**Branch:** `main`

---

## Summary

RC-5 M6 delivers the **Autonomous Agent Runtime** — a reason–act–observe loop that drives the existing `Planner` and `ExecutionEngine` through an autonomous *session*, enforcing budgets, retries, timeouts, approval checkpoints, and cancellation, while streaming reasoning events to the frontend.

The implementation strictly follows the RC-5 charter: **no duplicate execution pipelines**, **no callback framework**, **no event bus redesign**, and **no architecture rewrite**. The runtime is a thin orchestration layer that:
- **Owns the session**: budgets, retry/timeout policies, approval checkpoints (human-in-the-loop), reasoning event streaming, autonomous cancellation
- **Reuses unchanged**: `Planner` (DAG planning/replanning), `ExecutionEngine` (scheduling/lifecycles/checkpoints/streaming), `ToolExecutor` (only execution path)

---

## Architecture

### Responsibility Split (frozen from RC-5 charter)

| Component | Responsibility |
|-----------|----------------|
| `Planner` | planning, replanning, dependency graph generation |
| `ExecutionEngine` | execution, scheduling, checkpoints, lifecycle, `execution:progress` streaming |
| `ToolExecutor` | only execution path for a single tool |
| `AutonomousRuntime` | session budgets, retry/timeout policies, approval checkpoints, reasoning loop, `autonomous:session`/`autonomous:reasoning` streaming |

### Session Lifecycle

```
start_session()
    │
    ├─► Planner.plan()  ──► approval_gate() ──► ExecutionEngine.run()
    │                           │
    │                           ├─► approve_session() ──► continue
    │                           └─► reject_session() ──► cancel
    │
    ├─► budget_breach() checks (steps, duration, plans)
    ├─► recover_failure() ──► Planner.replan_with_feedback() ──► retry/replan
    │
    └─► Terminal: Completed | Failed | Cancelled
```

---

## Deliverables

### Backend (src-tauri/src/copilot/autonomous/)

| File | Lines | Description |
|------|-------|-------------|
| `mod.rs` | 26 | Public re-exports |
| `models.rs` | 390 | Pure data types + deterministic policy decisions |
| `runtime.rs` | 1,360 | `AutonomousRuntime` implementation + tests |
| `commands/autonomous.rs` | 111 | Tauri IPC command handlers |

### Frontend (frontend/src/)

| File | Description |
|------|-------------|
| `types/autonomous.ts` | TypeScript mirrors of backend models |
| `services/autonomousRepository.ts` | IPC bindings (start, getProgress, listRecent, pause, resume, cancel, approve, reject) |
| `hooks/useAutonomousStream.ts` | Live stream hook (`autonomous:session` + `autonomous:reasoning`) |
| `features/autonomous/AutonomousDashboard.tsx` | Live dashboard (status, budgets, reasoning log, approval gate, controls) |
| `features/autonomous/AutonomousControls.tsx` | Pause/Resume/Cancel buttons |
| `features/autonomous/AutonomousReasonLog.tsx` | Reasoning event timeline |
| `features/autonomous/AutonomousApprovalGate.tsx` | Approve/Reject checkpoint UI |
| `features/autonomous/ExecutionDigest.tsx` | Budget counters |

### Tests

| File | Tests | Status |
|------|-------|--------|
| `models.rs` (mod tests) | 4 | ✅ |
| `runtime.rs` (mod tests) | 4 | ✅ |
| `useAutonomousStream.test.ts` | 3 | ✅ |
| `AutonomousDashboard.test.tsx` | 7 | ✅ |

**Total: 18 new tests, all passing**

---

## Key Features Implemented

### 1. Execution Budget (`ExecutionBudget`)
- `max_steps` — step count ceiling across all plan runs
- `max_plans` — distinct engine executions
- `max_replans` — feedback-driven replans
- `max_duration_seconds` — wall-clock session timeout
- Enforced at start of each loop iteration (`budget_breach()`)

### 2. Retry Policy (`RetryPolicy`)
- `max_attempts` — extra re-attempts of a failing plan before replanning
- `backoff_ms` — delay before retry
- `retry_on_timeout` — whether timeouts count toward retry budget
- Applied in `recover_failure()` with feedback-aware replanning

### 3. Timeout Policy (`TimeoutPolicy`)
- `step_timeout_ms` — per-tool timeout (passed to `ToolExecutor`)
- `plan_timeout_seconds` — whole-plan deadline (cancels engine run)
- `approval_timeout_seconds` — auto-reject if no human decision
- Enforced in `run_plan()` and `approval_gate()`

### 4. Approval Workflow (`ApprovalPolicy` + `ApprovalMode`)
- `Automatic` — never pauses for confirmation
- `OnRisk` — pauses for tools marked `requires_confirmation` or `High` risk
- `Manual` — pauses before every plan run
- `gate_replans` — also gate replans introducing new tools
- Pure decision function `approval_required()` for unit testing

### 5. Reasoning Event Streaming
- Events: `Planning`, `Executing`, `Observed`, `Replanning`, `AwaitingApproval`, `ApprovalResolved`, `BudgetUpdate`, `Pause`, `Terminal`
- Bounded history (200 events) in session state
- Streamed via `autonomous:reasoning` (live) + `autonomous:session` (snapshot)
- Reconnect/restore via `autonomous_get_progress` (replays `reasoning` array)

### 6. Autonomous Controls
- `pause_session()` — pauses engine + suspends loop
- `resume_session()` — resumes engine + loop
- `cancel_session()` — cancels engine + token, marks terminal
- `approve_session()` / `reject_session()` — resolve approval checkpoint

### 7. IPC Commands (all registered in `lib.rs`)
| Command | Description |
|---------|-------------|
| `autonomous_start` | Start session, returns initial snapshot |
| `autonomous_get_progress` | Reconnect snapshot |
| `autonomous_list_recent` | Recent sessions for re-attach |
| `autonomous_pause` | Pause running session |
| `autonomous_resume` | Resume paused session |
| `autonomous_cancel` | Cancel session |
| `autonomous_approve` | Approve checkpoint |
| `autonomous_reject` | Reject checkpoint (terminates) |

---

## Frontend Integration

The `AutonomousDashboard` consumes `useAutonomousStream(sessionId)` and renders:
- Header: goal, session ID, status badge (running/paused/waiting_approval/completed/failed/cancelled)
- `ExecutionDigest`: plans attempted/completed, steps completed/left, retries, replans
- `AutonomousApprovalGate`: shown when `pending_approval` present
- Error banner: when `progress.error` set
- `AutonomousReasonLog`: newest-first reasoning timeline

All controls call the corresponding IPC commands and `refresh()` after action.

---

## Gate Results

| Gate | Result |
|------|--------|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets -- -D warnings` | ✅ |
| `cargo build` | ✅ |
| `cargo test` | ✅ (271 tests: 266 unit + 5 integration + 1 doc) |
| `npm run build` | ✅ |
| `npx tsc -b --noEmit` | ✅ |
| `npm test` | ✅ (19 tests) |

---

## Files Modified (beyond new files)

- `src-tauri/src/lib.rs` — `AutonomousRuntime` construction + event emitter wiring + `app.manage()` + invoke handler registration
- `frontend/src/hooks/useAutonomousStream.test.ts` — Fixed test mocks
- `frontend/src/features/autonomous/AutonomousDashboard.test.tsx` — Fixed test mocks and assertions

---

## Compatibility

- No breaking changes to existing `Planner`, `ExecutionEngine`, `ToolExecutor`, or `execution:progress` streaming
- Frontend `execution:progress` dashboard (M5) unchanged
- New `autonomous:*` events are additive

---

## Follow-up (out of scope for M6)

- Persistent session store (survive app restart)
- Multi-workspace session coordination
- Advanced approval UX (diff view, risk badges)
- Metrics/observability hooks for session outcomes