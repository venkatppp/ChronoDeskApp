# RC-10 M4 Engineering Report — Production Hardening (Security Hardening)

## Summary

RC-10 M4 completes the production-hardening milestone sequence with a
security hardening subsystem spanning backend, database, and IPC. Starting
from the RC-10 M3 data-integrity base, ChronoDesk now runs a non-fatal
startup security validation and a background security monitor that score
the environment 0–100 across six categories (database, files, secrets,
backup, input, config), persist every finding and recommendation into two
durable ledgers, audit every security-relevant action append-only, and
surface the whole picture to the user through eleven thin IPC commands.

Three established codebases are reused rather than duplicated: the M3
`backup_runs` ledger is shared (via `Arc`) so backup presence/checksum
checks verify against the same single-owned source of truth the backup
engine writes; the keyring-backed `SecretStore` the LLM settings use is
shared too; and the `AuditService`/ledger pattern mirrors M2/M3. The
security layer stores API keys exactly as the LLM layer already does and
adds a read-only `api_key_storage_state` inspection that classifies key
storage (`None` / `Secure` / `Plaintext` / `SecretStoreUnavailable`)
without ever migrating a plaintext key or reading the key value.

The milestone is purely additive: one migration (`0031`) adds four new
ledger tables, no existing table is altered, no existing IPC signature
changes, and the only edits to existing files are mechanical
(`Arc`-sharing two existing repositories with the security engine, module
registration, schema-version bump 30 → 31, and one event constant).
`CURRENT_SCHEMA_VERSION` is updated to 31.

## Architecture

### What changed (additive only)

| Layer | Addition |
|---|---|
| Migration | `0031_security_hardening.sql` — four ledgers: `security_audit_log` (append-only audit trail), `security_config` (persisted policy key/values), `security_findings` (per-run check results grouped by `run_id`), `security_recommendations` (rule-owned, statused so ack/dismiss survives reruns), with newest-first and unique-per-rule indexes |
| Models | `models/security.rs` — `SecuritySeverity`, check DTOs, `SecurityFinding`, `SecurityAuditEntry`, `SecurityConfigEntry`, `SecurityScoreReport` (0–100 `health_score` + status), `StartupValidationReport`, `SecurityDiagnosticsReport`, `SecretValidationReport`, `PermissionsReport`, `SecurityRecommendation` (+ status), `SecurityAction` |
| Repository | `repositories/security_repository.rs` — all SQL for the four ledgers, including `upsert_recommendation` (keyed by rule, preserving applied/dismissed status) and the retention prunes |
| Engine | `security/` — pure check functions (`checks.rs`), stateful battery (`validator.rs`), `0..100` scorer (`scoring.rs`), pure recommendation rules (`recommendations.rs`), the policy table (`policy.rs`), the audit lifecycle (`audit.rs`), and the `SecurityEngine` facade (`mod.rs`) with startup validation, monitor loop, diagnostics, secrets/permissions sub-batteries, and config/status/history surfaces |
| LLM integration | `llm/models.rs` — `ApiKeyStorageState`; `repositories/llm.rs` — side-effect-free `api_key_storage_state()`; keyring store shared via `Arc` so security and LLM settings use one store |
| Commands | `commands/security.rs` — `security_status`, `security_diagnostics`, `security_secrets`, `security_permissions`, `security_history`, `security_audit_log`, `security_config`, `security_set_config`, `security_recommendations`, `security_apply_recommendation`, `security_dismiss_recommendation` (thin forwards, registered in `lib.rs`) |
| Runtime | `lib.rs` — `SecurityEngine` managed state; `Arc`-shared `MaintenanceRepository` and keyring `SecretStore`; non-fatal startup validation; `run_monitor_loop` spawned; `EVENT_SECURITY_STATUS` (`security:status`) in `app_events.rs` |
| Schema | `database/schema.rs` — `CURRENT_SCHEMA_VERSION` bumped to 31 |
| Frontend | none this milestone — security hardening is backend/IPC-only (the M5 surface work is noted under TODOs) |

### How the surfaces compose

```
lib.rs setup()
  └─ SecurityEngine::new(SecurityRepository, Arc<MaintenanceRepository>,
                         Arc<LLMRepository>, Arc<SecretStore>, db_path, backup_dir)
       ├─ SecurityValidator  — run_full() battery: DB (WAL, FK, trusted_schema,
       │  │                   secure_delete), file perms, secret storage state,
       │  │                   backup presence/checksum (shared M3 ledger),
       │  │                   input/path, and policy config
       ├─ scoring            — health_score(checks) + score_findings
       ├─ Recommendations    — pure rule engine → statused recommendations
       ├─ AuditService       — append + retention prune of the audit ledger
       └─ policies           — persisted key/value thresholds and their validators
Engine surfaces: startup_validation (non-fatal, audited), diagnostics, status,
  secrets, permissions, history, audit_log, config/set_config,
  recommendations/{apply,dismiss}, monitor_tick, run_monitor_loop, prune_ledgers
commands/security.rs  — thin forwards, exactly one engine call each
```

