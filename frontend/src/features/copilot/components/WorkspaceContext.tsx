// WorkspaceContext - Current workspace indicator and context chip

import { Folder, ChevronDown } from "lucide-react";
import { cn } from "@/utils/cn";

interface WorkspaceContextProps {
  workspaceName: string | null;
  onSwitch?: () => void;
}

export function WorkspaceContext({ workspaceName, onSwitch }: WorkspaceContextProps) {
  if (!workspaceName) return null;

  return (
    <div className="border-b border-(--color-border-subtle) bg-(--color-surface) px-4 py-2">
      <button
        onClick={onSwitch}
        className={cn(
          "flex items-center gap-2 rounded-lg border border-(--color-border) bg-(--color-surface-raised) px-3 py-2 text-sm transition-colors",
          onSwitch && "hover:border-(--color-accent) hover:bg-(--color-surface-hover)"
        )}
        disabled={!onSwitch}
      >
        <Folder className="h-4 w-4 text-(--color-accent)" />
        <span className="text-(--color-foreground)">Context: {workspaceName}</span>
        {onSwitch && <ChevronDown className="ml-auto h-4 w-4 text-(--color-muted-foreground)" />}
      </button>
    </div>
  );
}
