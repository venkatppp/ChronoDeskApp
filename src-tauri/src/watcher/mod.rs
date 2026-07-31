//! File Watcher (blueprint §4.2, §11).
//!
//! Wraps the `notify` crate to stream OS-level file create/modify/delete/
//! rename events, scoped to user-opted-in folders only, through
//! [`debounce`] and [`event_handler`]'s normalization/ignore-filtering
//! into the Workspace and Timeline Engines. Runs entirely on background
//! tokio tasks so it never blocks the Tauri event loop, and reconnects
//! automatically if the underlying OS watch fails.
//!
//! **Status:** Phase 3 ✅.

pub mod debounce;
pub mod event_handler;
pub mod file_watcher;

pub use file_watcher::FileWatcher;
