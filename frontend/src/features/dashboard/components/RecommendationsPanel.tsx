import { useMemo, useState } from "react";
import { AlertTriangle, ArrowRight, Archive, PlayCircle, ChevronUp, Zap, RotateCcw, Loader2, type LucideIcon } from "lucide-react";
import { useNavigate } from "react-router-dom";
import type { Recommendation } from "@/types/intelligence";
import type { ActionType } from "@/types/actions";
import { actionRepository } from "@/services/actionRepository";

const PRIORITY_LABELS: Record<string, { label: string; color: string; border: string; bg: string; icon: LucideIcon }> = {
  critical: {
    label: "Critical",
    color: "text-(--color-danger)",
    border: "border-(--color-danger)/30",
    bg: "bg-(--color-danger)/5",
    icon: AlertTriangle,
  },
  high: {
    label: "High Priority",
    color: "text-(--color-warning)",
    border: "border-(--color-warning)/30",
    bg: "bg-(--color-warning)/5",
    icon: AlertTriangle,
  },
  medium: {
    label: "Medium Priority",
    color: "text-(--color-violet)",
    border: "border-(--color-violet)/25",
    bg: "bg-(--color-violet)/10",
    icon: PlayCircle,
  },
  low: {
    label: "Low Priority",
    color: "text-(--color-muted-foreground)",
    border: "border-(--color-border)",
    bg: "bg-(--color-surface)",
    icon: Archive,
  },
};

const CATEGORY_COLORS: Record<string, string> = {
  organization: "text-(--color-warning)",
  productivity: "text-(--color-violet)",
  context: "text-(--color-success)",
  files: "text-(--color-warning)",
  search: "text-(--color-violet)",
  health: "text-(--color-danger)",
};

interface RecommendationsPanelProps {
  recommendations: Recommendation[];
  isLoading: boolean;
  onActionSuccess?: () => void;
}

