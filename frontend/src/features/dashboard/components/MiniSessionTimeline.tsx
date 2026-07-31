// Mini Session Timeline Component
//
// Displays a compact, vertical timeline of recent session events.

import type { SessionEventSummary } from "@/types/session";

interface MiniSessionTimelineProps {
  events: SessionEventSummary[];
  maxEvents?: number;
}

export function MiniSessionTimeline({ events, maxEvents = 5 }: MiniSessionTimelineProps) {
  const displayEvents = events.slice(0, maxEvents);

  if (displayEvents.length === 0) {
    return null;
  }

  const formatEventType = (type: string): string => {
    const formatted = type.replace(/_/g, " ");
    return formatted.charAt(0).toUpperCase() + formatted.slice(1);
  };

  return (
    <div className="flex flex-col gap-1.5">
      {displayEvents.map((event, i) => (
        <div key={i} className="flex items-center gap-2 text-xs">
          <span className="font-mono text-[10px] text-(--color-faint-foreground) tabular-nums">
            {new Date(event.occurredAt).toLocaleTimeString("en-US", {
              hour: "2-digit",
              minute: "2-digit",
              hour12: false,
            })}
          </span>
          <div className="h-px flex-1 bg-(--color-border-subtle)" />
          <span className="text-(--color-muted-foreground)">
            {formatEventType(event.eventType)}
            {event.fileName && (
              <span className="ml-1 font-(family-name:--font-mono) text-(--color-foreground)">
                {event.fileName}
              </span>
            )}
          </span>
        </div>
      ))}
      {events.length > maxEvents && (
        <span className="text-[10px] text-(--color-faint-foreground)">
          +{events.length - maxEvents} more event{events.length - maxEvents > 1 ? "s" : ""}
        </span>
      )}
    </div>
  );
}
