import { invoke } from "@tauri-apps/api/core";
import type { TimelineEvent } from "@/types/timeline";

/**
 * Repository Pattern boundary for timeline data, mirroring
 * `WorkspaceRepository`. Kept as its own file/interface — not folded
 * into `WorkspaceRepository` — for the same reason the backend keeps
 * `TimelineRepository` and `WorkspaceRepository` separate: they're
 * different aggregates with different query shapes (pagination, a
 * per-workspace feed) that don't belong on the same interface.
 */
export interface TimelineRepository {
  listWorkspaceTimeline(workspaceId: string, limit?: number): Promise<TimelineEvent[]>;
  getRecentActivity(workspaceId: string): Promise<TimelineEvent[]>;
}

export class TauriTimelineRepository implements TimelineRepository {
  async listWorkspaceTimeline(workspaceId: string, limit?: number): Promise<TimelineEvent[]> {
    return invoke<TimelineEvent[]>("list_workspace_timeline", { workspaceId, limit: limit ?? null });
  }

  async getRecentActivity(workspaceId: string): Promise<TimelineEvent[]> {
    return invoke<TimelineEvent[]>("get_recent_activity", { workspaceId });
  }
}

let repositoryInstance: TimelineRepository | null = null;

/** Composition-root accessor, same pattern as `getWorkspaceRepository`. */
export function getTimelineRepository(): TimelineRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriTimelineRepository();
  }
  return repositoryInstance;
}
