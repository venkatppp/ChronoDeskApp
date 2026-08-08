import { invoke } from "@tauri-apps/api/core";
import type { CreateWorkspaceInput, UpdateWorkspaceInput, Workspace, WorkspaceStats, ProductivityBrief } from "@/types/workspace";
import type { Recommendation } from "@/types/intelligence";
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
  getDashboardStats(workspaceId: string): Promise<WorkspaceStats>;
  createWorkspace(input: CreateWorkspaceInput): Promise<Workspace>;
  updateWorkspace(id: string, input: UpdateWorkspaceInput): Promise<Workspace>;
  deleteWorkspace(id: string): Promise<void>;
  switchWorkspace(id: string): Promise<void>;
  openFile(path: string): Promise<void>;
  getBriefing(): Promise<ProductivityBrief>;
  listRecommendations(): Promise<Recommendation[]>;
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

  async getDashboardStats(workspaceId: string): Promise<WorkspaceStats> {
    return invoke<WorkspaceStats>("get_workspace_statistics", { workspaceId });
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

  async openFile(path: string): Promise<void> {
    await invoke<void>("open_file", { path });
  }

  async getBriefing(): Promise<ProductivityBrief> {
    const workspaces = await this.listActiveWorkspaces();

    const now = new Date();
    const hour = now.getHours();
    const greeting = hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";

    if (workspaces.length === 0) {
      return {
        greeting,
        lastActiveRelative: null,
        workspacesCount: 0,
        healthyCount: 0,
        attentionCount: 0,
        topWorkspaceName: null,
        topWorkspaceHealth: 0,
        attentionWorkspaces: [],
        todayEventsCount: 0,
        hoursWorked: 0,
        filesEdited: 0,
        mostActiveLanguage: null,
        mostEditedFile: null,
      };
    }

    const sorted = [...workspaces].sort(
      (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
    );

    const mostRecent = sorted[0];
    const healthy = workspaces.filter((w) => w.healthScore >= 70);
    const attention = workspaces.filter((w) => w.healthScore < 50);

    // Real measured activity for today comes from the analytics engine
    // (session durations summed from the timeline, distinct files,
    // edit/commit counts) — never approximated from event counts.
    let hoursWorked = 0;
    let filesEdited = 0;
    let mostActiveLanguage: string | null = null;

    try {
      const today = await invoke<{
        totalDurationSeconds?: number;
        fileCount?: number;
        editCount?: number;
        languages?: { language: string; percentage: number }[];
      }>("get_today_summary");
      hoursWorked = Math.round((today.totalDurationSeconds ?? 0) / 3600);
      filesEdited = today.editCount ?? today.fileCount ?? 0;
      const langs = today.languages ?? [];
      if (langs.length > 0) {
        mostActiveLanguage = langs[0].language;
      }
    } catch {
      // Non-critical: default values will be used
    }

    return {
      greeting,
      lastActiveRelative: formatRelativeTime(mostRecent.lastActiveAt),
      workspacesCount: workspaces.length,
      healthyCount: healthy.length,
      attentionCount: attention.length,
      topWorkspaceName: mostRecent.name,
      topWorkspaceHealth: Math.round(mostRecent.healthScore),
      attentionWorkspaces: attention.map((w) => ({ name: w.name, health: Math.round(w.healthScore) })),
      todayEventsCount: 0,
      hoursWorked,
      filesEdited,
      mostActiveLanguage,
      mostEditedFile: null,
    };
  }

  async listRecommendations(): Promise<Recommendation[]> {
    // This method is deprecated - use IntelligenceRepository instead
    // Returning empty array for backward compatibility
    return [];
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
