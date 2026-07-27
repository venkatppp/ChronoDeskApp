import { FolderGit2 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import type { Workspace } from "@/types/workspace";

interface WorkspaceCardProps {
  workspace: Workspace;
  onOpen?: (workspace: Workspace) => void;
}

/** A workspace idle for this long shows an "Idle" badge on its card. */
const IDLE_BADGE_THRESHOLD_DAYS = 4;

function idleDays(lastActiveAt: string): number {
  return Math.floor((Date.now() - new Date(lastActiveAt).getTime()) / (24 * 60 * 60 * 1000));
}

export function WorkspaceCard({ workspace, onOpen }: WorkspaceCardProps) {
  const idle = idleDays(workspace.lastActiveAt);

  return (
    <Card
      role="button"
      tabIndex={0}
      onClick={() => onOpen?.(workspace)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") onOpen?.(workspace);
      }}
      className="cursor-pointer hover:border-(--color-accent) hover:bg-(--color-surface-hover) focus-visible:border-(--color-accent)"
    >
      <CardHeader className="flex-row items-start justify-between gap-2">
        <div className="min-w-0">
          <CardTitle className="truncate">{workspace.name}</CardTitle>
          <p className="mt-1 flex items-center gap-1.5 text-xs text-(--color-faint-foreground)">
            <span className="font-(family-name:--font-mono)">{formatRelativeTime(workspace.lastActiveAt)}</span>
          </p>
        </div>
        <ProgressRing value={workspace.healthScore} size={40} strokeWidth={3.5} />
      </CardHeader>
      <CardContent className="flex items-center justify-between">
        <div className="flex min-w-0 items-center gap-1.5 text-xs text-(--color-muted-foreground)">
          {workspace.rootPath && (
            <>
              <FolderGit2 className="h-3.5 w-3.5 shrink-0" strokeWidth={1.75} />
              <span className="truncate font-(family-name:--font-mono)">{workspace.rootPath}</span>
            </>
          )}
        </div>
        {idle >= IDLE_BADGE_THRESHOLD_DAYS && <Badge variant="neutral">Idle {idle}d</Badge>}
      </CardContent>
    </Card>
  );
}
