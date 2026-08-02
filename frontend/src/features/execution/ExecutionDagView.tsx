// ExecutionDagView - renders the plan's task DAG with live step status,
// dependency edges, and conditional gates.

import { ArrowDown, Ban, Check, CircleDashed, Loader2, X } from "lucide-react";
import { cn } from "@/utils/cn";
import type { ExecutionProgress, ExecutionStep, StepStatus } from "@/types/execution";

const STEP_META: Record<StepStatus, { label: string; className: string }> = {
  completed: {
    label: "Completed",
    className: "border-(--color-success) bg-(--color-success-muted) text-(--color-success)",
  },
  failed: {
    label: "Failed",
    className: "border-(--color-destructive) bg-(--color-destructive-muted) text-(--color-destructive)",
  },
  skipped: {
    label: "Skipped",
    className: "border-(--color-border) text-(--color-muted-foreground)",
  },
  running: {
    label: "Running",
    className: "border-(--color-accent) bg-(--color-accent-muted) text-(--color-accent)",
  },
  pending: {
    label: "Pending",
    className: "border-(--color-border) text-(--color-muted-foreground)",
  },
};

function StepStatusIcon({ status }: { status: StepStatus }) {
  switch (status) {
    case "running":
      return <Loader2 className="h-4 w-4 animate-spin" data-testid={`status-icon-${status}`} />;
    case "completed":
      return <Check className="h-4 w-4" data-testid={`status-icon-${status}`} />;
    case "failed":
      return <X className="h-4 w-4" data-testid={`status-icon-${status}`} />;
    case "skipped":
      return <Ban className="h-4 w-4" data-testid={`status-icon-${status}`} />;
    default:
      return <CircleDashed className="h-4 w-4" data-testid={`status-icon-${status}`} />;
  }
}

export function ExecutionDagView({ progress }: { progress: ExecutionProgress }) {
  const plan = progress.plan;
  const steps: ExecutionStep[] = progress.steps;
  const running = progress.status === "running";

  const completed = steps.filter((s) => s.status === "completed").length;
  const failed = steps.filter((s) => s.status === "failed").length;
  const skipped = steps.filter((s) => s.status === "skipped").length;

  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
      <h2 className="mb-3 font-(family-name:--font-display) text-sm font-semibold text-(--color-foreground)">
        Execution DAG
        <span className="ml-2 text-xs font-normal text-(--color-faint-foreground)">
          {completed} completed · {failed} failed · {skipped} skipped
        </span>
      </h2>

      {steps.length === 0 && (
        <p className="text-sm text-(--color-muted-foreground)">No steps in this execution.</p>
      )}

      <ol className="space-y-2">
        {steps.map((step) => {
          const meta = STEP_META[step.status];
          const taskIndex = plan?.tasks.findIndex((_t, idx) => idx === step.step_number);
          const task = taskIndex !== undefined && taskIndex >= 0 ? plan?.tasks[taskIndex] : undefined;
          const dependencies = task?.dependencies ?? [];
          return (
            <li
              key={step.id}
              className="overflow-hidden rounded-md border border-(--color-border-subtle)"
            >
              <div
                className={cn("flex items-center gap-3 px-3 py-2", meta.className)}
                data-testid={`step-${step.step_number}-${step.status}`}
              >
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-(--color-surface-raised)/80 text-xs font-medium">
                  {step.step_number + 1}
                </span>
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm text-(--color-foreground)">{step.description}</p>
                  <p className="truncate text-xs text-(--color-muted-foreground)">
                    {step.tool_name ?? "no tool"}
                    {step.error ? ` — ${step.error}` : ""}
                  </p>
                </div>
                <span
                  className={cn(
                    "shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium",
                    running && step.status === "running"
                      ? "bg-(--color-accent-muted) text-(--color-accent)"
                      : "bg-(--color-surface-raised)/80 text-(--color-muted-foreground)"
                  )}
                >
                  {meta.label}
                </span>
                <StepStatusIcon status={step.status} />
              </div>

              {dependencies.length > 0 && (
                <div className="flex flex-wrap gap-1 border-t border-(--color-border-subtle) bg-(--color-surface) px-3 py-1.5">
                  {dependencies.map((depId: string) => {
                    const depIndex = plan?.tasks.findIndex((t) => t.id === depId) ?? -1;
                    const dep = steps[depIndex];
                    return (
                      <span
                        key={depId}
                        className="inline-flex items-center gap-1 rounded bg-(--color-surface-hover) px-1.5 py-0.5 text-[10px] text-(--color-muted-foreground)"
                        data-testid="dependency-tag"
                      >
                        <ArrowDown className="h-3 w-3" />
                        deps on step {depIndex + 1}
                        {dep ? ` (${STEP_META[dep.status].label})` : ""}
                      </span>
                    );
                  })}
                </div>
              )}
            </li>
          );
        })}
      </ol>
    </div>
  );
}