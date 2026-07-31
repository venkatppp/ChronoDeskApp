//! Tauri IPC command handlers, grouped by domain.
//!
//! Each submodule owns one cohesive set of `#[tauri::command]` functions.
//! `lib.rs` only ever imports from here — no `#[tauri::command]` function
//! is defined outside this module, so the full surface area callable from
//! the frontend is always visible in one place.

pub mod duplicates;
pub mod graph;
pub mod search;
pub mod session;
pub mod system;
pub mod timeline;
pub mod watcher;
pub mod workspace;
