import { useCallback, useEffect, useState } from "react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import { getTimelineRepository } from "@/services/timelineRepository";
import { getSessionRepository } from "@/services/sessionRepository";
import { getAnalyticsRepository } from "@/services/analyticsRepository";
import { useAppEvents } from "@/hooks/useAppEvents";
import { DASHBOARD_REFRESH_EVENTS } from "@/utils/backendEvents";
import type { Recommendation, Workspace, WorkspaceStats, ProductivityBrief } from "@/types/workspace";
import type { TimelineEvent } from "@/types/timeline";
import type { SessionSummary } from "@/types/session";
import type { DailyBriefing, DailySummary } from "@/types/analytics";

interface DashboardData {
  workspaces: Workspace[];
  briefing: ProductivityBrief | null;
  recommendations: Recommendation[];
  workspaceStats: Record<string, WorkspaceStats>;
  recentActivity: TimelineEvent[];
  smartResumeSession: SessionSummary | null;
  dailyBriefing: DailyBriefing | null;
  todaySummary: DailySummary | null;
  yesterdaySummary: DailySummary | null;
  isLoading: boolean;
  error: string | null;
}

const INITIAL_STATE: DashboardData = {
  workspaces: [],
  briefing: null,
  recommendations: [],
  workspaceStats: {},
  recentActivity: [],
  smartResumeSession: null,
  dailyBriefing: null,
  todaySummary: null,
  yesterdaySummary: null,
  isLoading: true,
  error: null,
};

export function useDashboardData(): DashboardData {
  const [state, setState] = useState<DashboardData>(INITIAL_STATE);

  const load = useCallback(async () => {
    const workspaceRepository = getWorkspaceRepository();
    const timelineRepository = getTimelineRepository();
    const sessionRepository = getSessionRepository();
    const analyticsRepository = getAnalyticsRepository();

    try {
      const [workspaces, briefing, recommendations, smartResumeSession, dailyBriefing, todaySummary, yesterdaySummary] = await Promise.all([
        workspaceRepository.listActiveWorkspaces(),
        workspaceRepository.getBriefing(),
        workspaceRepository.listRecommendations(),
        sessionRepository.getSmartResumeSession().catch(() => null),
        analyticsRepository.getDailyBriefing().catch(() => null),
        analyticsRepository.getTodaySummary().catch(() => null),
        analyticsRepository.getYesterdaySummary().catch(() => null),
      ]);

      const sorted = [...workspaces].sort(
        (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
      );
      const mostRecentWorkspace = sorted[0];

      const [allStats, recentActivity] = await Promise.all([
        workspaces.length > 0
          ? Promise.all(
              workspaces.map((w) =>
                workspaceRepository.getDashboardStats(w.id).catch(() => null),
              ),
            )
          : Promise.resolve([]),
        mostRecentWorkspace
          ? timelineRepository.getRecentActivity(mostRecentWorkspace.id)
          : Promise.resolve([] as TimelineEvent[]),
      ]);

      const statsMap: Record<string, WorkspaceStats> = {};
      if (allStats.length > 0) {
        for (let i = 0; i < workspaces.length; i++) {
          const s = allStats[i];
          if (s) statsMap[workspaces[i].id] = s;
        }
      }

      setState({
        workspaces,
        briefing,
        recommendations,
        workspaceStats: statsMap,
        recentActivity,
        smartResumeSession,
        dailyBriefing,
        todaySummary,
        yesterdaySummary,
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

  useAppEvents(DASHBOARD_REFRESH_EVENTS, () => {
    void load();
  });

  return state;
}
