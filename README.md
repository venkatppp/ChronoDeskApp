# ChronoDesk — Phase 1: Project Initialization

Status: **awaiting your approval before Phase 2**

## What this phase delivers

A working Tauri 2 + React 19 + TypeScript desktop app shell with:

- Full professional folder structure (`components/`, `features/`, `hooks/`, `services/`, `contexts/`, `layouts/`, `pages/`, `utils/`, `types/` on the frontend; domain-module scaffold on the backend)
- Tailwind CSS v4 with a dark-mode-first design token system (`src/index.css`) and a light-mode override
- A small hand-built shadcn/ui-style primitive set (`Button`, `Card`, `Badge`, `ProgressRing`) built on `class-variance-authority` + `tailwind-merge`, matching the JetBrains/Linear/Raycast-inspired brief
- Client-side routing (`react-router-dom`, `HashRouter` — required because Tauri serves the frontend from a custom asset protocol, not a real HTTP server with history-API fallback)
- Theme system (dark/light/system) via `ThemeContext` + `useTheme`, persisted to `localStorage`
- `AppLayout` composing a `Sidebar` (with the signature "time-thread" active-route indicator) and `Topbar` (global search input, theme toggle, notifications)
- A fully working **Dashboard** screen (briefing banner, workspace cards with a circular health-score "ProgressRing", recommendations panel) wired to mock data through a `WorkspaceRepository` interface — the same interface a future `TauriWorkspaceRepository` will implement in Phase 3, so no component changes when real IPC lands
- Placeholder screens for Workspaces / Timeline / Graph / Analytics / Settings, each labeled with the phase that implements it, so navigation is fully functional end-to-end today
- A minimal, real Tauri Rust backend (`src-tauri/`) with `commands::system::get_app_version` / `health_check` wired end-to-end, plus **empty, documented scaffold modules** (`database`, `watcher`, `workspace`, `timeline`, `search`, `graph`, `ml`) matching the blueprint's engine breakdown — each is a no-op today, populated in the phase noted in its doc comment

## Folder structure

```
chronodesk/
├── frontend/                      # Tauri's "distDir" — React + TS app
│   └── src/
│       ├── components/
│       │   ├── ui/                # Button, Card, Badge, ProgressRing, ComingSoon
│       │   └── navigation/        # Sidebar, NavItem, Topbar
│       ├── features/
│       │   └── dashboard/         # DashboardView + its components/hooks
│       ├── hooks/                 # useTheme
│       ├── services/              # WorkspaceRepository (Repository Pattern)
│       ├── contexts/              # ThemeContext
│       ├── layouts/               # AppLayout
│       ├── pages/                 # Route-level thin wrappers
│       ├── utils/                 # cn(), formatRelativeTime()
│       ├── types/                 # Workspace, Recommendation domain types
│       ├── App.tsx                # Route table
│       ├── main.tsx                # Entry point
│       └── index.css               # Design tokens (dark-first) + fonts
└── src-tauri/                     # Rust backend
    ├── src/
    │   ├── main.rs                 # Binary entry → chronodesk_lib::run()
    │   ├── lib.rs                  # Tauri Builder, plugin wiring, module map
    │   ├── commands/                # Tauri IPC handlers (system.rs today)
    │   ├── database/ watcher/ workspace/ timeline/ search/ graph/ ml/   # Phase 2–5 scaffolds
    ├── capabilities/default.json    # Least-privilege IPC permission set
    ├── icons/                       # Placeholder app icons (regenerate before shipping)
    ├── tauri.conf.json
    ├── build.rs
    └── Cargo.toml
```

## Architecture decisions carried over from the blueprint

