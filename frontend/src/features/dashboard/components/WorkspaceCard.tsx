import { FolderGit2, FileCode, Files, Activity, Archive } from "lucide-react";
import { Card, CardContent } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import type { Workspace, WorkspaceStats } from "@/types/workspace";

interface WorkspaceCardProps {
  workspace: Workspace;
  stats?: WorkspaceStats;
  onOpen?: (workspace: Workspace) => void;
}

const IDLE_BADGE_THRESHOLD_DAYS = 4;

function idleDays(lastActiveAt: string): number {
  return Math.floor((Date.now() - new Date(lastActiveAt).getTime()) / (24 * 60 * 60 * 1000));
}

const LANG_STYLES: Record<string, { label: string; style: React.CSSProperties }> = {
  rust: { label: "Rust", style: { color: "var(--color-danger)" } },
  node: { label: "Node", style: { color: "var(--color-success)" } },
  python: { label: "Python", style: { color: "var(--color-accent)" } },
  java: { label: "Java", style: { color: "var(--color-danger)" } },
  gradle: { label: "Gradle", style: { color: "var(--color-accent-muted)" } },
  go: { label: "Go", style: { color: "var(--color-accent-muted)" } },
  git: { label: "Git", style: { color: "var(--color-warning)" } },
};

export function detectLanguage(rootPath: string | null): { label: string; style: React.CSSProperties } | null {
  if (!rootPath) return null;
  const lower = rootPath.toLowerCase();
  if (/cargo\.toml/.test(lower)) return LANG_STYLES.rust;
  if (/package\.json/.test(lower)) return LANG_STYLES.node;
  if (/pyproject\.toml|setup\.py|requirements\.txt/.test(lower)) return LANG_STYLES.python;
  if (/pom\.xml/.test(lower)) return LANG_STYLES.java;
  if (/build\.gradle/.test(lower)) return LANG_STYLES.gradle;
  if (/go\.mod/.test(lower)) return LANG_STYLES.go;
  if (/\.git/.test(lower)) return LANG_STYLES.git;
  return null;
}

export function WorkspaceCard({ workspace, stats, onOpen }: WorkspaceCardProps) {
  const idle = idleDays(workspace.lastActiveAt);
  const lang = detectLanguage(workspace.rootPath);

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
      <div className="flex items-start justify-between gap-3 p-4 pb-2">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-sm font-bold text-(--color-foreground)">{workspace.name}</h3>
            {workspace.status === "archived" && (
              <Badge variant="neutral">
                <Archive className="mr-0.5 h-2.5 w-2.5" strokeWidth={2} />
                Archived
              </Badge>
            )}
          </div>
          <p className="mt-0.5 text-xs text-(--color-faint-foreground)">{formatRelativeTime(workspace.lastActiveAt)}</p>
        </div>
        <ProgressRing value={workspace.healthScore} size={36} strokeWidth={3} />
      </div>
      <CardContent>
        <div className="flex items-center gap-3">
          <div className="flex min-w-0 flex-1 items-center gap-1.5 text-xs text-(--color-muted-foreground)">
            {workspace.rootPath && (
              <>
                <FolderGit2 className="h-3.5 w-3.5 shrink-0" strokeWidth={1.75} />
                <span className="truncate font-(family-name:--font-mono)">{workspace.rootPath}</span>
              </>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            {lang && (
              <Badge variant="neutral">
                <FileCode className="mr-1 h-3 w-3" strokeWidth={1.75} style={lang.style} />
                {lang.label}
              </Badge>
            )}
            {idle >= IDLE_BADGE_THRESHOLD_DAYS && <Badge variant="neutral">Idle {idle}d</Badge>}
          </div>
        </div>
        {stats && (
          <div className="mt-3 flex items-center gap-3 border-t border-(--color-border-subtle) pt-3 text-xs text-(--color-faint-foreground)">
            <span className="flex items-center gap-1">
              <Files className="h-3 w-3" strokeWidth={1.75} />
              {stats.fileCount} file{stats.fileCount !== 1 ? "s" : ""}
            </span>
            <span className="flex items-center gap-1">
              <Activity className="h-3 w-3" strokeWidth={1.75} />
              {stats.timelineEventCount} event{stats.timelineEventCount !== 1 ? "s" : ""}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
