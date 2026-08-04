# RC-10 M2 Engineering Report — Production Hardening (Reliability & Recovery)

## Summary

RC-10 M2 completes the second production-hardening milestone: a full
fault-tolerance subsystem spanning backend, database, IPC, and frontend.
The runtime now records its own lifecycle in an append-only reliability
journal (checkpoints with SHA-256 checksums, heartbeats, crashes,
rollbacks, recovery runs, self-healing actions, health snapshots), detects
an unclean shutdown at every launch, and automatically resumes interrupted
jobs from a validated checkpoint or rolls back to the newest valid ancestor
when the checkpoint itself is corrupt. A background watchdog monitors
worker liveness, a health monitor turns it into a single 0–100 score, and a
self-healing service executes the safe remediation (worker monitoring
restarts, checkpoint verification, bounded history pruning). A clean
shutdown is recorded through the `RunEvent::Exit` hook so the next launch
can distinguish a clean stop from a crash.

The backend subsystem (migration `0029`, `models/recovery.rs`,
`RecoveryRepository`, `performance/recovery/` with seven components) was
found in the working tree as uncommitted, unfinished work. This milestone
fixed the two watchdog defects that made the subsystem non-functional,
fixed the clippy violations, added the missing IPC command layer and the
missing frontend surface, wired two repository test suites that were never
registered (one from M1), and shipped the whole thing through every quality
gate.

## Architecture

### What changed (additive only)

| Layer | Addition |
|---|---|
| Migration | `0029_recovery.sql` — `recovery_journal`, `crash_reports`, `worker_health`, `recovery_history` (all new tables, indexed for newest-first reads) |
| Models | `models/recovery.rs` — DTOs for journal entries, checkpoints, crashes, worker health, health snapshots, recovery runs, rollback/self-healing results |
| Repository | `repositories/recovery_repository.rs` — all SQL behind the four tables + `performance_repository.rs` gains its (previously unwired) test module |
| Engine | `performance/recovery/` — `Journal` (checksummed writer), `CheckpointValidator` (pure), `CrashRecoveryService` (startup detection + resume/rollback), `RollbackService`, `WatchdogService`, `HealthMonitor`, `SelfHealingService`, and the `RecoveryManager` facade |
| Commands | `commands/recovery.rs` — `recovery_status`, `recovery_history`, `recovery_crash_reports`, `recovery_latest_checkpoint`, `recovery_self_heal`, `recovery_rollback`, `recovery_tick` (thin wrappers, registered in `lib.rs`) |
| Runtime | `lib.rs` — startup recovery pass before the window shows, background watchdog loop, `RunEvent::Exit` clean-shutdown checkpoint, `RecoveryManager` managed state |
| Frontend | `types/recovery.ts`, `services/recoveryRepository.ts`, `pages/RecoveryPage.tsx`, `components/recovery/{HealthDashboard,CrashPanel,HistoryPanel,JournalPanel}.tsx`, route + sidebar entry |

### How the surfaces compose

```
lib.rs setup()
  └─ RecoveryManager::new(RecoveryRepository::new(pool))
       └─ startup() — register runtime worker → detect_and_recover()
            ├─ detect_crash(latest checkpoint, grace)          [pure rule]
            ├─ CheckpointValidator::validate                   [checksum + ordering]
            ├─ resume jobs from payload          OR  rollback to newest valid ancestor
            └─ crash report + journal entry + recovery_history run   [auditable]
  └─ watchdog_loop() every 30s
       └─ watchdog_tick()
            ├─ heartbeat(runtime) + heartbeat journal every 10th tick
            ├─ WatchdogService::scan — stalled/recovered events, miss counters
            ├─ HealthMonitor::capture — 0..=100 score, persisted health entry
            └─ SelfHealingService::run — restart_worker / verify_checkpoint / prune
  └─ RunEvent::Exit → record_clean_shutdown()   ['clean' checkpoint]
RecoveryManager (managed Tauri state) ← commands/recovery.rs (thin forwards)
```

