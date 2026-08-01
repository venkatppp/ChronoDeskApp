// SuggestedActions - Quick action chips for follow-up prompts

import { Zap } from "lucide-react";
import { cn } from "@/utils/cn";
import type { SuggestedAction } from "@/types/copilot";

interface SuggestedActionsProps {
  actions: SuggestedAction[];
  onExecute: (action: SuggestedAction) => void;
  disabled?: boolean;
}

export function SuggestedActions({ actions, onExecute, disabled }: SuggestedActionsProps) {
  if (actions.length === 0) return null;

  return (
    <div className="border-t border-(--color-border-subtle) bg-(--color-surface) px-4 py-3">
      <div className="mb-2 flex items-center gap-1.5 text-xs font-medium text-(--color-muted-foreground)">
        <Zap className="h-3.5 w-3.5" />
        <span>Suggested Actions</span>
      </div>
      <div className="flex flex-wrap gap-2">
        {actions.map((action, idx) => (
          <button
            key={idx}
            onClick={() => onExecute(action)}
            disabled={disabled}
            className={cn(
              "rounded-lg border border-(--color-border) bg-(--color-surface-raised) px-3 py-2 text-left transition-colors",
              "hover:border-(--color-accent) hover:bg-(--color-surface-hover)",
              "disabled:cursor-not-allowed disabled:opacity-50"
            )}
          >
            <div className="text-sm font-medium text-(--color-foreground)">{action.title}</div>
            <div className="mt-0.5 text-xs text-(--color-muted-foreground)">
              {action.description}
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}
