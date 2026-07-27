//! Frontend event emission (blueprint's "Frontend Refresh" pipeline
//! stage): lets the backend push real-time updates instead of the
//! frontend polling.
//!
//! Defined as a small trait rather than depending on `tauri::AppHandle`
//! directly everywhere an event needs emitting, so [`crate::watcher::FileWatcher`]
//! — a background-task-owning type whose test suite predates any
//! specific Tauri wiring decision — doesn't have to hard-depend on a
//! running Tauri app to be unit-tested. [`NoopEmitter`] backs every test
//! in this crate; `lib.rs` wires the real [`tauri::AppHandle`]
//! implementation in production.

use serde::Serialize;

/// Emits a named event with a JSON-serializable payload to every
/// listening frontend window.
pub trait AppEventEmitter: Send + Sync {
    fn emit_event(&self, event: &str, payload: serde_json::Value);
}

impl AppEventEmitter for tauri::AppHandle {
    fn emit_event(&self, event: &str, payload: serde_json::Value) {
        use tauri::Emitter;
        if let Err(err) = self.emit(event, payload) {
            tracing::warn!(event, error = %err, "failed to emit frontend event");
        }
    }
}

/// Event name constants — the exact strings the frontend's
/// `@tauri-apps/api/event` `listen(...)` calls match against.
pub const EVENT_WORKSPACE_CREATED: &str = "workspace:created";
pub const EVENT_WORKSPACE_UPDATED: &str = "workspace:updated";
pub const EVENT_WORKSPACE_DELETED: &str = "workspace:deleted";
pub const EVENT_FILE_CHANGED: &str = "file:changed";
pub const EVENT_TIMELINE_EVENT_ADDED: &str = "timeline:event_added";
pub const EVENT_SEARCH_INDEXED: &str = "search:indexed";
pub const EVENT_GRAPH_EDGE_ADDED: &str = "graph:edge_added";

/// Serializes `payload` and emits it. Serialization failure is logged,
/// not propagated — event emission is always best-effort and must never
/// fail the operation that triggered it (e.g. a workspace creation
/// succeeding in the database must not roll back or error out just
/// because notifying the frontend about it failed).
pub fn emit<T: Serialize>(emitter: &dyn AppEventEmitter, event: &str, payload: &T) {
    match serde_json::to_value(payload) {
        Ok(value) => emitter.emit_event(event, value),
        Err(err) => tracing::warn!(event, error = %err, "failed to serialize event payload"),
    }
}

/// No-op emitter backing every test in this crate, so tests exercise
/// event-emitting code paths without needing a running Tauri app.
#[derive(Debug, Clone, Default)]
pub struct NoopEmitter;

impl AppEventEmitter for NoopEmitter {
    fn emit_event(&self, _event: &str, _payload: serde_json::Value) {}
}
