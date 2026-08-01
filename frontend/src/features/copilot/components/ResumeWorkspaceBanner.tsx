// ResumeWorkspaceBanner - Resume where you left off

import { Play, X } from "lucide-react";
import { cn } from "@/utils/cn";
import type { ResumeContext } from "@/types/proactive";

interface ResumeWorkspaceBannerProps {
  context: ResumeContext;
  onResume: () => void;
  onDismiss: () => void;
}

export function ResumeWorkspaceBanner({
  context,
  onResume,
  onDismiss,
}: ResumeWorkspaceBannerProps) {
  const timeSinceActive = getTimeSince(context.last_active);

  return (
    <div className="rounded-lg border border-(--color-accent) bg-(--color-accent-muted) p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <Play className="h-5 w-5 text-(--color-accent)" />
            <h3 className="font-semibold text-(--color-foreground)">Resume where you left off?</h3>
          </div>

          <p className="mt-1 text-sm text-(--color-muted-foreground)">
            Last active {timeSinceActive}
          </p>

          {context.unfinished_work.length > 0 && (
            <div className="mt-2">
              <p className="text-xs font-medium text-(--color-muted-foreground)">
                Unfinished work:
              </p>
              <ul className="mt-1 space-y-1">
                {context.unfinished_work.slice(0, 3).map((work, idx) => (
                  <li key={idx} className="flex items-start gap-2 text-sm text-(--color-foreground)">
                    <span className="mt-1 h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-accent)" />
                    <span>{work.description}</span>
                  </li>
                ))}
                {context.unfinished_work.length > 3 && (
                  <li className="text-sm text-(--color-muted-foreground)">
                    +{context.unfinished_work.length - 3} more items
                  </li>
                )}
              </ul>
            </div>
          )}

          <div className="mt-3 flex gap-2">
            <button
              onClick={onResume}
              className={cn(
                "rounded-lg bg-(--color-accent) px-4 py-2 text-sm font-medium text-(--color-accent-foreground)",
                "transition-colors hover:bg-(--color-accent)/90"
              )}
            >
              Resume Work
            </button>
            <button
              onClick={onDismiss}
              className="rounded-lg border border-(--color-border) bg-(--color-surface) px-4 py-2 text-sm text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
            >
              Start Fresh
            </button>
          </div>
        </div>

        <button
          onClick={onDismiss}
          className="rounded p-1 text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          title="Dismiss"
        >
          <X className="h-4 w-4" />
        </button>
      </div>
    </div>
  );
}

function getTimeSince(timestamp: string): string {
  const now = new Date();
  const past = new Date(timestamp);
  const diffMs = now.getTime() - past.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 60) return `${diffMins} minutes ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours} hours ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays} days ago`;
}
