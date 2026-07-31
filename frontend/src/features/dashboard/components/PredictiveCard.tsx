import { Brain, TrendingUp, FileText, Zap, Clock, Activity, Target } from "lucide-react";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import type { PredictionsSummary } from "@/types/predictive";

interface PredictiveCardProps {
  predictions: PredictionsSummary | null;
  isLoading: boolean;
}

const WORKFLOW_LABELS: Record<string, { label: string; color: string }> = {
  coding: { label: "Coding", color: "text-(--color-accent)" },
  debugging: { label: "Debugging", color: "text-(--color-danger)" },
  documentation: { label: "Documentation", color: "text-(--color-success)" },
  research: { label: "Research", color: "text-(--color-warning)" },
  meeting: { label: "Meeting", color: "text-(--color-info)" },
  custom: { label: "Custom", color: "text-(--color-muted-foreground)" },
};

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m`;
  return "< 1m";
}

export function PredictiveCard({ predictions, isLoading }: PredictiveCardProps) {
  if (isLoading) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
            <Brain className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Predictive Intelligence
            </p>
            <div className="mt-2 space-y-2">
              <div className="h-4 w-full animate-pulse rounded bg-(--color-surface)" />
              <div className="h-4 w-3/4 animate-pulse rounded bg-(--color-surface)" />
              <div className="h-4 w-2/3 animate-pulse rounded bg-(--color-surface)" />
            </div>
          </div>
        </div>
      </Card>
    );
  }

  if (!predictions) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-surface-hover)">
            <Brain className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Predictive Intelligence
            </p>
            <p className="mt-1 text-sm text-(--color-muted-foreground)">Learning from your workflow...</p>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-4">
      <div className="flex flex-col gap-3">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
            <Brain className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Predictive Intelligence
            </p>
          </div>
        </div>

        <div className="space-y-3">
          {predictions.currentWorkflow && (
            <div className="flex items-start gap-2">
              <Activity className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-accent)" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  Current Workflow
                </p>
                <div className="mt-1 flex items-center gap-2">
                  <Badge
                    variant="accent"
                    className={`text-xs ${WORKFLOW_LABELS[predictions.currentWorkflow.workflowType]?.color || ""}`}
                  >
                    {WORKFLOW_LABELS[predictions.currentWorkflow.workflowType]?.label || "Unknown"}
                  </Badge>
                  <span className="text-xs text-(--color-faint-foreground)">
                    {Math.round(predictions.currentWorkflow.confidence * 100)}% confidence
                  </span>
                </div>
                {predictions.currentWorkflow.activeFiles.length > 0 && (
                  <p className="mt-1 text-xs text-(--color-muted-foreground)">
                    {predictions.currentWorkflow.activeFiles.length} active {predictions.currentWorkflow.activeFiles.length === 1 ? "file" : "files"}
                  </p>
                )}
              </div>
            </div>
          )}

          {predictions.nextWorkspace && (
            <div className="flex items-start gap-2">
              <Target className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-success)" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  Next Workspace
                </p>
                <div className="mt-1 flex items-center gap-2">
                  <p className="truncate text-sm font-medium text-(--color-foreground)">
                    {predictions.nextWorkspace.workspaceName}
                  </p>
                  <TrendingUp className="h-3 w-3 shrink-0 text-(--color-success)" strokeWidth={2} />
                  <span className="text-xs text-(--color-faint-foreground)">
                    {Math.round(predictions.nextWorkspace.confidence * 100)}%
                  </span>
                </div>
                {predictions.nextWorkspace.reason && (
                  <p className="mt-1 text-xs text-(--color-muted-foreground)">{predictions.nextWorkspace.reason}</p>
                )}
              </div>
            </div>
          )}

          {predictions.nextFiles.length > 0 && (
            <div className="flex items-start gap-2">
              <FileText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-warning)" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  Likely Next Files
                </p>
                <div className="mt-1 space-y-1">
                  {predictions.nextFiles.slice(0, 3).map((file, idx) => (
                    <div key={idx} className="flex items-center gap-2">
                      <span className="min-w-0 flex-1 truncate font-(family-name:--font-mono) text-xs text-(--color-foreground)">
                        {file.filePath.split('/').pop() || file.filePath}
                      </span>
                      <span className="text-[10px] text-(--color-faint-foreground)">
                        {Math.round(file.confidence * 100)}%
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {predictions.nextActions.length > 0 && (
            <div className="flex items-start gap-2">
              <Zap className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-info)" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  Suggested Actions
                </p>
                <div className="mt-1 space-y-1">
                  {predictions.nextActions.slice(0, 3).map((action, idx) => (
                    <div key={idx} className="flex items-start gap-2">
                      <span className="min-w-0 flex-1 text-xs text-(--color-foreground)">{action.description}</span>
                      <span className="shrink-0 text-[10px] text-(--color-faint-foreground)">
                        {Math.round(action.confidence * 100)}%
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {predictions.sessionContinuation && (
            <div className="flex items-start gap-2">
              <Clock className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-muted-foreground)" strokeWidth={1.75} />
              <div className="min-w-0 flex-1">
                <p className="text-[11px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  Session Outlook
                </p>
                <div className="mt-1 flex items-center gap-2">
                  <Badge variant={predictions.sessionContinuation.willContinue ? "success" : "neutral"}>
                    {predictions.sessionContinuation.willContinue ? "Continuing" : "Ending soon"}
                  </Badge>
                  {predictions.sessionContinuation.estimatedDurationSeconds > 0 && (
                    <span className="text-xs text-(--color-muted-foreground)">
                      ~{formatDuration(predictions.sessionContinuation.estimatedDurationSeconds)}
                    </span>
                  )}
                </div>
                {predictions.sessionContinuation.reason && (
                  <p className="mt-1 text-xs text-(--color-muted-foreground)">{predictions.sessionContinuation.reason}</p>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}
