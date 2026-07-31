import { useCallback, useEffect, useState } from "react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import { getTimelineRepository } from "@/services/timelineRepository";
import { getSessionRepository } from "@/services/sessionRepository";
import { getAnalyticsRepository } from "@/services/analyticsRepository";
import { getIntelligenceRepository } from "@/services/intelligenceRepository";
import { contextMemoryRepository } from "@/services/contextMemoryRepository";
import { predictiveRepository } from "@/services/predictiveRepository";
import { useAppEvents } from "@/hooks/useAppEvents";
import { DASHBOARD_REFRESH_EVENTS } from "@/utils/backendEvents";
import type { Workspace, WorkspaceStats, ProductivityBrief } from "@/types/workspace";
import type { Recommendation } from "@/types/intelligence";
import type { TimelineEvent } from "@/types/timeline";
import type { SessionSummary } from "@/types/session";
import type { DailyBriefing, DailySummary } from "@/types/analytics";
import type { ContextSnapshot, RelatedWorkspace } from "@/types/contextMemory";
import type { PredictionsSummary } from "@/types/predictive";

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
  latestSnapshot: ContextSnapshot | null;
  relatedWorkspaces: RelatedWorkspace[];
  predictions: PredictionsSummary | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

const INITIAL_STATE: Omit<DashboardData, 'refresh'> = {
  workspaces: [],
  briefing: null,
  recommendations: [],
  workspaceStats: {},
  recentActivity: [],
  smartResumeSession: null,
  dailyBriefing: null,
  todaySummary: null,
  yesterdaySummary: null,
  latestSnapshot: null,
  relatedWorkspaces: [],
  predictions: null,
  isLoading: true,
  error: null,
};

export function useDashboardData(): DashboardData {
  const [state, setState] = useState<Omit<DashboardData, 'refresh'>>(INITIAL_STATE);

  const load = useCallback(async () => {
    const workspaceRepository = getWorkspaceRepository();
    const timelineRepository = getTimelineRepository();
    const sessionRepository = getSessionRepository();
    const analyticsRepository = getAnalyticsRepository();
    const intelligenceRepository = getIntelligenceRepository();

    try {
      const [workspaces, briefing, smartResumeSession, dailyBriefing, todaySummary, yesterdaySummary] = await Promise.all([
        workspaceRepository.listActiveWorkspaces(),
        workspaceRepository.getBriefing(),
        sessionRepository.getSmartResumeSession().catch(() => null),
        analyticsRepository.getDailyBriefing().catch(() => null),
        analyticsRepository.getTodaySummary().catch(() => null),
        analyticsRepository.getYesterdaySummary().catch(() => null),
      ]);

      const sorted = [...workspaces].sort(
        (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
      );
      const mostRecentWorkspace = sorted[0];

      // Get recommendations from the first active workspace (or empty if none)
      let recommendations: Recommendation[] = [];
      if (mostRecentWorkspace) {
        try {
          // Convert UUID string to number for intelligence API
          // This is a temporary bridge - in production, the API should accept UUIDs
          const workspaceIdNumber = 1; // Placeholder: needs proper UUID to i64 conversion
          recommendations = await intelligenceRepository.getWorkspaceRecommendations(workspaceIdNumber);
        } catch (err) {
          console.warn("Failed to load recommendations:", err);
          recommendations = [];
        }
      }

      const [allStats, recentActivity, latestSnapshot, relatedWorkspaces, predictions] = await Promise.all([
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
        mostRecentWorkspace
          ? contextMemoryRepository.getLatestSnapshot(mostRecentWorkspace.id).catch(() => null)
          : Promise.resolve(null),
        mostRecentWorkspace
          ? contextMemoryRepository.getRelatedWorkspaces(mostRecentWorkspace.id, 0.2, 5).catch(() => [])
          : Promise.resolve([]),
        predictiveRepository.getPredictionsSummary().catch(() => null),
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
        latestSnapshot,
        relatedWorkspaces,
        predictions,
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

  return { ...state, refresh: load };
}
