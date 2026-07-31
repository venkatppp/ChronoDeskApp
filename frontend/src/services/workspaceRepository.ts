import { invoke } from "@tauri-apps/api/core";
import type { CreateWorkspaceInput, Recommendation, UpdateWorkspaceInput, Workspace, WorkspaceStats, ProductivityBrief } from "@/types/workspace";
import type { TimelineEvent } from "@/types/timeline";
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

    let hoursWorked = 0;
    let filesEdited = 0;
    let mostEditedFile: string | null = null;
    let mostActiveLanguage: string | null = null;

    try {
      const today = now.toISOString().slice(0, 10);
      const timeline = await invoke<TimelineEvent[]>("list_workspace_timeline", {
        workspaceId: mostRecent.id,
        limit: 200,
      });
      const todayEvents = timeline.filter((e) => e.occurredAt.slice(0, 10) === today);
      const edits = todayEvents.filter((e) => e.eventType === "edit");
      filesEdited = edits.length;

      const editFiles = edits
        .map((e) => e.fileId)
        .filter(Boolean) as string[];
      if (editFiles.length > 0) {
        const freq: Record<string, number> = {};
        for (const f of editFiles) freq[f] = (freq[f] || 0) + 1;
        mostEditedFile = Object.entries(freq).sort((a, b) => b[1] - a[1])[0]?.[0] ?? null;
      }

      const early = new Date(now);
      early.setHours(0, 0, 0, 0);
      let totalMs = 0;
      for (const e of todayEvents) {
        const ts = new Date(e.occurredAt).getTime();
        if (ts >= early.getTime()) {
          totalMs += 60000;
        }
      }
      hoursWorked = Math.round(totalMs / 3600000);
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
      mostEditedFile,
    };
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
          message: `Archive "${workspace.name}"`,
          workspaceId: workspace.id,
          priority: 1,
          reason: `Idle ${Math.floor(idleDays)} days — no activity detected`,
          estimatedEffort: "quick",
          expectedImpact: "medium",
          category: "maintenance",
        });
      } else if (workspace.healthScore < 50) {
        recommendations.push({
          id: `health-${workspace.id}`,
          kind: "attention",
          message: `Review "${workspace.name}" health`,
          workspaceId: workspace.id,
          priority: 2,
          reason: `Health score is ${Math.round(workspace.healthScore)}% — may need restructuring`,
          estimatedEffort: "moderate",
          expectedImpact: "high",
          category: "health",
        });
      } else if (idleDays >= IDLE_RESUME_THRESHOLD_DAYS) {
        recommendations.push({
          id: `resume-${workspace.id}`,
          kind: "resume",
          message: `Resume "${workspace.name}"`,
          workspaceId: workspace.id,
          priority: 3,
          reason: `Last active ${Math.floor(idleDays)} days ago`,
          estimatedEffort: "quick",
          expectedImpact: "medium",
          category: "productivity",
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
