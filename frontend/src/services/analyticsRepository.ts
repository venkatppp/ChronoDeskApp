// Analytics Repository - IPC wrapper for analytics commands

import { invoke } from "@tauri-apps/api/core";
import type {
  DailyBriefing,
  DailySummary,
  WeeklySummary,
  MonthlySummary,
  WorkspaceInsight,
} from "@/types/analytics";

export class AnalyticsRepository {
  async getDailyBriefing(): Promise<DailyBriefing> {
    return invoke<DailyBriefing>("get_daily_briefing");
  }

  async getTodaySummary(): Promise<DailySummary> {
    return invoke<DailySummary>("get_today_summary");
  }

  async getYesterdaySummary(): Promise<DailySummary> {
    return invoke<DailySummary>("get_yesterday_summary");
  }

  async getThisWeekSummary(): Promise<WeeklySummary> {
    return invoke<WeeklySummary>("get_this_week_summary");
  }

  async getLastWeekSummary(): Promise<WeeklySummary> {
    return invoke<WeeklySummary>("get_last_week_summary");
  }

  async getThisMonthSummary(): Promise<MonthlySummary> {
    return invoke<MonthlySummary>("get_this_month_summary");
  }

  async getWorkspaceInsight(workspaceId: string): Promise<WorkspaceInsight> {
    return invoke<WorkspaceInsight>("get_workspace_insight", { workspaceId });
  }
}

let analyticsRepositoryInstance: AnalyticsRepository | null = null;

export function getAnalyticsRepository(): AnalyticsRepository {
  if (!analyticsRepositoryInstance) {
    analyticsRepositoryInstance = new AnalyticsRepository();
  }
  return analyticsRepositoryInstance;
}
