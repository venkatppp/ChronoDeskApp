/**
 * Event name constants, mirroring `src-tauri/src/app_events.rs` exactly.
 * Kept as a single source of truth on this side too, rather than raw
 * string literals scattered across every component that listens.
 */
export const BACKEND_EVENTS = {
  workspaceCreated: "workspace:created",
  workspaceUpdated: "workspace:updated",
  workspaceDeleted: "workspace:deleted",
  workspaceSwitched: "workspace:switched",
  fileChanged: "file:changed",
  timelineEventAdded: "timeline:event_added",
  sessionStarted: "session:started",
  sessionEnded: "session:ended",
  workflowChanged: "workflow:changed",
  actionExecuted: "action:executed",
  predictionUpdated: "prediction:updated",
  recommendationUpdated: "recommendation:updated",
  healthUpdated: "health:updated",
  snapshotCreated: "snapshot:created",
  searchIndexed: "search:indexed",
  graphEdgeAdded: "graph:edge_added",
} as const;

/** Every event that should trigger a dashboard refresh. */
export const DASHBOARD_REFRESH_EVENTS: string[] = [
  BACKEND_EVENTS.workspaceCreated,
  BACKEND_EVENTS.workspaceUpdated,
  BACKEND_EVENTS.workspaceDeleted,
  BACKEND_EVENTS.workspaceSwitched,
  BACKEND_EVENTS.timelineEventAdded,
  BACKEND_EVENTS.sessionStarted,
  BACKEND_EVENTS.sessionEnded,
  BACKEND_EVENTS.workflowChanged,
  BACKEND_EVENTS.actionExecuted,
  BACKEND_EVENTS.predictionUpdated,
  BACKEND_EVENTS.recommendationUpdated,
  BACKEND_EVENTS.healthUpdated,
  BACKEND_EVENTS.snapshotCreated,
];