export function RecommendationsPanel({ recommendations, isLoading, onActionSuccess }: RecommendationsPanelProps) {
  const navigate = useNavigate();
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [sortBy, setSortBy] = useState<"priority" | "effort" | "impact">("priority");
  const [executing, setExecuting] = useState<string | null>(null);
  const [actionResults, setActionResults] = useState<Map<string, { success: boolean; message: string; actionId?: number }>>(new Map());

  const handleExecuteAction = async (rec: Recommendation, actionType: ActionType) => {
    setExecuting(rec.id);
    try {
      const result = await actionRepository.executeAction({
        actionType,
        workspaceId: rec.workspaceId,
        recommendationId: rec.id,
        metadata: rec.metadata,
      });

      setActionResults((prev) => new Map(prev).set(rec.id, {
        success: result.success,
        message: result.message,
        actionId: result.actionId,
      }));

      if (result.success && onActionSuccess) {
        // Trigger dashboard refresh
        setTimeout(() => onActionSuccess(), 500);
      }
    } catch (error) {
      setActionResults((prev) => new Map(prev).set(rec.id, {
        success: false,
        message: error instanceof Error ? error.message : "Action failed",
      }));
    } finally {
      setExecuting(null);
    }
  };

  const handleUndo = async (rec: Recommendation) => {
    const result = actionResults.get(rec.id);
    if (!result?.actionId) return;

    setExecuting(rec.id);
    try {
      const undoResult = await actionRepository.undoAction(result.actionId);
      
      setActionResults((prev) => {
        const next = new Map(prev);
        next.delete(rec.id);
        return next;
      });

      if (undoResult.success && onActionSuccess) {
        setTimeout(() => onActionSuccess(), 500);
      }
    } catch (error) {
      console.error("Undo failed:", error);
    } finally {
      setExecuting(null);
    }
  };

  const getActionButton = (rec: Recommendation) => {
    const result = actionResults.get(rec.id);
    const isExecuting = executing === rec.id;

    // If action was executed, show result or undo button
    if (result) {
      if (result.success && result.actionId) {
        return (
          <button
            onClick={() => handleUndo(rec)}
            disabled={isExecuting}
            className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-warning)/30 bg-(--color-warning)/10 px-2.5 py-1 text-xs font-medium text-(--color-warning) transition-colors hover:bg-(--color-warning)/20 disabled:opacity-50"
            title="Undo action"
          >
            {isExecuting ? (
              <Loader2 className="h-3 w-3 animate-spin" strokeWidth={2} />
            ) : (
              <RotateCcw className="h-3 w-3" strokeWidth={2} />
            )}
            Undo
          </button>
        );
      }
      return null;
    }

    // Map recommendation metadata to action buttons
    const category = rec.category;
    const title = rec.title.toLowerCase();

    if (title.includes("archive") || category === "organization") {
      return (
        <button
          onClick={() => handleExecuteAction(rec, "archive_workspace")}
          disabled={isExecuting}
          className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-accent)/25 bg-(--color-accent)/10 px-2.5 py-1 text-xs font-medium text-(--color-accent) transition-colors hover:bg-(--color-accent)/20 disabled:opacity-50"
        >
          {isExecuting ? <Loader2 className="h-3 w-3 animate-spin" strokeWidth={2} /> : <Archive className="h-3 w-3" strokeWidth={2} />}
          Archive
        </button>
      );
    }

    if (title.includes("resume") || title.includes("session")) {
      return (
        <button
          onClick={() => handleExecuteAction(rec, "resume_previous_session")}
          disabled={isExecuting}
          className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-success)/30 bg-(--color-success)/10 px-2.5 py-1 text-xs font-medium text-(--color-success) transition-colors hover:bg-(--color-success)/20 disabled:opacity-50"
        >
          {isExecuting ? <Loader2 className="h-3 w-3 animate-spin" strokeWidth={2} /> : <PlayCircle className="h-3 w-3" strokeWidth={2} />}
          Resume
        </button>
      );
    }

    if (title.includes("clean") || title.includes("duplicate")) {
      return (
        <button
          onClick={() => handleExecuteAction(rec, "clean_duplicate_files")}
          disabled={isExecuting}
          className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-accent)/25 bg-(--color-accent)/10 px-2.5 py-1 text-xs font-medium text-(--color-accent) transition-colors hover:bg-(--color-accent)/20 disabled:opacity-50"
        >
          {isExecuting ? <Loader2 className="h-3 w-3 animate-spin" strokeWidth={2} /> : null}
          Clean
        </button>
      );
    }

    return null;
  };

  const visible = useMemo(() => {
    return recommendations.filter((r) => !dismissed.has(r.id));
  }, [recommendations, dismissed]);

  const sorted = useMemo(() => {
    const items = [...visible];
    switch (sortBy) {
      case "effort":
        return items.sort((a, b) => a.effort - b.effort);
      case "impact":
        return items.sort((a, b) => b.impact - a.impact);
      default:
        return items.sort((a, b) => {
          const order = { critical: 0, high: 1, medium: 2, low: 3 };
          return (order[a.priority] ?? 2) - (order[b.priority] ?? 2);
        });
    }
  }, [visible, sortBy]);

  if (isLoading) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Priority Queue</p>
        <div className="flex flex-col gap-2">
          {[0, 1].map((i) => (
            <div key={i} className="h-28 animate-pulse rounded-[var(--radius-control)] bg-(--color-surface)" />
          ))}
        </div>
      </div>
    );
  }

  if (recommendations.length === 0) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Priority Queue</p>
        <div className="flex items-center justify-center rounded-[var(--radius-control)] border border-dashed border-(--color-border-subtle) px-4 py-8 text-center">
          <div>
            <ChevronUp className="mx-auto mb-2 h-5 w-5 text-(--color-success)" strokeWidth={2} />
            <p className="text-sm text-(--color-muted-foreground)">All clear — no issues need attention.</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
          Priority Queue
          {dismissed.size > 0 && (
            <span className="ml-1 text-(--color-muted-foreground)">({visible.length})</span>
          )}
        </p>
        <select
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
          className="glass-control rounded-[var(--radius-control)] px-1.5 py-0.5 text-[10px] text-(--color-muted-foreground) focus:outline-none"
        >
          <option value="priority">Priority</option>
          <option value="effort">Effort</option>
          <option value="impact">Impact</option>
        </select>
      </div>
      <div className="flex flex-col gap-2">
        {sorted.map((rec) => {
          const priority = PRIORITY_LABELS[rec.priority] ?? PRIORITY_LABELS.medium;
          const Icon = priority.icon;

          return (
            <div
              key={rec.id}
              className={`glass-panel flex flex-col gap-3 rounded-[var(--radius-card)] border ${priority.border} p-4 transition-colors hover:border-(--color-accent)/30`}
            >
              <div className="flex flex-wrap items-start justify-between gap-2.5">
                <div className="flex min-w-0 flex-wrap items-center gap-2">
                  <span
                    className={`inline-flex shrink-0 items-center gap-1.5 rounded-full border ${priority.border} ${priority.bg} px-2.5 py-1 text-[11px] font-semibold ${priority.color}`}
                  >
                    <Icon className="h-3 w-3" strokeWidth={2} />
                    {priority.label}
                  </span>
                  <h3 className="min-w-0 truncate text-sm font-medium text-(--color-foreground)">{rec.title}</h3>
                </div>
                <div className="flex shrink-0 flex-wrap items-center gap-1.5">
                  {getActionButton(rec)}
                  {rec.action.type === "open_view" && (
                    <button
                      onClick={() => {
                        const action = rec.action as { type: "open_view"; view: string };
                        navigate(`/${action.view}`);
                      }}
                      className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) px-2.5 py-1 text-xs font-medium text-(--color-accent) transition-colors hover:border-(--color-accent)/40 hover:bg-(--color-accent)/10"
                    >
                      View
                      <ArrowRight className="h-3 w-3" strokeWidth={2} />
                    </button>
                  )}
                  <button
                    onClick={() => setDismissed((prev) => new Set(prev).add(rec.id))}
                    className="flex h-7 w-7 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-hover) text-sm text-(--color-faint-foreground) transition-colors hover:text-(--color-muted-foreground)"
                    title="Dismiss"
                    aria-label="Dismiss recommendation"
                  >
                    &times;
                  </button>
                </div>
              </div>
              <p className="text-xs leading-relaxed text-(--color-muted-foreground)">{rec.description}</p>
              {actionResults.get(rec.id) && (
                <p className={`text-xs font-medium ${actionResults.get(rec.id)?.success ? "text-(--color-success)" : "text-(--color-danger)"}`}>
                  {actionResults.get(rec.id)?.message}
                </p>
              )}
              <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-(--color-border-subtle) pt-3">
                <span className={`inline-flex items-center gap-1 text-[10px] font-medium uppercase tracking-wider ${CATEGORY_COLORS[rec.category] ?? "text-(--color-faint-foreground)"}`}>
                  {rec.category}
                </span>
                <span className="inline-flex items-center gap-1 text-[10px] text-(--color-faint-foreground)">
                  <Zap className="h-2.5 w-2.5" strokeWidth={2} />
                  Effort: {Math.round(rec.effort * 100)}%
                </span>
                <span className="inline-flex items-center gap-1 text-[10px] text-(--color-faint-foreground)">
                  Impact: {Math.round(rec.impact * 100)}%
                </span>
                <span className="inline-flex items-center gap-1 text-[10px] text-(--color-faint-foreground)">
                  Confidence: {Math.round(rec.confidence * 100)}%
                </span>
              </div>
            </div>
          );
        })}
      </div>
      {dismissed.size > 0 && (
        <button
          onClick={() => setDismissed(new Set())}
          className="text-left text-[10px] text-(--color-faint-foreground) hover:text-(--color-accent)"
        >
          Show {dismissed.size} dismissed
        </button>
      )}
    </div>
  );
}
