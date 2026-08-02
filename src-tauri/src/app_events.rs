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
pub const EVENT_WORKSPACE_SWITCHED: &str = "workspace:switched";
pub const EVENT_FILE_CHANGED: &str = "file:changed";
pub const EVENT_TIMELINE_EVENT_ADDED: &str = "timeline:event_added";
pub const EVENT_SEARCH_INDEXED: &str = "search:indexed";
pub const EVENT_GRAPH_EDGE_ADDED: &str = "graph:edge_added";

// Session events
pub const EVENT_SESSION_STARTED: &str = "session:started";
pub const EVENT_SESSION_ENDED: &str = "session:ended";

// Intelligence events
pub const EVENT_WORKFLOW_CHANGED: &str = "workflow:changed";
pub const EVENT_PREDICTION_UPDATED: &str = "prediction:updated";
pub const EVENT_RECOMMENDATION_UPDATED: &str = "recommendation:updated";
pub const EVENT_HEALTH_UPDATED: &str = "health:updated";

// Context memory events
pub const EVENT_SNAPSHOT_CREATED: &str = "snapshot:created";

// Action events
pub const EVENT_ACTION_EXECUTED: &str = "action:executed";

// Proactive AI events
pub const EVENT_PROACTIVE_NOTIFICATION: &str = "proactive:notification";
pub const EVENT_RESUME_CONTEXT_READY: &str = "proactive:resume_context_ready";
pub const EVENT_PLAN_GENERATED: &str = "proactive:plan_generated";
pub const EVENT_AUTOMATION_REQUEST: &str = "proactive:automation_request";

// Plan execution events
pub const EVENT_EXECUTION_PROGRESS: &str = "execution:progress";

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
