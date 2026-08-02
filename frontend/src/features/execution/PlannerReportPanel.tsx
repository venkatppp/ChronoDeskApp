// PlannerReportPanel - renders the planner's final run summary once execution
// is planner-driven (replan count, completed/skipped/replaced tasks).

import { CheckCircle2, RefreshCcw, SkipForward, XCircle } from "lucide-react";
import type { PlannerReport } from "@/types/execution";

export function PlannerReportPanel({ report }: { report: PlannerReport }) {
  const taskLabel = (id: string) =>
    report.plan.tasks.find((t) => t.id === id)?.description ?? `task ${id.slice(0, 8)}`;

  return (
    <div
      className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4"
      data-testid="planner-report"
    >
      <h2 className="mb-3 flex items-center gap-2 font-(family-name:--font-display) text-sm font-semibold text-(--color-foreground)">
        <RefreshCcw className="h-4 w-4 text-(--color-accent)" />
        Planner Report
      </h2>

      {report.error && (
        <p className="mb-3 rounded-md border border-(--color-destructive)/30 bg-(--color-destructive-muted) px-3 py-2 text-sm text-(--color-destructive)">
          {report.error}
        </p>
      )}

      <div className="mb-3 flex items-center gap-2 text-sm text-(--color-muted-foreground)">
        <span className="rounded-full bg-(--color-surface-hover) px-2 py-0.5 text-xs">
          {report.replan_count} replan{report.replan_count === 1 ? "" : "s"}
        </span>
        <span className="text-xs text-(--color-faint-foreground)">{report.plan.tasks.length} plan tasks</span>
      </div>

      <dl className="space-y-2 text-sm">
        <div className="flex items-start gap-2">
          <div className="flex gap-1 text-sm" data-testid="planner-completed">
            <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-(--color-success)" />
            <span className="text-(--color-muted-foreground)">Completed:</span>
          </div>
          <ul className="flex-1 space-y-0.5" data-testid="planner-completed-list">
            {report.completed.length === 0 && (
              <li className="text-xs text-(--color-faint-foreground)">none</li>
            )}
            {report.completed.map((id) => (
              <li key={id} className="text-xs text-(--color-foreground)">
                {taskLabel(id)}
              </li>
            ))}
          </ul>
        </div>

        <div className="flex items-start gap-2">
          <SkipForward className="mt-0.5 h-4 w-4 shrink-0 text-(--color-muted-foreground)" />
          <div className="flex-1">
            <p className="text-sm text-(--color-muted-foreground)">Skipped:</p>
            <ul className="space-y-0.5">
              {report.skipped.length === 0 && (
                <li className="text-xs text-(--color-faint-foreground)">none</li>
              )}
              {report.skipped.map((id) => (
                <li key={id} className="text-xs text-(--color-foreground)">
                  {taskLabel(id)}
                </li>
              ))}
            </ul>
          </div>
        </div>

        <div className="flex items-start gap-2">
          <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-(--color-destructive)" />
          <div className="flex-1">
            <p className="text-sm text-(--color-muted-foreground)">Replaced by replanning:</p>
            <ul className="space-y-0.5">
              {report.replaced.length === 0 && (
                <li className="text-xs text-(--color-faint-foreground)">none</li>
              )}
              {report.replaced.map((id) => (
                <li key={id} className="text-xs text-(--color-foreground)">
                  {taskLabel(id)}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </dl>
    </div>
  );
}