Dependency order matches the rest of the codebase: commands → engine →
repository → models → database. SQL lives entirely in
`SecurityRepository` (plus the two reused repositories); policy (what is
an acceptable threshold) lives in `policy.rs`; scoring is pure; the
validator is the only stateful gatherer. The engine's monitor loop and
startup pass are never fatal — a failing check must never block the app.

## Data flow

**Startup validation:** on boot, `startup_validation` runs the full
battery once, persists every check into `security_findings` under a fresh
`run_id`, refreshes the recommendation table from the pure rules, audits
the run (`startup_validation`, severity by verdict), and logs the score.
Any error is logged and startup continues.

**Monitor loop:** a spawned async task reads the policy interval
(`security.monitor_interval_seconds`, default 300s), and every tick runs
`monitor_tick` — the same full battery — persists findings, refreshes
recommendations, applies no recommendations automatically, audits the
tick, emits `security:status` with the scored report, and prunes the audit
and findings ledgers to their retention windows. Interval and retentions
are configurable per-user via `security_set_config` and validated by
`policy.rs` before persisting.

**Manual surfaces:** `security_diagnostics` re-runs the battery on demand
(audited, never fatal); `security_secrets` and `security_permissions` run
the two targeted sub-batteries (LLM key storage state; Diachronous file
permissions) coherently with the full report; `security_history` /
`security_audit_log` page the two ledgers newest-first; `security_status`
replays the latest run into a score report.

**Recommendations:** `apply_recommendation` executes the action the pure
rule offers via the engine (prune audit log, prune findings history) and
flips the row to `applied`; `dismiss_recommendation` flips it to
`dismissed` so a known non-issue never reappears. Because recommendation
rows are unique per rule, status survives every future battery run.

**Audit:** every intervention — startup validation, diagnostics, monitor
ticks, config changes, recommendation applies/dismissals, and ledger
prunes — appends to `security_audit_log` with action, severity, actor
(system/monitor/user), target and detail. Retention is bounded by
`security.audit_retention_days` (default 90).

## Deliverables

- **Backend**: `security/` (8 modules + 44 tests total), `SecurityEngine`
  engine tests, `SecurityRepository` + tests, `models/security.rs`,
  `commands/security.rs` (11 commands), migration `0031`, schema-version
  bump, `api_key_storage_state` + `ApiKeyStorageState`, `Arc`-shared
  maintenance/secret-store wiring, `security:status` event.
- **Correctness fixes in this milestone** (see "Flaky-test root cause"
  below): three same-root-cause flaky RC-10 M1 prune tests made
  deterministic by backdating their rows, plus seven clippy lint
  cleanups in the new security code.
- **Frontend**: none this milestone.

## Frontend

No frontend changes. All 116 existing frontend tests continue to pass
(regression surface only).

## Backend

- `checks.rs` — the pure check functions: database facts (foreign keys,
  trusted schema, secure delete, journal mode), file-permission checks,
  secret-storage checks against `ApiKeyStorageState`, backup
  presence/checksum checks against the shared M3 ledger, path/input
  checks justified at the boundary, and policy-config checks. Each
  returns the shared `SecurityCheck` DTO (category, severity, passed,
  detail) so the scorer and the ledgers decode one shape.
- `validator.rs` — the stateful battery: gathers environment facts (DB
  path, journal mode, file perms, key storage, backup row + on-disk
  file/checksum, policy values) and composes the checks. `run_full`,
  `run_secrets`, `run_permissions` share one check registry; the
  non-fatal design lives here (a failure in one check never aborts the
  run).
- `scoring.rs` — a pure `0..100 >health_score` over check counts and a
  severity-weighted `score_findings`, with a status mapping. Unit-tested
  without a database.
- `recommendations.rs` — pure rules over the score + ledger sizes +
  secrets verdict offering either auto-applyable actions
  (`prune_audit_log`, `prune_findings_history`) or guidance.
- `policy.rs` — the `security_config` key registry and validators
  (interval range, retention range), each with unit tests.
- `audit.rs` — `AuditService`: append + retention prune in one owner.
- `security_repository.rs` — all four ledgers' SQL (`audit`,
  `config`, `findings`, `recommendations`), `upsert_recommendation` keyed
  on the unique `rule` column, status transitions, and the two retention
  prunes.
- `llm.rs` — `api_key_storage_state`: reads the stored marker, never the
  key; reports `None`/`Secure`/`Plaintext`/`SecretStoreUnavailable`.

## Flaky-test root cause and fix

The RC-10 M4 code itself passed 44/44, but two pre-existing RC-10 M1
tests were intermittently failing, blocking the "0 failing tests" gate.

