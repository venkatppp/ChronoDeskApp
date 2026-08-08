// AutonomousReasonLog - renders the reasoning timeline for a session.

import { cn } from "@/utils/cn";
import type { ReasoningEvent } from "@/types/autonomous";

const PHASE_STYLE: Record<string, string> = {
  planning: "text-(--color-accent)",
  executing: "text-(--color-success)",
  observed: "text-cyan-500",
  replanning: "text-(--color-warning)",
  awaiting_approval: "text-yellow-500",
  approval_resolved: "text-lime-500",
  budget_update: "text-purple-500",
  pause: "text-(--color-muted-foreground)",
  terminal: "text-(--color-danger)",
};

const PHASE_LABEL: Record<string, string> = {
  planning: "Planning",
  executing: "Executing",
  observed: "Observed",
  replanning: "Replanning",
  awaiting_approval: "Awaiting Approval",
  approval_resolved: "Approval Resolved",
  budget_update: "Budget Update",
  pause: "Pause",
  terminal: "Terminal",
};

export function AutonomousReasonLog({ events }: { events: ReasoningEvent[] }) {
  if (events.length === 0) {
    return (
      <p className="text-sm text-(--color-muted-foreground) text-center py-4">
        No reasoning events yet.
      </p>
    );
  }

  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
      <h3 className="mb-3 font-medium text-sm text-(--color-foreground)">
        Reasoning Log (newest first)
      </h3>
      <div className="space-y-2 max-h-64 overflow-y-auto">
        {events
          .slice()
          .reverse()
          .map((event) => (
            <div
              key={event.created_at}
              className="flex gap-2 text-xs"
              data-testid="reasoning-event"
            >
              <span
                className={cn(
                  "shrink-0 w-28 font-mono text-(--color-muted-foreground)",
                  PHASE_STYLE[event.phase]
                )}
              >
                {PHASE_LABEL[event.phase] ?? event.phase}
              </span>
              <span className="text-(--color-foreground)">{event.message}</span>
              {event.detail && (
                <span className="text-(--color-faint-foreground) font-mono">
                  {JSON.stringify(event.detail)}
                </span>
              )}
            </div>
          ))}
      </div>
    </div>
  );
}