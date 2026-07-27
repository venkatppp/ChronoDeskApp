/**
 * Mirrors the Rust backend's `TimelineEventType`
 * (`src-tauri/src/models/timeline.rs`) — the storage-level vocabulary,
 * matching the database's `CHECK` constraint exactly.
 */
export type TimelineEventType =
  | "create"
  | "open"
  | "close"
  | "edit"
  | "move"
  | "delete"
  | "commit"
  | "visit"
  | "screenshot"
  | "workspace_switch";

/** Mirrors the Rust backend's `TimelineEvent` struct. */
export interface TimelineEvent {
  id: string;
  workspaceId: string;
  fileId: string | null;
  eventType: TimelineEventType;
  occurredAt: string; // ISO-8601 / RFC 3339
  metadata: Record<string, unknown> | null;
  createdAt: string;
}