- **Repository Pattern on the frontend** (`services/workspaceRepository.ts`): every feature reads data through a `WorkspaceRepository` interface, never a concrete data source. Phase 1 ships `MockWorkspaceRepository`; Phase 3 adds `TauriWorkspaceRepository` calling `@tauri-apps/api`'s `invoke`, swapped in at the single composition point (`getWorkspaceRepository()`).
- **Thin command handlers** (`commands/system.rs`): `#[tauri::command]` functions never contain business logic — they'll delegate to the relevant engine module once those exist.
- **Single-responsibility Rust modules**: the backend is pre-split into `database`, `watcher`, `workspace`, `timeline`, `search`, `graph`, `ml` exactly matching the blueprint's Software Architecture section, so later phases only add files, never restructure.
- **Local-first defaults**: fonts are self-hosted (`@fontsource/*`, Latin subset only — no runtime network calls), theme preference persists to `localStorage`, and the Tauri CSP is locked to `'self'` plus the asset protocol.

## How to run this

```bash
# 1. Install dependencies
cd chronodesk/frontend
pnpm install

# 2. Run the frontend alone (browser, for fast UI iteration)
pnpm dev              # http://localhost:1420

# 3. Run the full desktop app (requires the Rust toolchain + platform
#    prerequisites: https://v2.tauri.app/start/prerequisites/)
cd chronodesk/src-tauri
cargo tauri dev        # or: pnpm dlx @tauri-apps/cli dev
```

## Verification performed in this environment

| Check | Result |
|---|---|
| `pnpm run build` (tsc -b + vite build) | ✅ Passes clean, zero errors |
| `pnpm run lint` (ESLint, typescript-eslint, react-hooks) | ✅ 0 errors, 1 harmless fast-refresh warning (context file exports both a component and a context object — standard, intentional) |
| Rust stub modules (`database`, `watcher`, `workspace`, `timeline`, `search`, `graph`, `ml`) | ✅ Individually parsed clean with `rustc --edition 2021` |
| Full `cargo check` on `src-tauri` | ⚠️ **Could not be completed in this sandbox** — see note below |

### Rust backend verification note

This sandbox only has Ubuntu Noble's packaged `rustc 1.75` (Dec 2023) available, with no route to install a newer toolchain (rustup's download domains aren't reachable from here). As of mid-2026, several foundational crates in the ecosystem (`time`, `toml`/`toml_datetime`, and others pulled in transitively by `tauri`/`tauri-build`) have adopted Rust's `edition2024`, which requires `rustc >= 1.85`. I tried pinning the obvious offenders (`time`, `toml`) to older patch versions and pinning `tauri` itself to its original `2.0.0` release, but Cargo's resolver still floats *their* transitive dependencies to the latest crates.io versions unless the entire graph is pinned — which isn't practical to do by hand.

This is an environment limitation, not a defect in the code: `lib.rs`, `main.rs`, and `commands/system.rs` were written and manually cross-checked line-by-line against the documented Tauri 2 API (`tauri::Builder`, `tauri_plugin_log::Builder`, the `Manager` trait, `#[tauri::command]`, `tauri::generate_handler!`/`generate_context!`), and I caught and fixed one real bug this way (a missing direct `log` dependency for `log::LevelFilter`, which Rust won't resolve through a transitive crate). **Please run `cargo check` (or `cargo tauri dev`) once on your own machine with `rustup`-installed Rust ≥ 1.85** to get a fully compiler-verified result — I'd expect it to succeed, but I want to be upfront that I couldn't confirm it myself here.

## Testing strategy so far

- `commands/system.rs` includes `#[cfg(test)]` unit tests for both commands (`get_app_version_matches_cargo_manifest`, `health_check_reports_ok`) — run with `cargo test` once the toolchain gap above is resolved.
- No frontend test runner is wired up yet (no test framework installed). Recommend adding Vitest + React Testing Library in Phase 2 alongside the Database Layer, once there's real state logic worth unit-testing beyond presentational components.

## What's next (Phase 2 — proposed)

Per the blueprint's execution plan (§15.3), Phase 2 is the **Database Layer**: SQLite connection pool + migrations in `src-tauri/src/database/`, the `workspaces` / `artifacts` / `timeline_events` schema from blueprint §7.2, and the first real Tauri commands (`workspace::list_active`, etc.) replacing `MockWorkspaceRepository` on the frontend with `TauriWorkspaceRepository`. I'll wait for your go-ahead before starting it.