**Root cause — a millisecond-precision boundary race in the
`performance_profiles` prune tests.** The ledger's `occurred_at` column
defaults to `strftime('%Y-%m-%dT%H:%M:%fZ','now')` (RFC3339 with
*millisecond* precision, migration `0028`), and
`prune_profiles_older_than` computes its cutoff in SQL with the *same*
format at the DELETE's own start. The tests inserted a row and pruned
immediately with `days = 0`; when both statements evaluated `'now'`
within the same millisecond the two strings are byte-identical, so the
strict `occurred_at < cutoff` is false and the just-inserted row
survived — `removed == 0` instead of `1`. Verified empirically with the
SQLite CLI (two same-ms statements produce identical values) and by
reproduction (one test failed 3/3 runs, the other 1/6).

The production SQL semantics are correct: "delete samples older than N
days" with strict `<` at the boundary, and the optimizer only ever prunes
with days ≥ 30. The defect was the tests' assumption that a row inserted
"now" is strictly older than a days-0 cutoff.

**Fix (test-only, smallest possible, assertions unchanged):** each test
backdates its inserted row to `now - 10 days` before pruning — the exact
pattern already used by `crash_reports_prune_before_cutoff`
(`recovery_repository_tests.rs`) and `audit_prune_removes_only_old_rows`
(`security_repository.rs`). Applied to all three instances of the
one-root-cause defect:

1. `repositories/performance_repository_tests.rs::prune_removes_only_old_rows_and_reports_count`
2. `performance/profiler_tests.rs::prune_older_than_removes_old_ledger_rows`
3. `performance/engine_tests.rs::prune_action_applies_when_history_exists`

No production code changed. The three fixed tests pass 20/20 consecutive
runs (previously failing up to `removed == 0` intermittently). Seven
additional clippy warnings in the new security code (redundant closure,
manual `.ok()`, `== false`, `assert_eq!(.., true)`, useless `vec!`) were
cleaned up so `clippy --all-targets -- -D warnings` is green.

## Design decisions

- **Shared, single-owned ledgers.** The M3 `backup_runs` repository and
  the LLM keyring store are passed in as `Arc`s — the security engine
  reads them, nothing duplicates their SQL or their storage logic. One
  owner, two readers.
- **Read-only secret-state inspection.** `api_key_storage_state` reads
  the marker and pings the keyring; it never migrates a plaintext key
  (unlike `get_settings`, which does), so the validator an observe
  storage without any side effect, and the raw key value never leaves the
  secret store.
- **Non-fatal everywhere.** Startup validation and the monitor loop log
  and continue on any check failure or DB error; a hardening subsystem
  must not be able to take the app down.
- **Rules persist status per rule.** The unique index on `rule` lets an
  applied/dismissed recommendation survive every subsequent battery, so
  the user's acknowledgments are durable and a suggestion never silently
  reappears.
- **Checks are pure; the validator is stateful; score is pure.** One
  shared `SecurityCheck` shape flows through composition, persistence,
  and scoring — no drifted duplicate DTOs.

## Trade-offs

- **Monitor loop runs in-process.** A spawned async task polls on the
  policy interval rather than a separate daemon; it shares the app pool
  and stops when the app exits, which keeps the hardening subsystem
  dependency-free at the cost of not hardening an offline database.
- **Recommendations do not auto-apply except via user action.**
  `monitor_tick` persists recommendations but applies none
  automatically; the only auto-action path is the user clicking Apply.
  A fully automatic remediation path was deliberately not defaulted on.
- **Backend-only milestone.** The security surface is exposed over IPC
  (11 commands) and `security:status`, but there is no dedicated
  frontend page yet — the Milestone Keep allows the surface work to land
  separately (TODO below).

## Compatibility

- **No architecture rewrites.** `SecurityEngine` is a facade; commands
  are thin wrappers; dependency direction is unchanged.
- **No breaking APIs, IPC, or schema.** One new migration creates four
  new tables; no existing table was altered. Eleven new `security_*`
  commands; nothing existing changed signature. Existing
  `MaintenanceRepository`/`LLMRepository`/keyring behavior is unchanged —
  they are merely wrapped in `Arc` and shared.
- **`Arc`-shared repositories are transparent.** Callers of
  `MaintenanceRepository`/`LLMRepository` construct and use them exactly
  as before; only `lib.rs` wiring changed.
- **Frontend untouched** — no existing page, service, or type changed.

## Quality gates

All gates ran clean on the final tree:

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 0 warnings |
| `cargo build` | pass |
| `cargo test` | 638 passed (631 lib + 6 integration + 1 doc), 3 ignored, 0 failed |
| `npm run build` | pass |
| `npx tsc -b` | pass |
| `npm test` | 116 passed, 0 failed |

Backend security suite: 44/44 (engine 11, checks 8, scoring 6,
recommendations 5, validator 4, policy 4, repository 6).

## Remaining TODOs

- RC-10 M5: security frontend surface (Security page: score, checks,
  secrets/permissions, recommendations with apply/dismiss, audit log,
  policy config) on top of the 11 IPC commands and `security:status`.
- Optional: auto-apply of non-destructive recommendations behind an
  explicit opt-in policy key.
- Optional: per-run security history rendering (already grouped by
  `run_id`; only the UI is missing).