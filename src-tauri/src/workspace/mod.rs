//! Workspace Engine (blueprint §4.2).
//!
//! Owns workspace lifecycle: detecting workspace boundaries from
//! filesystem activity ([`heuristics`], [`detector`]) and orchestrating
//! creation/reactivation through [`crate::services::WorkspaceService`]
//! ([`manager`]). This is the module the file watcher pipeline calls on
//! every relevant filesystem event.
//!
//! **Status:** Phase 3 ✅ — heuristic-based detection. Later phases may
//! add an ML-driven clustering signal (blueprint §6) alongside these
//! heuristics, not in place of them; a git repo is a git repo regardless
//! of what a clustering model thinks.

pub mod detector;
pub mod heuristics;
pub mod manager;

pub use manager::WorkspaceManager;
