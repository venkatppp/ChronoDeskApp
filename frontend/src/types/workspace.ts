/**
 * Domain model for a ChronoDesk Workspace.
 *
 * Mirrors the Rust backend's `Workspace` struct
 * (`src-tauri/src/models/workspace.rs`) exactly — field names, casing,
 * and optionality all match what `#[serde(rename_all = "camelCase")]`
 * produces over Tauri's IPC boundary. Keeping this type a 1:1 mirror
 * means a payload from `invoke("get_workspace", ...)` can be trusted as
 * this shape without a translation layer.
 */
export type WorkspaceStatus = "active" | "archived";

export interface Workspace {
  id: string;
  name: string;
  description: string | null;
  status: WorkspaceStatus;
  /** 0–100 composite health score, see blueprint §12. */
  healthScore: number;
  /**
   * Filesystem directory this workspace corresponds to, if it was
   * created by the Workspace Engine's detector rather than manually.
   */
  rootPath: string | null;
  lastActiveAt: string; // ISO-8601 / RFC 3339
  createdAt: string;
  updatedAt: string;
}

/** Payload for `invoke("create_workspace", { input })`. */
export interface CreateWorkspaceInput {
  name: string;
  description?: string | null;
}

/** Payload for `invoke("update_workspace", { id, input })`. Every field is optional — `None` leaves the column unchanged. Pass `description: ""` to clear. */
export interface UpdateWorkspaceInput {
  name?: string;
  description?: string | null;
  status?: WorkspaceStatus;
  healthScore?: number;
}

/**
 * A single item in the "Today's Briefing" / recommendation feed on the
 * dashboard (blueprint §3.2, Home Dashboard + Recommendations Panel).
 *
 * Computed client-side from real workspace/timeline data (see
 * `TauriWorkspaceRepository`) — ChronoDesk's backend Recommendation
 * Engine (blueprint §6) is a later phase; this is an honest, if simple,
 * heuristic layer over data that's actually real today, not a mock.
 */
export type RecommendationKind = "resume" | "archive" | "duplicate" | "deadline";

export interface Recommendation {
  id: string;
  kind: RecommendationKind;
  message: string;
  workspaceId?: string;
}
