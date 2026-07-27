# ChronoDesk Architecture

## Layering

```
commands   ── Tauri IPC handlers. Thin: pull state, call one service/engine
   │           method, return. No SQL, no business rules.
   ▼
engines    ── workspace::WorkspaceManager, timeline::TimelineEngine.
   │           Pipeline-facing facades: the objects FileWatcher and
   │           commands hold. Orchestrate services + (for WorkspaceManager)
   │           the detection heuristics.
   ▼
services   ── WorkspaceService, TimelineService. Business logic composing
   │           one or more repositories (e.g. "creating a workspace also
   │           records a timeline event" lives here, not in either
   │           repository).
   ▼
repositories ── One per aggregate (Workspace, File, Timeline, Settings).
   │             The only layer that writes SQL. Return typed models or
   │             DatabaseError. Independently unit-testable against a
   │             tempfile-backed database.
   ▼
database   ── Database facade: connection pool (WAL, foreign keys on) +
   │           migration runner. The only thing that opens a raw
   │           SqlitePool.
   ▼
SQLite (WAL mode)
```

`models` (typed domain structs + DTOs) and `errors` (DatabaseError,
WatcherError) are used by every layer above `database` and sit outside
this vertical stack. `app_events` is the one deliberate cross-cutting
exception — reached from both `commands` (Tauri-aware) and `watcher` (a
background engine), since both are where a user-visible change actually
happens and needs to reach the frontend.

**Rule enforced by convention** (not a compiler check, but consistently
followed across every module): a layer only ever calls the layer
directly below it. `commands` never imports a repository directly;
`workspace::WorkspaceManager` never runs SQL; `services` never touch
`tauri` types.

## Module dependency graph

```
commands/{system,workspace,timeline,watcher}
        |
        +--> services::{WorkspaceService, TimelineService}
        +--> timeline::TimelineEngine
        +--> watcher::FileWatcher
                  |
                  +--> workspace::WorkspaceManager --> services::WorkspaceService
                  +--> timeline::TimelineEngine --> services::TimelineService
                  +--> watcher::debounce::Debouncer   (no downward deps -- pure)
                  +--> watcher::event_handler          (no downward deps -- pure, notify-typed only)
                  +--> app_events::AppEventEmitter     (trait; tauri::AppHandle impl, NoopEmitter for tests)

services::WorkspaceService --> repositories::{WorkspaceRepository, TimelineRepository}
services::TimelineService  --> timeline::recorder::TimelineRecorder --> repositories::{FileRepository, TimelineRepository}
                           \--> repositories::TimelineRepository

repositories::* --> database::Database (via SqlitePool) + models::*

database::Database --> migrations/*.sql (embedded at compile time via sqlx::migrate!)
```

No cycles. `workspace` and `timeline` never depend on `watcher` —
`watcher` depends on them, one-directionally, which is why
`WorkspaceManager`/`TimelineEngine` are independently unit-testable
without spinning up a filesystem watch.

## Key design decisions

- **Repository row-decoding pattern.** SQLite has no enum type, so every
  enum-bearing model (`Workspace`, `FileArtifact`, `TimelineEvent`) has a
  private `*Row` struct (raw `String` for the enum column) plus a
  `TryFrom<Row>` conversion, rather than hand-writing `sqlx::Decode` for
  each enum. Chosen for compile-verifiability risk reduction as much as
  style: this sandbox cannot run `cargo check` (see `PROJECT_STATE.md`),
  so avoiding custom trait implementations in favor of a pattern backed
  entirely by well-established `sqlx` derive/feature behavior
  (`chrono`/`uuid` features, `#[derive(FromRow)]`) minimizes surface
  area for an undetectable API-usage mistake.
- **Runtime-checked SQL, not `sqlx::query!`.** The compile-time-checked
  macros need a reachable database (or an offline `.sqlx` cache) at
  build time. Every query in this codebase is the runtime `sqlx::query`/
  `query_as`, trading compile-time column-name checking for zero
  build-time database dependency — the right trade for a project other
  people will `git clone` and build without a pre-seeded dev database.
- **`TimelineActivity` vs. `TimelineEventType`.** Deliberately two
  enums. `TimelineEventType` is small and matches the database `CHECK`
  constraint exactly (widening it needs a migration — see `0003`).
  `TimelineActivity` is the domain vocabulary Phase 3's spec calls for
  (`WorkspaceRenamed`, `ProjectImported`, etc.) and maps many-to-one onto
  the storage enum via one exhaustive `match` in `timeline::events`, so
  adding a new domain activity without updating that mapping is a
  compile error, not a silent gap.
- **`WorkspaceManager`/`TimelineEngine` symmetry.** Both wrap their
  respective `*Service` rather than a raw repository, and both are the
  type the watcher pipeline holds — kept structurally identical
  on purpose (see the module-level doc comment in each) so the pattern
  only has to be learned once.
- **`AppEventEmitter` as a trait, not a hard `tauri::AppHandle`
  dependency.** `FileWatcher`'s test suite predates any Tauri-specific
  wiring decision and needs to keep working without a running Tauri app;
  the trait plus `NoopEmitter` default achieves that while
  `with_event_emitter()` swaps in the real implementation in `lib.rs`.
- **Repositories never expose their pool.** `tests/backend_integration.rs`
  stores the pool itself on its local `FullStack` test struct rather than
  reaching into a repository for it, keeping "no pool accessor" a rule
  with no test-only exception carved into production code.

## Async runtime notes

- Tauri's `setup()` hook is synchronous; `Database::initialize` and
  `commands::watcher::restore_watched_paths` are async (sqlx/tokio). Both
  are bridged with `tauri::async_runtime::block_on`, which is the
  documented, standard way to run async setup work to completion before
  a Tauri app's window is created — not a workaround.
- `FileWatcher::watch`/`unwatch` use `tokio::sync::Mutex` (not
  `std::sync::Mutex`) around the active-watch registry, since the guard
  needs to be safely held across `.await` points in principle even
  though today's critical sections happen to be short — the async-aware
  primitive is the correct default regardless.
- Each watched root gets three independent tokio tasks (OS watch +
  reconnect, intake/normalize, debounce-drain + record), decoupled via
  unbounded mpsc channels rather than a single monolithic loop, so a slow
  database write during the recording stage can never cause the OS watch
  callback (which runs on `notify`'s own thread and must stay responsive)
  to block or drop events.
