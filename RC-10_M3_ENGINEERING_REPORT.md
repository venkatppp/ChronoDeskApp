# RC-10 M3 Engineering Report — Production Hardening (Data Integrity & Backup)

## Summary

RC-10 M3 completes the third production-hardening milestone: a data
integrity & backup subsystem spanning backend, database, IPC, and
frontend. ChronoDesk can now create consistent, checksummed snapshots of
its SQLite database (SQLite's online `VACUUM INTO`), restore from any
snapshot safely and reversibly, run the full `PRAGMA` integrity battery
(`integrity_check`, `quick_check`, `foreign_key_check`, page statistics),
and perform a measured maintenance pass (WAL checkpoint → gated `VACUUM` →
`PRAGMA optimize`). Every backup, staged restore, integrity check and
maintenance run is recorded in a new `backup_runs` audit ledger, so every
database-level intervention is auditable after the fact — the same
philosophy as M2's `recovery_history`.

The critical design property is that **restores are staged, never applied
live**: a validated snapshot is copied to a `restore-pending.db` marker
next to the database, and `Database::initialize_at` swaps it in *before*
the pool opens on the next launch, preserving the previous database as a
`chronodesk-pre-restore-*.db` safety copy. Swapping a WAL database under
an open pool — the alternative — is exactly the hazard this avoids.

The milestone continues the established additive pattern (engine facade →
repository → thin IPC commands → frontend types/service/page/components),
bumps the previously-stale `CURRENT_SCHEMA_VERSION` (22 → 30, with all
intervening milestones documented), and passes every quality gate.

## Architecture

### What changed (additive only)

| Layer | Addition |
|---|---|
| Migration | `0030_data_integrity.sql` — `backup_runs` audit ledger (kind, status, path, size, SHA-256 checksum, detail, durations, timestamps) + newest-first indexes |
| Models | `models/backup.rs` — `BackupRun` (ledger row), `IntegrityLines`/`IntegrityChecks`/`IntegrityReport` (the `PRAGMA` battery), `RestoreResult` (staged restore), `MaintenanceReport` (before/after measurements) |
| Repository | `repositories/maintenance_repository.rs` — the `backup_runs` ledger SQL + the maintenance statements (`VACUUM`, `VACUUM INTO`, `PRAGMA optimize`, `wal_checkpoint(TRUNCATE)`) |
| Engine | `maintenance/` — `IntegrityChecker` (sole owner of the diagnostic `PRAGMA` battery, runnable over the live pool *and* read-only backup files), `BackupService` (snapshot + SHA-256 via the shared `HashingService`), `RestoreService` (validate → stage → report/cancel), `MaintenanceRunner` (checkpoint → gated VACUUM → optimize), and the `MaintenanceEngine` facade |
| Database | `database/mod.rs` — `apply_pending_restore` (marker swap + safety copy + stale sidecar cleanup, before the pool opens), `RESTORE_PENDING_FILE` / `PRE_RESTORE_BACKUP_PREFIX` constants |
| Commands | `commands/maintenance.rs` — `maintenance_integrity`, `maintenance_backup`, `maintenance_backups`, `maintenance_restore`, `maintenance_pending_restore`, `maintenance_cancel_restore`, `maintenance_optimize` (thin wrappers, registered in `lib.rs`) |
| Runtime | `lib.rs` — `MaintenanceEngine` managed state built from `app_data_dir` (db path + `backups/` dir); app-data-dir resolution refactored to be resolved once |
| Schema | `database/schema.rs` — `CURRENT_SCHEMA_VERSION` bumped to 30 with doc comments for 28/29/30 (was stale at 22) |
| Frontend | `types/backup.ts`, `services/maintenanceRepository.ts`, `pages/MaintenancePage.tsx`, `components/maintenance/{BackupPanel,IntegrityPanel,MaintenancePanel}.tsx`, route (`/maintenance`) + sidebar entry |

### How the surfaces compose

```
lib.rs setup()
  └─ MaintenanceEngine::new(MaintenanceRepository::new(pool), db_path, backups_dir)
       ├─ IntegrityChecker  — PRAGMA battery: page stats + journal mode
       │                      + integrity_check + quick_check + fk check
       │                      (live pool, or read-only immutable pool over a backup file)
       ├─ BackupService     — VACUUM INTO → SHA-256 → size → ledger row
       ├─ RestoreService    — header check → file battery → copy to
       │                      restore-pending.db → ledger row (staged)
       │                      (pending()/cancel() for the UI)
       └─ MaintenanceRunner — checkpoint → should_vacuum()? VACUUM → optimize
                              → before/after stats → ledger row
Database::initialize_at(next launch)
  └─ apply_pending_restore(db_path)   [before the pool opens]
       ├─ marker present? → copy live db to chronodesk-pre-restore-<ts>.db
       ├─ delete stale -wal/-shm of both files
       └─ rename(restore-pending.db → chronodesk.db)
MaintenanceEngine (managed Tauri state) ← commands/maintenance.rs (thin forwards)
```

Dependency order mirrors the rest of the codebase: commands → engine →
repository → models → database. SQL is deliberately split: the audit
ledger and the maintenance *statements* live in
`MaintenanceRepository`; the diagnostic *battery* lives in
`IntegrityChecker` because it must run against arbitrary read-only backup
files, which a single-pool repository cannot express. Policy
(`should_vacuum`, staging rules) lives in the engine modules.

## Data flow

**Backup:** `BackupService::create` ensures the backups directory exists,
then runs `VACUUM INTO '<dir>/chronodesk-<timestamp>.db'` through the
pool. `VACUUM INTO` is SQLite's online backup: the snapshot reflects one
consistent database state even while writers are active, the output is a
standalone file (no WAL sidecars), and the live database is untouched.
The snapshot is SHA-256-hashed with the shared `HashingService` and a
`success` (or `failed`) `backup_runs` row records filename, size,
checksum, and duration.

**Restore (staged):** `RestoreService::stage` first rejects anything that
isn't a SQLite file (magic-header check), then opens the backup
read-only (`read_only` + `immutable`) and runs the same `quick_check` and
`foreign_key_check` queries that certify the live database. A validated
backup is copied to `restore-pending.db` next to the live database and
recorded as a `staged` ledger row. On the next launch,
`apply_pending_restore` swaps it in before any connection exists,
preserving the outgoing database as a `chronodesk-pre-restore-*.db`
safety copy and deleting stale `-wal`/`-shm` sidecars (an old WAL must
never survive its database file). Staging can be cancelled at any time;
`pending()` reports the staged state (re-validated) so the UI can show a
"restart to apply" banner.

**Integrity:** `maintenance_integrity` runs the full battery against the
live pool: page count/size/freelist (one table-valued-pragma read),
journal mode, full `integrity_check`, `quick_check`, and
`foreign_key_check` (formatted as human-readable violation lines). The
report's `ok` is the conjunction of all three verdicts; every run is
audited.

**Maintenance:** `maintenance_optimize` checkpoints the WAL into the main
file (`wal_checkpoint(TRUNCATE)`), then runs `VACUUM` only when
`should_vacuum` passes (free pages ≥ 64 **and** ≥ 10% of the file — a full
rewrite is not something to do on every click), then `PRAGMA optimize`.
Before/after free pages and file size produce the recovered-bytes
measurement; every run is audited.

**Frontend:** `MaintenancePage` has three tabs — Backups (back up now,
the audit ledger, per-backup Restore, staged-restore banner with Cancel),
Integrity (battery results with violation lines), Maintenance (before/after
measurements and the vacuum decision). All actions refresh the ledger.

## Deliverables

- **Backend**: `maintenance/` (4 modules + in-module tests + engine tests),
  `MaintenanceRepository` + tests, `models/backup.rs`, `commands/maintenance.rs`
  (7 commands), migration `0030`, `apply_pending_restore` in `database/mod.rs`
  + tests, schema-version correction.
- **Frontend**: Maintenance page with three tabs, three components, typed
  service, route (`/maintenance`) and sidebar entry.
- **Correctness fixes in this milestone**:
  1. **`CURRENT_SCHEMA_VERSION` was stale (22)** while the schema had
     advanced through 28/29 — corrected to 30 with the intervening
     milestones documented (the constant is informational; migrations
     remain authoritative).
  2. Test-side: WAL-mode nuance — a live database's rows live in its
     `-wal` file, so the pending-restore integration test checkpoints the
     source before staging, exactly matching what `VACUUM INTO` produces.

## Frontend

The Maintenance page mirrors the RC-10 M1/M2 page structure: tab
navigation (Backups / Integrity / Maintenance) with inline action
feedback. Backups renders the "Back up now" action, the audit ledger
(status badges, filename, SHA-256, size, duration), a Restore action on
every successful backup, and a banner for a staged restore with a Cancel
action. Integrity renders the verdict badge, file statistics (size,
pages, page size, free pages, journal mode) and any violation lines.
Maintenance renders the before/after file size, free pages, recovered
bytes, checkpointed frames and the vacuum decision. Empty, loading, and
error states are handled everywhere. Tests cover the service IPC contract
(8), the page wiring and all five action flows (6), and each component's
render and state handling (8).

## Backend

- `IntegrityChecker` — sole owner of the `PRAGMA` battery; `check_live`
  over the pool, `check_file` over a read-only immutable connection
  (backup validation); `page_stats` via table-valued pragma functions
  (`pragma_page_count()/pragma_page_size()/pragma_freelist_count()`) in a
  single read; `PageStats::freelist_ratio`/`size_bytes` for the runner.
- `BackupService` — snapshot via `VACUUM INTO`, SHA-256 via the shared
  `HashingService`, audited `success`/`failed` ledger rows.
- `RestoreService` — SQLite magic-header rejection, file battery
  validation, marker copy, `staged` ledger row; `pending()` re-validates
  the staged copy; `cancel()` removes the marker and audits the cancel.
- `MaintenanceRunner` — checkpoint → gated `VACUUM` (`should_vacuum` is a
  pure, unit-tested policy function) → optimize, measured and audited.
- `MaintenanceRepository` — the `backup_runs` ledger (record, recent,
  by-id, latest-of-kind) and the maintenance statements, including
  `vacuum_into` with path validation (absolute only, NUL rejected, single
  quotes escaped — no stringly-typed SQL injection surface).
- `apply_pending_restore` — marker swap, pre-restore safety copy, stale
  sidecar cleanup; runs before the pool opens in `initialize_at`, so the
  swap is atomic from SQLite's point of view.

## Tests

### Backend (594 total: 587 lib + 6 integration + 1 doc, 3 ignored)

| Suite | Tests | Covers |
|---|---|---|
| `maintenance/integrity.rs` (in-module) | 4 | live battery verdicts + WAL, file battery over a snapshot, non-SQLite rejection, page-stat math |
| `maintenance/backup.rs` (in-module) | 3 | snapshot + checksum + ledger, on-demand backup dir creation, snapshot reflects live data |
| `maintenance/restore.rs` (in-module) | 5 | stage validate/copy/record + pending, junk/invalid rejection with failed ledger row, cancel, header detection, marker path |
| `maintenance/runner.rs` (in-module) | 3 | report + audit, database stays healthy after maintenance, vacuum policy gates |
| `maintenance/engine_tests.rs` | 6 | integrity report + audit, backup/list/restore/cancel round trip, non-backup restore rejection, maintenance audit, backup directory wiring, full launch cycle applies a staged restore |
| `repositories/maintenance_repository.rs` (in-module) | 6 | optimize/checkpoint no-ops, `vacuum_into` validity, path rejection, ledger round trip + by-id, latest-of-kind, quote escaping |
| `database/mod.rs` (in-module) | 2 | pending restore applied before the pool opens (with safety copy), no-op without a marker |
| M1/M2 suites | unchanged | all prior tests still green (the 587 lib total includes the M3 additions plus the full M1/M2 regression surface) |

### Frontend (116 total across 22 files)

| Suite | Tests | Covers |
|---|---|---|
| `services/maintenanceRepository.test.ts` | 8 | every `maintenance_*` IPC command + argument shape + singleton |
| `pages/MaintenancePage.test.tsx` | 6 | ledger render, backup action, restore/cancel flow, integrity tab, maintenance tab, staged-restore banner |
| `components/maintenance/BackupPanel.test.tsx` | 4 | ledger rows with badges/sizes/checksums, restore visibility, restore/backup/cancel callbacks, empty/loading/error |
| `components/maintenance/IntegrityPanel.test.tsx` | 2 | healthy verdict + stats, violation lines, run/state handling |
| `components/maintenance/MaintenancePanel.test.tsx` | 2 | before/after + recovery, vacuum/no-vacuum badges, run/state handling |

## Design decisions

- **Restores are staged and applied on next launch, never live.** Swapping
  a WAL database under an open pool corrupts the running session; staging a
  validated copy that `initialize_at` swaps in *before* the pool opens is
  honest and safe. The outgoing database is always preserved as a
  `chronodesk-pre-restore-*.db` safety copy, so a bad restore is manually
  reversible.
- **Backup via `VACUUM INTO`, not a file copy.** A raw copy of a live WAL
  database is inconsistent (rows live in `-wal`); `VACUUM INTO` produces a
  consistent, compacted, standalone file with no sidecars.
- **Validation uses the same queries as certification.** `check_file`
  runs the identical `quick_check`/`foreign_key_check` battery over the
  backup that `check_live` runs over the live database — one owner
  (`IntegrityChecker`), no drifted duplicate SQL.
- **VACUUM is gated, `optimize` is not.** A full file rewrite only runs
  when free pages are both substantial (≥ 64) and a meaningful share
  (≥ 10%); checkpoint + optimize always run. The gate is a pure,
  unit-tested policy function.
- **Every intervention is audited.** One `backup_runs` ledger records
  successes, failures, and staged restores with checksums — the M2
  audit-trail philosophy extended to the data layer. Failed stages record
  the rejection reason.
- **Ledger stores filenames, engine resolves paths.** A backup row stores
  the file name (not an absolute path); `MaintenanceEngine::restore` joins
  it against the configured backup directory, and the restore command
  takes a ledger id — the UI never fabricates filesystem paths.
- **Path inputs are validated at the boundary.** `vacuum_into` accepts
  only absolute, NUL-free paths and escapes single quotes before
  interpolating into SQL (SQLite's `VACUUM INTO` takes a literal, not a
  bound parameter).

## Trade-offs

- **Restores require a relaunch.** Staging applies on the next launch
  rather than immediately; the UI says so explicitly and offers Cancel.
  Immediate in-place swap would require quiescing every pool user — a
  bigger, riskier change than the milestone warrants.
- **`VACUUM INTO` briefly blocks writers.** SQLite serializes the
  snapshot against active writers for the duration of the copy; on a very
  large database this could pause writes for a moment. It is the standard
  safe online backup and requires no external tool.
- **Full `integrity_check` scans every page.** The full battery is
  deliberately a user-initiated action, not a startup cost; restore
  validation uses the cheaper `quick_check`.
- **Backups are managed, not auto-rotated.** This milestone creates
  snapshots and the audit ledger but does not prune old snapshots —
  deletion/retention is left to a later milestone (noted below).
- **The diagnostic battery lives outside the repository.** A single-pool
  repository cannot validate arbitrary files; the battery is centralized
  in `IntegrityChecker` (one owner) rather than duplicated across
  repository and service layers.

## Compatibility

- **No architecture rewrites.** `MaintenanceEngine` is a facade; commands
  are thin wrappers; dependency direction is unchanged (commands → engine
  → repository → models → database).
- **No breaking APIs, IPC, or schema.** One new migration (`0030`) creates
  one new table; no existing table was altered. Seven new `maintenance_*`
  commands; nothing existing changed signature. `DatabaseError` needed no
  new variants (existing `IoError`/`InvalidInput`/`NotFound`/`Connection`
  cover every new failure surface).
- **`initialize_at` behavior is additive**: the pending-restore swap is a
  no-op when no marker exists, so every existing test and the production
  first-launch path behave exactly as before.
- **Frontend is purely additive**: one route, one sidebar entry, new
  types/service/components. No existing page was modified.

## Quality gates

All gates ran clean on the final tree:

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass, 0 warnings |
| `cargo build` | pass |
| `cargo test` | 594 passed (587 lib + 6 integration + 1 doc), 3 ignored, 0 failed |
| `npm run build` | pass |
| `npx tsc -b` | pass |
| `npm test` | 116 passed, 0 failed |
| `npm run lint` | no new problems (13 pre-existing errors, 5 pre-existing warnings, same as M1/M2) |

## Remaining TODOs

- RC-10 M4 (production hardening): security hardening — secure-storage
  review, command/audit logging, secret handling validation.
- Optional: snapshot retention/rotation (auto-prune old backups with a
  configurable keep window) on top of the new `backup_runs` ledger.
- Optional: frontend live events for maintenance actions (currently the
  page reflects state on load/action only).
