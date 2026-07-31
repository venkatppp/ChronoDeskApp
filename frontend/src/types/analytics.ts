// Analytics types matching Rust models

export interface TrendIndicator {
  current: number;
  previous: number;
  changePercent: number;
  description: string;
}

export interface LanguageUsage {
  language: string;
  fileCount: number;
  editCount: number;
  percentage: number;
}

export interface WorkspaceDaySummary {
  workspaceId: string;
  workspaceName: string;
  durationSeconds: number;
  sessionCount: number;
  editCount: number;
}

export interface DailySummary {
  date: string;
  totalDurationSeconds: number;
  sessionCount: number;
  workspaceCount: number;
  fileCount: number;
  editCount: number;
  commitCount: number;
  languages: LanguageUsage[];
  mostActiveWorkspace?: WorkspaceDaySummary;
  longestSessionDuration?: number;
  averageSessionDuration?: number;
}

export interface WeeklySummary {
  weekStart: string;
  weekEnd: string;
  totalDurationSeconds: number;
  sessionCount: number;
  workspaceCount: number;
  fileCount: number;
  editCount: number;
  commitCount: number;
  languages: LanguageUsage[];
  mostProductiveDay?: string;
  averageDailyDuration: number;
  focusTrend?: TrendIndicator;
}

export interface MonthlySummary {
  monthStart: string;
  monthEnd: string;
  totalDurationSeconds: number;
  sessionCount: number;
  workspaceCount: number;
  fileCount: number;
  editCount: number;
  commitCount: number;
  languages: LanguageUsage[];
  activeWorkspaces: string[];
  weeklyAverageDuration: number;
}

export interface ActivitySummary {
  timeRange: string;
  durationSeconds: number;
  sessionCount: number;
  workspaceCount: number;
  fileCount: number;
  editCount: number;
  commitCount: number;
  primaryLanguage?: string;
}

export interface DailyBriefing {
  greeting: string;
  summary: ActivitySummary;
  mostActiveWorkspace?: WorkspaceDaySummary;
  longestFocusSession?: number;
  primaryLanguage?: string;
  insights: string[];
  suggestions: string[];
}

export interface WorkspaceInsight {
  workspaceId: string;
  workspaceName: string;
  todayEdits: number;
  weeklyEdits: number;
  totalSessions: number;
  averageSessionDuration: number;
  mostEditedFiles: string[];
  primaryLanguage?: string;
  lastActive: string;
  activityTrend?: TrendIndicator;
  healthTrend?: TrendIndicator;
}
