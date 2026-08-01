// ToolExecutionCard - Visualizes tool execution progress and results

import { CheckCircle2, XCircle, Loader2, Clock } from "lucide-react";
import { cn } from "@/utils/cn";
import type { ToolCall, ToolExecutionStatus } from "@/types/copilot";
import { useState } from "react";

interface ToolExecutionCardProps {
  toolCall: ToolCall;
}

const STATUS_CONFIG: Record<
  ToolExecutionStatus,
  { icon: typeof CheckCircle2; color: string; label: string }
> = {
  pending: {
    icon: Clock,
    color: "text-(--color-muted-foreground)",
    label: "Pending",
  },
  success: {
    icon: CheckCircle2,
    color: "text-(--color-success)",
    label: "Success",
  },
  failed: {
    icon: XCircle,
    color: "text-(--color-danger)",
    label: "Failed",
  },
  cancelled: {
    icon: XCircle,
    color: "text-(--color-muted-foreground)",
    label: "Cancelled",
  },
};

export function ToolExecutionCard({ toolCall }: ToolExecutionCardProps) {
  const [expanded, setExpanded] = useState(false);
  const config = STATUS_CONFIG[toolCall.status];
  const Icon = config.icon;

  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
      <div
        className="flex cursor-pointer items-start justify-between gap-3"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-start gap-2">
          <Icon className={cn("mt-0.5 h-4 w-4 shrink-0", config.color)} />
          <div className="flex flex-col gap-1">
            <span className="text-sm font-medium text-(--color-foreground)">
              {toolCall.tool_name}
            </span>
            <span className={cn("text-xs", config.color)}>{config.label}</span>
          </div>
        </div>
        {toolCall.status === "pending" && (
          <Loader2 className="h-4 w-4 animate-spin text-(--color-muted-foreground)" />
        )}
      </div>

      {expanded && (
        <div className="mt-3 space-y-2 border-t border-(--color-border-subtle) pt-3">
          {/* Arguments */}
          <div>
            <div className="mb-1 text-xs font-medium text-(--color-muted-foreground)">
              Arguments
            </div>
            <pre className="overflow-x-auto rounded bg-(--color-background) p-2 text-xs text-(--color-foreground)">
              {JSON.stringify(toolCall.arguments, null, 2)}
            </pre>
          </div>

          {/* Result */}
          {toolCall.result && (
            <div>
              <div className="mb-1 text-xs font-medium text-(--color-muted-foreground)">
                Result
              </div>
              <pre className="overflow-x-auto rounded bg-(--color-background) p-2 text-xs text-(--color-foreground)">
                {JSON.stringify(toolCall.result, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
