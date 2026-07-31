import { useMemo, useState } from "react";
import { AlertTriangle, ArrowRight, Archive, PlayCircle, ChevronUp, Clock, Zap, type LucideIcon } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { Recommendation } from "@/types/workspace";

const PRIORITY_LABELS: Record<string, { label: string; color: string; border: string; bg: string; icon: LucideIcon }> = {
  attention: {
    label: "High Priority",
    color: "text-(--color-warning)",
    border: "border-(--color-warning)/30",
    bg: "bg-(--color-warning)/5",
    icon: AlertTriangle,
  },
  resume: {
    label: "Medium Priority",
    color: "text-(--color-accent)",
    border: "border-(--color-accent)/20",
    bg: "bg-(--color-accent)/5",
    icon: PlayCircle,
  },
  archive: {
    label: "Low Priority",
    color: "text-(--color-muted-foreground)",
    border: "border-(--color-border)",
    bg: "bg-(--color-surface)",
    icon: Archive,
  },
};

const EFFORT_ICONS: Record<string, LucideIcon> = {
  quick: Zap,
  moderate: Clock,
  significant: Clock,
};

const EFFORT_LABELS: Record<string, string> = {
  quick: "Quick",
  moderate: "Moderate",
  significant: "Significant",
};

const CATEGORY_COLORS: Record<string, string> = {
  maintenance: "text-(--color-warning)",
  productivity: "text-(--color-accent)",
  health: "text-(--color-danger)",
  exploration: "text-(--color-success)",
};

interface RecommendationsPanelProps {
  recommendations: Recommendation[];
  isLoading: boolean;
}

export function RecommendationsPanel({ recommendations, isLoading }: RecommendationsPanelProps) {
  const navigate = useNavigate();
  const workspaceRepo = getWorkspaceRepository();
  const [dismissed, setDismissed] = useState<Set<string>>(new Set());
  const [sortBy, setSortBy] = useState<"priority" | "effort" | "impact">("priority");

  const visible = useMemo(() => {
    return recommendations.filter((r) => !dismissed.has(r.id));
  }, [recommendations, dismissed]);

  const sorted = useMemo(() => {
    const items = [...visible];
    switch (sortBy) {
      case "effort":
        return items.sort((a, b) => {
          const order = { quick: 1, moderate: 2, significant: 3 };
          return (order[a.estimatedEffort] ?? 2) - (order[b.estimatedEffort] ?? 2);
        });
      case "impact": {
        const order = { low: 1, medium: 2, high: 3 };
        return items.sort((a, b) => (order[b.expectedImpact] ?? 0) - (order[a.expectedImpact] ?? 0));
      }
      default:
        return items.sort((a, b) => a.priority - b.priority);
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
          className="rounded border border-(--color-border-subtle) bg-(--color-surface) px-1.5 py-0.5 text-[10px] text-(--color-muted-foreground) focus:outline-none"
        >
          <option value="priority">Priority</option>
          <option value="effort">Effort</option>
          <option value="impact">Impact</option>
        </select>
      </div>
      <div className="flex flex-col gap-2">
        {sorted.map((rec) => {
          const priority = PRIORITY_LABELS[rec.kind] ?? PRIORITY_LABELS.archive;
          const Icon = priority.icon;
          const EffortIcon = EFFORT_ICONS[rec.estimatedEffort] ?? Clock;

          return (
            <div
              key={rec.id}
              className={`rounded-[var(--radius-control)] border ${priority.border} ${priority.bg} p-3 transition-colors hover:bg-(--color-surface-hover)`}
            >
              <div className="flex items-start justify-between gap-2">
                <div className="flex items-start gap-2.5">
                  <Icon className={`mt-0.5 h-4 w-4 shrink-0 ${priority.color}`} strokeWidth={2} />
                  <div className="min-w-0">
                    <p className={`text-xs font-semibold uppercase tracking-wide ${priority.color}`}>
                      {priority.label}
                    </p>
                    <p className="mt-0.5 text-sm font-medium text-(--color-foreground)">{rec.message}</p>
                    <p className="mt-0.5 text-xs text-(--color-muted-foreground)">{rec.reason}</p>
                    <div className="mt-1.5 flex items-center gap-2">
                      <span className={`inline-flex items-center gap-0.5 text-[9px] font-medium uppercase tracking-wider ${CATEGORY_COLORS[rec.category] ?? "text-(--color-faint-foreground)"}`}>
                        {rec.category}
                      </span>
                      <span className="inline-flex items-center gap-0.5 text-[9px] text-(--color-faint-foreground)">
                        <EffortIcon className="h-2.5 w-2.5" strokeWidth={2} />
                        {EFFORT_LABELS[rec.estimatedEffort]}
                      </span>
                      <span className="inline-flex items-center gap-0.5 text-[9px] text-(--color-faint-foreground)">
                        Impact: {rec.expectedImpact}
                      </span>
                    </div>
                  </div>
                </div>
                <div className="flex shrink-0 gap-1">
                  {(rec.kind === "resume" || rec.kind === "attention") && rec.workspaceId && (
                    <button
                      onClick={() => {
                        workspaceRepo.switchWorkspace(rec.workspaceId!).then(() => {
                          localStorage.setItem("activeWorkspaceId", rec.workspaceId!);
                          navigate("/timeline");
                        }).catch(() => {});
                      }}
                      className="flex items-center gap-1 rounded bg-(--color-accent)/10 px-2.5 py-1 text-xs font-medium text-(--color-accent) transition-colors hover:bg-(--color-accent)/20"
                    >
                      Review
                      <ArrowRight className="h-3 w-3" strokeWidth={2} />
                    </button>
                  )}
                  {rec.kind === "archive" && rec.workspaceId && (
                    <button
                      onClick={() => {
                        workspaceRepo.updateWorkspace(rec.workspaceId!, { status: "archived" }).catch(() => {});
                      }}
                      className="flex items-center gap-1 rounded bg-(--color-surface-hover) px-2.5 py-1 text-xs font-medium text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
                    >
                      Dismiss
                    </button>
                  )}
                  <button
                    onClick={() => setDismissed((prev) => new Set(prev).add(rec.id))}
                    className="rounded bg-(--color-surface-hover) px-1.5 py-1 text-[10px] font-medium text-(--color-faint-foreground) transition-colors hover:text-(--color-muted-foreground)"
                    title="Dismiss"
                  >
                    &times;
                  </button>
                </div>
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
