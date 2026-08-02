// ExecutionTimeline - live feed of recent execution events (checkpoint saves,
// step completions/failures, pause/resume) from the streamed snapshot.

import { History } from "lucide-react";
import type { ExecutionEvent, ExecutionEventType } from "@/types/execution";

const EVENT_LABEL: Record<ExecutionEventType, string> = {
  started: "Started",
  step_started: "Step started",
  step_completed: "Step completed",
  step_failed: "Step failed",
  paused: "Paused",
  resumed: "Resumed",
  checkpoint_saved: "Checkpoint saved",
  checkpoint_loaded: "Checkpoint loaded",
  completed: "Completed",
  failed: "Failed",
  cancelled: "Cancelled",
};

function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

export function ExecutionTimeline({ events }: { events: ExecutionEvent[] }) {
  // Backend returns most-recent-first; show newest at the top.
  const ordered = [...events].reverse();

  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
      <h2 className="mb-3 flex items-center gap-2 font-(family-name:--font-display) text-sm font-semibold text-(--color-foreground)">
        <History className="h-4 w-4 text-(--color-accent)" />
        Live Timeline
      </h2>

      {ordered.length === 0 && (
        <p className="text-sm text-(--color-muted-foreground)">No events recorded yet.</p>
      )}

      <ol className="relative space-y-2 before:absolute before:left-[5px] before:top-1 before:bottom-1 before:w-px before:bg-(--color-border-subtle)">
        {ordered.map((event) => (
          <li key={event.id} className="relative pl-6" data-event-type={event.event_type}>
            <span
              className="absolute left-0 top-1.5 h-3 w-3 rounded-full border-2 border-(--color-surface-raised) bg-(--color-border-subtle)"
              aria-hidden="true"
            />
            <div className="flex items-baseline justify-between gap-2">
              <p className="truncate text-sm text-(--color-foreground)">
                <span className="font-medium">{EVENT_LABEL[event.event_type] ?? event.event_type}</span>
                {event.step_number !== null && event.step_number !== undefined && (
                  <span className="ml-1 text-xs text-(--color-muted-foreground)">
                    step {event.step_number + 1}
                  </span>
                )}
              </p>
              <span className="shrink-0 text-xs tabular-nums text-(--color-faint-foreground)">
                {formatTime(event.created_at)}
              </span>
            </div>
            {event.message && (
              <p className="mt-0.5 truncate text-xs text-(--color-muted-foreground)">
                {event.message}
              </p>
            )}
          </li>
        ))}
      </ol>
    </div>
  );
}