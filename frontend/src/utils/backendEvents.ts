/**
 * Event name constants, mirroring `src-tauri/src/app_events.rs` exactly.
 * Kept as a single source of truth on this side too, rather than raw
 * string literals scattered across every component that listens.
 */
export const BACKEND_EVENTS = {
  workspaceCreated: "workspace:created",
  workspaceUpdated: "workspace:updated",
  workspaceDeleted: "workspace:deleted",
  fileChanged: "file:changed",
  timelineEventAdded: "timeline:event_added",
} as const;

/** Every event that should trigger a dashboard refresh. */
export const DASHBOARD_REFRESH_EVENTS: string[] = [
  BACKEND_EVENTS.workspaceCreated,
  BACKEND_EVENTS.workspaceUpdated,
  BACKEND_EVENTS.workspaceDeleted,
  BACKEND_EVENTS.timelineEventAdded,
];
