import { useCallback, useEffect, useState } from "react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import { getTimelineRepository } from "@/services/timelineRepository";
import { useAppEvents } from "@/hooks/useAppEvents";
import { DASHBOARD_REFRESH_EVENTS } from "@/utils/backendEvents";
import type { Recommendation, Workspace } from "@/types/workspace";
import type { TimelineEvent } from "@/types/timeline";

interface DashboardData {
  workspaces: Workspace[];
  briefing: string | null;
  recommendations: Recommendation[];
  /** Recent timeline events for the most-recently-active workspace, if any. */
  recentActivity: TimelineEvent[];
  isLoading: boolean;
  error: string | null;
}

const INITIAL_STATE: DashboardData = {
  workspaces: [],
  briefing: null,
  recommendations: [],
  recentActivity: [],
  isLoading: true,
  error: null,
};

/**
 * Loads everything the Home Dashboard needs (blueprint §3.2) through the
 * repository abstraction, and keeps it live: subscribed to every backend
 * event that could mean the dashboard is stale
 * (`DASHBOARD_REFRESH_EVENTS`, emitted by the watcher pipeline and the
 * workspace commands — see `src-tauri/src/app_events.rs`), so a file
 * change on disk or a workspace edit from another window reaches this
 * screen with no manual refresh. Component code stays purely
 * presentational.
 */
export function useDashboardData(): DashboardData {
  const [state, setState] = useState<DashboardData>(INITIAL_STATE);

  const load = useCallback(async () => {
    const workspaceRepository = getWorkspaceRepository();
    const timelineRepository = getTimelineRepository();

    try {
      const [workspaces, briefing, recommendations] = await Promise.all([
        workspaceRepository.listActiveWorkspaces(),
        workspaceRepository.getBriefing(),
        workspaceRepository.listRecommendations(),
      ]);

      const mostRecentWorkspace = [...workspaces].sort(
        (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
      )[0];

      const recentActivity = mostRecentWorkspace
        ? await timelineRepository.getRecentActivity(mostRecentWorkspace.id)
        : [];

      setState({
        workspaces,
        briefing,
        recommendations,
        recentActivity,
        isLoading: false,
        error: null,
      });
    } catch (err) {
      setState((prev) => ({
        ...prev,
        isLoading: false,
        error: err instanceof Error ? err.message : "Failed to load dashboard data.",
      }));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // Re-run the load whenever the backend reports something changed —
  // this is what makes the dashboard "just update" per Phase 3's
  // "no manual refresh" requirement, rather than polling on a timer.
  useAppEvents(DASHBOARD_REFRESH_EVENTS, () => {
    void load();
  });

  return state;
}
