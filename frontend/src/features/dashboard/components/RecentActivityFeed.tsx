import { FilePlus, FileEdit, FileX, FolderInput, type LucideIcon } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/Card";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import type { TimelineEvent, TimelineEventType } from "@/types/timeline";

const EVENT_ICON: Partial<Record<TimelineEventType, LucideIcon>> = {
  create: FilePlus,
  edit: FileEdit,
  delete: FileX,
  workspace_switch: FolderInput,
};

const EVENT_LABEL: Record<TimelineEventType, string> = {
  create: "created",
  open: "opened",
  close: "closed",
  edit: "edited",
  move: "moved",
  delete: "deleted",
  commit: "committed",
  visit: "visited",
  screenshot: "captured",
  workspace_switch: "workspace activity",
};

function eventFileName(event: TimelineEvent): string | null {
  const path = event.metadata?.path;
  if (typeof path !== "string") return null;
  return path.split(/[/\\]/).pop() ?? path;
}

interface RecentActivityFeedProps {
  events: TimelineEvent[];
  isLoading: boolean;
}

/**
 * Live feed of recent timeline events (blueprint §3.2's Timeline
 * screen, surfaced on the dashboard) — real rows from SQLite via
 * `TauriTimelineRepository`, refreshed automatically whenever the
 * watcher pipeline records a new event (see `useDashboardData`'s
 * `useAppEvents` subscription). No polling, no manual refresh.
 */
export function RecentActivityFeed({ events, isLoading }: RecentActivityFeedProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent activity</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        {isLoading && (
          <div className="flex flex-col gap-2">
            {[0, 1, 2].map((i) => (
              <div key={i} className="h-8 animate-pulse rounded-[var(--radius-control)] bg-(--color-surface-hover)" />
            ))}
          </div>
        )}

        {!isLoading && events.length === 0 && (
          <p className="py-2 text-sm text-(--color-faint-foreground)">
            No activity yet. Watch a folder to see file changes appear here.
          </p>
        )}

        {!isLoading &&
          events.map((event) => {
            const Icon = EVENT_ICON[event.eventType] ?? FileEdit;
            const fileName = eventFileName(event);

            return (
              <div key={event.id} className="flex items-center gap-2.5 rounded-[var(--radius-control)] px-2 py-2 text-sm">
                <Icon className="h-4 w-4 shrink-0 text-(--color-accent)" strokeWidth={1.75} />
                <span className="min-w-0 flex-1 truncate text-(--color-foreground)">
                  {fileName ? (
                    <>
                      <span className="font-(family-name:--font-mono)">{fileName}</span>{" "}
                      <span className="text-(--color-muted-foreground)">{EVENT_LABEL[event.eventType]}</span>
                    </>
                  ) : (
                    <span className="text-(--color-muted-foreground)">{EVENT_LABEL[event.eventType]}</span>
                  )}
                </span>
                <span className="shrink-0 font-(family-name:--font-mono) text-xs text-(--color-faint-foreground)">
                  {formatRelativeTime(event.occurredAt)}
                </span>
              </div>
            );
          })}
      </CardContent>
    </Card>
  );
}
