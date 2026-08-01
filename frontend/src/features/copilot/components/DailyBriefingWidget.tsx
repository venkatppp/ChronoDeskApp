// DailyBriefingWidget - Today's priorities and workspace insights

import { Calendar, TrendingUp, CheckCircle2, Lightbulb } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import type { DailyBriefing } from "@/types/copilot";

interface DailyBriefingWidgetProps {
  briefing: DailyBriefing | null;
  loading?: boolean;
  onRefresh?: () => void;
}

export function DailyBriefingWidget({ briefing, loading, onRefresh }: DailyBriefingWidgetProps) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Calendar className="h-5 w-5 text-(--color-accent)" />
            Daily Briefing
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            <div className="h-4 w-3/4 rounded bg-(--color-surface-hover)" />
            <div className="h-4 w-full rounded bg-(--color-surface-hover)" />
            <div className="h-4 w-5/6 rounded bg-(--color-surface-hover)" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!briefing) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Calendar className="h-5 w-5 text-(--color-accent)" />
            Daily Briefing
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-(--color-muted-foreground)">
            No briefing available. Start working to generate insights.
          </p>
        </CardContent>
      </Card>
    );
  }

  const formattedDate = new Date(briefing.date).toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Calendar className="h-5 w-5 text-(--color-accent)" />
            Daily Briefing
          </CardTitle>
          {onRefresh && (
            <button
              onClick={onRefresh}
              className="text-xs text-(--color-muted-foreground) hover:text-(--color-accent)"
            >
              Refresh
            </button>
          )}
        </div>
        <p className="text-xs text-(--color-muted-foreground)">{formattedDate}</p>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Summary */}
        <div>
          <p className="text-sm text-(--color-foreground)">{briefing.summary}</p>
        </div>

        {/* Workspace Stats */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
            <div className="text-2xl font-bold text-(--color-accent)">
              {briefing.workspace_stats.active_workspaces}
            </div>
            <div className="text-xs text-(--color-muted-foreground)">Active Workspaces</div>
          </div>
          <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
            <div className="text-2xl font-bold text-(--color-accent)">
              {briefing.workspace_stats.files_modified}
            </div>
            <div className="text-xs text-(--color-muted-foreground)">Files Modified</div>
          </div>
          <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
            <div className="text-2xl font-bold text-(--color-accent)">
              {Math.round(briefing.workspace_stats.time_tracked / 3600)}h
            </div>
            <div className="text-xs text-(--color-muted-foreground)">Time Tracked</div>
          </div>
          <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
            <div className="text-2xl font-bold text-(--color-accent)">
              {briefing.workspace_stats.sessions_completed}
            </div>
            <div className="text-xs text-(--color-muted-foreground)">Sessions</div>
          </div>
        </div>

        {/* Highlights */}
        {briefing.highlights.length > 0 && (
          <div>
            <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-(--color-muted-foreground)">
              <TrendingUp className="h-3.5 w-3.5" />
              Highlights
            </div>
            <ul className="space-y-1.5">
              {briefing.highlights.map((highlight, idx) => (
                <li key={idx} className="flex items-start gap-2 text-sm text-(--color-foreground)">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-accent)" />
                  <span>{highlight}</span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Pending Tasks */}
        {briefing.pending_tasks.length > 0 && (
          <div>
            <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-(--color-muted-foreground)">
              <CheckCircle2 className="h-3.5 w-3.5" />
              Pending Tasks
            </div>
            <ul className="space-y-1.5">
              {briefing.pending_tasks.map((task, idx) => (
                <li key={idx} className="flex items-start gap-2 text-sm text-(--color-foreground)">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-warning)" />
                  <span>{task}</span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* Recommendations */}
        {briefing.recommendations.length > 0 && (
          <div>
            <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-(--color-muted-foreground)">
              <Lightbulb className="h-3.5 w-3.5" />
              Recommendations
            </div>
            <ul className="space-y-1.5">
              {briefing.recommendations.map((rec, idx) => (
                <li key={idx} className="flex items-start gap-2 text-sm text-(--color-foreground)">
                  <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-success)" />
                  <span>{rec}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
