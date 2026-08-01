// ExecutionPlanCard - Autonomous plan requiring approval

import { useState } from "react";
import { CheckCircle2, Clock, FileText, Target, ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/utils/cn";
import type { ExecutionPlan, PermissionLevel } from "@/types/proactive";

interface ExecutionPlanCardProps {
  plan: ExecutionPlan;
  onApprove: (planId: string, permission: PermissionLevel) => void;
  onReject: (planId: string) => void;
}

export function ExecutionPlanCard({ plan, onApprove, onReject }: ExecutionPlanCardProps) {
  const [showDetails, setShowDetails] = useState(false);
  const [selectedPermission, setSelectedPermission] = useState<PermissionLevel>("ask_each_time");

  const completedTasks = plan.tasks.filter((t) => t.completed).length;
  const progress = (completedTasks / plan.tasks.length) * 100;

  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <Target className="h-5 w-5 text-(--color-accent)" />
            <h3 className="font-semibold text-(--color-foreground)">{plan.goal}</h3>
          </div>

          <div className="mt-2 flex items-center gap-4 text-sm text-(--color-muted-foreground)">
            <div className="flex items-center gap-1.5">
              <Clock className="h-4 w-4" />
              <span>{plan.estimated_duration_minutes} min</span>
            </div>
            <div className="flex items-center gap-1.5">
              <FileText className="h-4 w-4" />
              <span>{plan.tasks.length} tasks</span>
            </div>
            <div className="flex items-center gap-1.5">
              <CheckCircle2 className="h-4 w-4" />
              <span>{Math.round(plan.confidence * 100)}% confidence</span>
            </div>
          </div>

          {/* Progress Bar */}
          {plan.status === "executing" && (
            <div className="mt-3">
              <div className="h-2 w-full overflow-hidden rounded-full bg-(--color-surface-hover)">
                <div
                  className="h-full bg-(--color-accent) transition-all"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="mt-1 text-xs text-(--color-muted-foreground)">
                {completedTasks} of {plan.tasks.length} tasks completed
              </p>
            </div>
          )}

          {/* Reasoning */}
          <p className="mt-2 text-sm text-(--color-muted-foreground)">{plan.reasoning}</p>

          {/* Tasks Toggle */}
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="mt-3 flex items-center gap-1.5 text-sm text-(--color-accent) hover:text-(--color-accent)/80"
          >
            {showDetails ? (
              <ChevronUp className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
            <span>{showDetails ? "Hide" : "Show"} tasks</span>
          </button>

          {showDetails && (
            <div className="mt-3 space-y-2">
              {plan.tasks.map((task, idx) => (
                <div
                  key={task.id}
                  className="flex items-start gap-2 rounded-md border border-(--color-border-subtle) bg-(--color-surface) p-2"
                >
                  <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-(--color-accent-muted) text-xs font-medium text-(--color-accent)">
                    {idx + 1}
                  </span>
                  <div className="flex-1">
                    <p className="text-sm text-(--color-foreground)">{task.description}</p>
                    <p className="mt-1 text-xs text-(--color-muted-foreground)">
                      {task.estimated_minutes} min
                    </p>
                  </div>
                  {task.completed && <CheckCircle2 className="h-5 w-5 text-(--color-success)" />}
                </div>
              ))}
            </div>
          )}

          {/* Approval UI */}
          {plan.status === "pending" && (
            <div className="mt-4 border-t border-(--color-border-subtle) pt-4">
              <p className="mb-2 text-sm font-medium text-(--color-foreground)">
                Automation Permission:
              </p>
              <div className="mb-3 flex gap-2">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="permission"
                    value="ask_each_time"
                    checked={selectedPermission === "ask_each_time"}
                    onChange={(e) => setSelectedPermission(e.target.value as PermissionLevel)}
                    className="text-(--color-accent)"
                  />
                  <span className="text-(--color-foreground)">Ask each time</span>
                </label>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="permission"
                    value="always_allow"
                    checked={selectedPermission === "always_allow"}
                    onChange={(e) => setSelectedPermission(e.target.value as PermissionLevel)}
                    className="text-(--color-accent)"
                  />
                  <span className="text-(--color-foreground)">Always allow</span>
                </label>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="radio"
                    name="permission"
                    value="always_reject"
                    checked={selectedPermission === "always_reject"}
                    onChange={(e) => setSelectedPermission(e.target.value as PermissionLevel)}
                    className="text-(--color-accent)"
                  />
                  <span className="text-(--color-foreground)">Never allow</span>
                </label>
              </div>

              <div className="flex gap-2">
                <button
                  onClick={() => onApprove(plan.id, selectedPermission)}
                  className={cn(
                    "flex-1 rounded-lg bg-(--color-accent) px-4 py-2 text-sm font-medium text-(--color-accent-foreground)",
                    "transition-colors hover:bg-(--color-accent)/90"
                  )}
                >
                  Approve & Execute
                </button>
                <button
                  onClick={() => onReject(plan.id)}
                  className="flex-1 rounded-lg border border-(--color-border) bg-(--color-surface) px-4 py-2 text-sm text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
                >
                  Reject
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
