import { Network, TrendingUp } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { RelatedWorkspace } from "@/types/contextMemory";

interface RelatedWorkCardProps {
  relatedWorkspaces: RelatedWorkspace[];
  isLoading: boolean;
}

const RELATIONSHIP_LABELS: Record<string, { label: string; color: string }> = {
  shared_files: { label: "Shared Files", color: "text-(--color-muted-foreground)" },
  shared_folders: { label: "Shared Folders", color: "text-(--color-success)" },
  shared_tech: { label: "Shared Tech", color: "text-(--color-warning)" },
  similar_patterns: { label: "Similar Patterns", color: "text-(--color-muted-foreground)" },
};

export function RelatedWorkCard({ relatedWorkspaces, isLoading }: RelatedWorkCardProps) {
  if (isLoading) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised)">
            <Network className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Related Work
            </p>
            <div className="mt-2 space-y-2">
              <div className="h-4 w-full animate-pulse rounded bg-(--color-surface)" />
              <div className="h-4 w-3/4 animate-pulse rounded bg-(--color-surface)" />
            </div>
          </div>
        </div>
      </Card>
    );
  }

  if (relatedWorkspaces.length === 0) {
    return (
      <Card className="p-4">
        <div className="flex items-start gap-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-hover)">
            <Network className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Related Work
            </p>
            <p className="mt-1 text-sm text-(--color-muted-foreground)">No related workspaces</p>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card className="p-4">
      <div className="flex items-start gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised)">
          <Network className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            Related Work
          </p>
          <div className="mt-2 space-y-2">
            {relatedWorkspaces.slice(0, 3).map((related) => {
              const relType = RELATIONSHIP_LABELS[related.relationshipType] || {
                label: "Related",
                color: "text-(--color-muted-foreground)",
              };
              return (
                <div key={related.workspaceId} className="flex items-start justify-between gap-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-(--color-foreground)">
                      {related.workspaceName}
                    </p>
                    <div className="flex items-center gap-2">
                      <span className={`text-xs ${relType.color}`}>{relType.label}</span>
                      <span className="text-xs text-(--color-faint-foreground)">
                        {Math.round(related.strength * 100)}% match
                      </span>
                    </div>
                  </div>
                  <TrendingUp
                    className="h-3.5 w-3.5 shrink-0 text-(--color-success)"
                    strokeWidth={1.75}
                  />
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </Card>
  );
}
