//! Timeline Engine (blueprint §4.2, §10).
//!
//! Owns the domain vocabulary of significant actions ([`events`]),
//! recording them with automatic file-row bookkeeping ([`recorder`]),
//! and the public facade the watcher pipeline and commands hold
//! ([`engine`]). Events are never mutated after being written — there is
//! no "update" anywhere in this module, only `record`.
//!
//! **Status:** Phase 3 ✅.

pub mod engine;
pub mod events;
pub mod recorder;

pub use engine::TimelineEngine;
pub use events::TimelineActivity;