The dependency order mirrors the rest of the codebase: commands → engine →
repository → models → database. The pure policy pieces
(`CheckpointValidator::validate`, `WatchdogService::evaluate`,
`HealthMonitor::assess`, `SelfHealingService::plan`) are functions of their
inputs and carry no SQL; the stateful components (`Journal`,
`CrashRecoveryService`, `RollbackService`, `HealthMonitor::capture`,
`SelfHealingService::run`, `WatchdogService::scan`) compose the
`RecoveryRepository`. `RecoveryManager` is the facade — no SQL, no policy,
every operation delegates.

## Data flow

**Crash detection and recovery (startup):** the previous session's last
checkpoint (`recovery_journal` where `entry_type = 'checkpoint'`) is read.
A `clean` state means the `Exit` hook ran; anything else means the process
died first. A detected crash classifies (`timeout` when older than the
120 s grace window, otherwise `unknown`), is recorded in `crash_reports`,
and the checkpoint is re-validated by recomputing the SHA-256 over
`entity|state|payload`. A valid checkpoint resumes its `active_jobs`
(recorded as `recovery` journal entry + `recovered` history run); an
invalid one rolls back to the newest valid ancestor (newest-first scan of
`recent_checkpoints`). Every decision is journaled and written to
`recovery_history`, so all automatic intervention is auditable.

**Watchdog loop (30 s interval):** the runtime heartbeat refreshes
`worker_health`; every pass scans every registered worker. A worker whose
heartbeat exceeds the grace window is reported `stalled`, which increments
`consecutive_misses`; a worker that stays stale is re-reported each pass so
the counter climbs until the self-healing restart threshold (3) — this
continuation was a defect fixed in this milestone. A worker whose journal
says it was still `stalled` but whose heartbeat is fresh again is reported
`recovered` and its monitoring state is restarted. Self-healing then
executes the planned actions (restart monitoring for stalled/failed
workers, verify the latest checkpoint when health is degraded, prune the
journal past 10k entries), and the health monitor persists a snapshot.

**Frontend:** `RecoveryPage` loads the status/history/checkpoint/tick on
mount and on every manual action; the self-heal and rollback buttons call
the corresponding commands and refresh both surfaces. No polling — actions
are user-driven (the background watchdog is backend-side).

## Deliverables

- **Backend**: `performance/recovery/` (8 modules + 7 test files), `RecoveryRepository` + tests, `models/recovery.rs`, `commands/recovery.rs` (7 commands), migration `0029`.
- **Frontend**: Recovery page with three tabs (Health / History / Journal), four components, typed service, route (`/recovery`) and sidebar entry.
- **Fixes in this milestone**:
  1. **Watchdog `scan` defect** — an already-stalled worker never re-reported, so `consecutive_misses` capped at 1 and the self-healing restart threshold (3) was unreachable; and a recovered worker could never be observed because `heartbeat_worker` immediately resets status to healthy. Fixed by synthesizing continuation `stalled` events and journal-informed `recovered` detection inside `scan`, keeping `evaluate` pure, the repository API untouched, and all pinned tests passing unmodified.
  2. **Clippy violations** in the uncommitted code (unused import, `too_many_arguments`, byte-slice lints, `len_zero`).
  3. **Dead test suites** — `recovery_repository_tests.rs` (11 tests) and M1's `performance_repository_tests.rs` (8 tests) were never wired via `mod tests;` and never ran; both are now registered per the `context_intel_repository` convention, and a buggy backdating assertion in `crash_reports_prune_before_cutoff` was fixed (the test never actually backdated the row).

## Frontend

The Recovery page mirrors the RC-10 M1 Performance page structure: tab
navigation (Health / History / Journal) with a Session Checkpoint card
above the tabs showing the latest checkpoint state, the watchdog tick
count, and the two manual intervention buttons (Run self-healing, Roll
back) with inline result/error feedback. Health renders the score ring,
status badge, monitored-worker rows, and open issues; History renders
recovery runs (trigger, outcome, actions, resumed jobs, rollback target,
duration) plus crash reports (type, severity, recovered/open); Journal
renders the append-only ledger with per-type badges. Empty, loading, and
error states are handled everywhere. Tests cover the service IPC contract
(8), the page wiring and both action flows (5), and each component's render
and state handling (10).

