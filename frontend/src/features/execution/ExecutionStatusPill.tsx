// ExecutionStatusPill - visual status indicator for an execution.

import { cn } from "@/utils/cn";
import type { ExecutionStatus } from "@/types/execution";

const STATUS_STYLE_CLASSES: Record<ExecutionStatus, string> = {
  pending: "border-(--color-border) text-(--color-muted-foreground)",
  running: "border-(--color-accent) bg-(--color-accent-muted) text-(--color-accent)",
  paused: "border-amber-500/60 bg-amber-500/10 text-amber-500",
  completed: "border-(--color-success) bg-(--color-success-muted) text-(--color-success)",
  failed:
    "border-(--color-destructive) bg-(--color-destructive-muted) text-(--color-destructive)",
  cancelled: "border-(--color-border) text-(--color-muted-foreground)",
};

export function ExecutionStatusPill({ status }: { status: ExecutionStatus }) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium capitalize",
        STATUS_STYLE_CLASSES[status]
      )}
      data-status={status}
    >
      {status}
    </span>
  );
}