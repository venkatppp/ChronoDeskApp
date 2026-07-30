import { invoke } from "@tauri-apps/api/core";
import type { CreateWorkspaceInput, Recommendation, UpdateWorkspaceInput, Workspace } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

/**
 * Repository Pattern boundary between the UI and the data source.
 *
 * `TauriWorkspaceRepository` (below) is the only implementation as of
 * Phase 3 — the Phase 1 `MockWorkspaceRepository` has been removed
 * entirely now that the real `commands::workspace::*` IPC commands exist
 * (blueprint's Workspace Engine, shipped Phase 3). No component outside
 * this file changed: they only ever depended on this interface.
 */
export interface WorkspaceRepository {
  listActiveWorkspaces(): Promise<Workspace[]>;
  listArchivedWorkspaces(): Promise<Workspace[]>;
  getWorkspace(id: string): Promise<Workspace>;
  createWorkspace(input: CreateWorkspaceInput): Promise<Workspace>;
  updateWorkspace(id: string, input: UpdateWorkspaceInput): Promise<Workspace>;
  deleteWorkspace(id: string): Promise<void>;
  switchWorkspace(id: string): Promise<void>;
  getBriefing(): Promise<string>;
  listRecommendations(): Promise<Recommendation[]>;
}

/** A workspace untouched for this long is flagged as "resume" material. */
const IDLE_RESUME_THRESHOLD_DAYS = 4;
/** A workspace untouched for this long is flagged for archiving instead. */
const IDLE_ARCHIVE_THRESHOLD_DAYS = 60;

function daysSince(iso: string): number {
  return (Date.now() - new Date(iso).getTime()) / (24 * 60 * 60 * 1000);
}

/**
 * Talks to the real Rust backend over Tauri's IPC boundary.
 *
 * `getBriefing` and `listRecommendations` have no dedicated backend
 * command yet (blueprint's Recommendation Engine is a later phase) —
 * rather than fabricate copy the way the Phase 1 mock did, both are
 * computed here from the real workspace list `listActiveWorkspaces`
 * already fetched, so every word on the dashboard traces back to an
 * actual row in SQLite.
 */
export class TauriWorkspaceRepository implements WorkspaceRepository {
  async listActiveWorkspaces(): Promise<Workspace[]> {
    return invoke<Workspace[]>("list_active_workspaces");
  }

  async listArchivedWorkspaces(): Promise<Workspace[]> {
    return invoke<Workspace[]>("list_archived_workspaces");
  }

  async getWorkspace(id: string): Promise<Workspace> {
    return invoke<Workspace>("get_workspace", { id });
  }

  async createWorkspace(input: CreateWorkspaceInput): Promise<Workspace> {
    return invoke<Workspace>("create_workspace", { input });
  }

  async updateWorkspace(id: string, input: UpdateWorkspaceInput): Promise<Workspace> {
    return invoke<Workspace>("update_workspace", { id, input });
  }

  async deleteWorkspace(id: string): Promise<void> {
    await invoke<void>("delete_workspace", { id });
  }

  async switchWorkspace(id: string): Promise<void> {
    await invoke<void>("switch_workspace", { id });
  }

  async getBriefing(): Promise<string> {
    const workspaces = await this.listActiveWorkspaces();

    if (workspaces.length === 0) {
      return "No active workspaces yet — add a folder to watch to get started.";
    }

    const mostRecent = [...workspaces].sort(
      (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
    )[0];

    const count = workspaces.length === 1 ? "1 active workspace" : `${workspaces.length} active workspaces`;
    return `You have ${count}. "${mostRecent.name}" was last active ${formatRelativeTime(mostRecent.lastActiveAt)}.`;
  }

  async listRecommendations(): Promise<Recommendation[]> {
    const workspaces = await this.listActiveWorkspaces();
    const recommendations: Recommendation[] = [];

    for (const workspace of workspaces) {
      const idleDays = daysSince(workspace.lastActiveAt);

      if (idleDays >= IDLE_ARCHIVE_THRESHOLD_DAYS) {
        recommendations.push({
          id: `archive-${workspace.id}`,
          kind: "archive",
          message: `Archive "${workspace.name}" — idle ${Math.floor(idleDays)} days`,
          workspaceId: workspace.id,
        });
      } else if (idleDays >= IDLE_RESUME_THRESHOLD_DAYS) {
        recommendations.push({
          id: `resume-${workspace.id}`,
          kind: "resume",
          message: `Resume "${workspace.name}" — idle ${Math.floor(idleDays)} days`,
          workspaceId: workspace.id,
        });
      }
    }

    return recommendations;
  }
}

let repositoryInstance: WorkspaceRepository | null = null;

/**
 * Composition-root accessor. Everything in `features/` imports this
 * function rather than instantiating a repository directly, so swapping
 * the implementation (e.g. for a future test double) is a one-line change.
 */
export function getWorkspaceRepository(): WorkspaceRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriWorkspaceRepository();
  }
  return repositoryInstance;
}
