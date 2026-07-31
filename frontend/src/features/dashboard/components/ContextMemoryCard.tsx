import { Archive, GitBranch } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { ContextSnapshot } from "@/types/contextMemory";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

interface ContextMemoryCardProps {
  snapshot: ContextSnapshot | null;
  isLoading: boolean;
}

export function ContextMemoryCard({ snapshot, isLoading }: ContextMemoryCardProps) {
  if (isLoading) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
            <Archive className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Context Memory
            </p>
            <div className="mt-2 h-4 w-32 animate-pulse rounded bg-(--color-surface)" />
          </div>
        </div>
      </Card>
    );
  }

  if (!snapshot) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-surface-hover)">
            <Archive className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Context Memory
            </p>
            <p className="mt-1 text-sm text-(--color-muted-foreground)">No context saved yet</p>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-4">
      <div className="flex items-start gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
          <Archive className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            Context Memory
          </p>
          <p className="mt-1 text-sm font-medium text-(--color-foreground)">
            Last worked on {formatRelativeTime(snapshot.capturedAt)}
          </p>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            {snapshot.activeFiles.length > 0 && (
              <span className="inline-flex items-center gap-1 text-xs text-(--color-muted-foreground)">
                <GitBranch className="h-3 w-3" strokeWidth={2} />
                {snapshot.activeFiles.length} file{snapshot.activeFiles.length !== 1 ? 's' : ''}
              </span>
            )}
            {snapshot.healthScore !== undefined && (
              <span className="inline-flex items-center gap-1 text-xs text-(--color-muted-foreground)">
                Health: {Math.round(snapshot.healthScore)}%
              </span>
            )}
          </div>
        </div>
      </div>
    </Card>
  );
}