## Backend

The subsystem is split into pure policy (no SQL) and stateful services
(compose the repository), matching the intelligence `health` precedent:

- `Journal` — the single writer to `recovery_journal`; computes the SHA-256
  checksum over `entity|state|payload` recorded with every entry.
- `CheckpointValidator` — pure rules: correct type, non-empty state,
  checksum match, `active_jobs` payload, monotonic timestamp against the
  previous checkpoint; `newest_valid` finds the rollback target.
- `CrashRecoveryService` — startup pass: `detect_crash` (pure),
  `detect_and_recover` (stateful; resumes or rolls back, persists the audit
  trail, opens the new session's `running` checkpoint).
- `RollbackService` — rolls back to the newest valid ancestor, journals the
  rollback, and reports restored jobs.
- `WatchdogService` — `evaluate` (pure transition events) + `scan`
  (applies events, re-reports persistent stalls, detects journaled
  recovery, journals everything).
- `HealthMonitor` — `assess` (pure 0–100 scoring with stalled/failed
  penalties) + `capture` (persists the health snapshot).
- `SelfHealingService` — `plan` (pure: restart workers past the miss
  threshold, verify the checkpoint when degraded) + `run` (executes,
  journals, and prunes the bounded ledger).
- `RecoveryManager` — facade: startup, clean shutdown, checkpoints,
  heartbeats, watchdog tick/loop, status, history, crash reports, manual
  self-healing and rollback, validator access, tick counter.
- `RecoveryRepository` — owns all SQL: journal append/read/prune,
  checkpoint queries, crash report CRUD, worker health upserts/misses,
  recovery history, health snapshots.

## Tests

### Backend (565 total: 558 lib + 6 integration + 1 doc, 3 ignored)

| Suite | Tests | Covers |
|---|---|---|
| `performance/recovery/checkpoint_validator_tests` | 8 | checksum/type/state/payload/ordering rules, newest-valid selection |
| `performance/recovery/crash_recovery_tests` | 6 | clean start, crash detection rules, resume, corrupt-checkpoint rollback, fail-open without ancestor |
| `performance/recovery/health_monitor_tests` | 5 | scoring, clamping, degraded/critical mapping, persisted snapshot |
| `performance/recovery/journal_tests` | 4 | round-trip, deterministic checksum, checkpoint payload, latest-wins |
| `performance/recovery/recovery_manager_tests` | 7 | startup pass, clean shutdown, crash recovery across fresh managers, history, watchdog tick, rollback, no-op self-healing |
| `performance/recovery/rollback_tests` | 3 | newest-valid-ancestor selection, journaling, fail-open |
| `performance/recovery/self_healing_tests` | 7 | plan rules, miss-threshold gating, restart execution, checkpoint verification + rollback, pruning |
| `performance/recovery/watchdog_tests` | 6 | transition events, continuing miss counting, journaled recovery detection |
| `repositories/recovery_repository_tests` | 11 | journal round-trip/limits/pruning, checkpoints, crash logging + pruning, worker health lifecycle, recovery runs, health snapshots |
| `repositories/performance_repository_tests` | 8 | M1 suite now wired and running (was silently dead) |

### Frontend (91 total across 18 files)

| Suite | Tests | Covers |
|---|---|---|
| `services/recoveryRepository.test.ts` | 8 | every `recovery_*` IPC command + argument shape + singleton |
| `pages/RecoveryPage.test.tsx` | 5 | health/checkpoint rendering, history tab, journal tab, self-heal action, rollback action |
| `components/recovery/HealthDashboard.test.tsx` | 3 | status badge, score, worker rows, issues, empty/loading/error |
| `components/recovery/CrashPanel.test.tsx` | 2 | crash rows with recovery state, empty/loading/error |
| `components/recovery/HistoryPanel.test.tsx` | 2 | runs with actions/jobs/rollback target, empty/loading/error |
| `components/recovery/JournalPanel.test.tsx` | 2 | entry badges/entities/states, empty/loading/error |

## Design decisions

- **The journal is the single source of truth for transitions.** Because
  `heartbeat_worker` immediately returns a row to `healthy`, the watchdog
  cannot observe a recovery from the row alone; it derives the `recovered`
  transition from the newest journal entry about the worker. This keeps the
  repository API untouched and makes the watchdog's transition accounting
  consistent with the audit trail.
- **Pure evaluation, stateful application.** `evaluate`, `assess`, `plan`
  and `validate` are pure functions of their inputs (trivially testable,
  no database); only the application passes (`scan`, `capture`, `run`,
  `detect_and_recover`) touch the repository. This mirrors the RC-10 M1
  optimizer split.
- **Miss counting is per-pass, not per-transition.** A persistently stale
  worker re-reports `stalled` every pass so `consecutive_misses` climbs to
  the self-healing restart threshold. Without this the threshold was dead
  code — the defect fixed in this milestone.
- **Checksummed checkpoints.** SHA-256 over `entity|state|payload` lets the
  validator detect a half-written checkpoint row after a crash and roll
  back instead of blindly resuming.
- **Best-effort audit writes.** A failing `recovery_history` write logs a
  warning but never fails startup recovery itself.
- **Bounded ledger.** Self-healing prunes the journal past 10k entries and
  crash reports past 30 days, matching the M1 profiler-ledger precedent.

## Trade-offs

- **Every launch writes a history row.** The startup pass records a
  `no_action` run each clean launch (same cadence as `startup_profiles`) so
  the audit trail proves the check happened — at the cost of one row per
  launch, bounded by pruning.
- **Worker monitoring is opt-in.** Only registered workers are watchdogged;
  integrations must call `register_worker`/`heartbeat` to opt in. Keeps the
  diff additive and avoids asserting liveness on subsystems that never
  agreed to it.
- **Watchdog loop is isolated but uncoordinated.** Each pass failure is
  logged and the loop continues, so a database hiccup cannot stop
  monitoring permanently; conversely the loop is not wired into the
  frontend (no live events) — the page reflects state on load/action, the
  watchdog runs in the backend.
- **Journal reads per worker per pass.** The `recovered` detection reads
  one row per worker per watchdog tick (30 s); negligible at current
  worker counts, and simpler than a batched read.
- **Crash classification is deliberately simple.** A non-`clean`
  checkpoint is a crash; `timeout` vs `unknown` depends only on the grace
  window — no heuristics beyond that.

## Compatibility

- **No architecture rewrites.** `RecoveryManager` is a facade; commands are
  thin wrappers; the repository owns all SQL; dependency direction is
  unchanged (commands → engine → repository → models → database).
- **No breaking APIs, IPC, or schema.** One new migration (`0029`) creates
  four new tables; no existing table was altered. Seven new `recovery_*`
  commands; nothing existing changed signature. The one behavioral change
  is internal to `WatchdogService::scan` (continuation events), which
  cannot break external callers because none could observe the previous
  (non-functional) miss accounting.
- **Frontend is purely additive**: one route, one sidebar entry, new
  types/service/components. No existing page was modified.

## Quality gates

All gates ran clean on the final tree:

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 0 warnings |
| `cargo build` | pass |
| `cargo test` | 565 passed (558 lib + 6 integration + 1 doc), 3 ignored, 0 failed |
| `npm run build` | pass |
| `npx tsc -b` | pass |
| `npm test` | 91 passed, 0 failed |
| `npm run lint` | no new problems (13 pre-existing errors, 5 pre-existing warnings, same as M1) |

## Remaining TODOs

- RC-10 M3 (production hardening): data integrity & backup — backup/restore
  of the SQLite database, `PRAGMA integrity_check`, VACUUM/`optimize`, and
  export surfaces.
- RC-10 M4 (production hardening): security hardening — secure-storage
  review, command/audit logging, secret handling validation.
- Optional: frontend live events for watchdog transitions (currently the
  page reflects state on load/manual action only).
