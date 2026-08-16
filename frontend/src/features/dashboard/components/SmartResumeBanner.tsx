// Smart Resume Banner Component
//
// The "Continue Working" hero surface — Level 1 chrome glass with real
// refraction where supported. Displays the last work context and lets the
// user resume it in one action.

import { ArrowRight, X } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { MiniSessionTimeline } from "./MiniSessionTimeline";
import type { SessionSummary } from "@/types/session";

interface SmartResumeBannerProps {
  session: SessionSummary;
  onResume: () => void;
  onDismiss: () => void;
}

export function SmartResumeBanner({ session, onResume, onDismiss }: SmartResumeBannerProps) {
  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);

    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  return (
    <Card className="relative rounded-3xl">
      <div className="flex flex-col gap-6 p-6 lg:flex-row lg:items-center lg:justify-between lg:p-7">
        <div className="flex min-w-0 items-center gap-5">
          <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-(--color-accent-muted) ring-1 ring-(--color-accent)/25 shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
            <ArrowRight className="h-5 w-5 text-(--color-accent)" strokeWidth={1.75} />
          </div>

          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
                Continue Working
              </p>
              <Badge variant="accent" className="shrink-0">
                {formatDuration(session.durationSeconds)}
              </Badge>
            </div>
            <h2 className="mt-1 truncate font-(family-name:--font-display) text-2xl font-semibold tracking-tight text-(--color-foreground)">
              {session.workspaceName}
            </h2>
            <p className="mt-1 text-[13px] text-(--color-muted-foreground)">
              {session.fileCount} file{session.fileCount !== 1 ? "s" : ""}
              {session.languages.length > 0 && (
                <>
                  {" "}· {session.languages.slice(0, 3).join(", ")}
                  {session.languages.length > 3 && ` +${session.languages.length - 3}`}
                </>
              )}
            </p>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-5">
          <div className="hidden flex-col items-center gap-1.5 md:flex">
            <ProgressRing value={session.productivityScore} size={48} strokeWidth={4} />
            <span className="text-[11px] font-medium text-(--color-muted-foreground)">
              {Math.round(session.productivityScore)}% productive
            </span>
          </div>
          <div className="flex flex-wrap items-center gap-2.5">
            <Button onClick={onResume} variant="primary" size="lg">
              <ArrowRight className="h-4 w-4" strokeWidth={1.75} />
              Resume session
            </Button>
            <Button onClick={onDismiss} variant="secondary" size="lg">
              Start fresh
            </Button>
          </div>
        </div>
      </div>

      {session.recentEvents.length > 0 && (
        <div className="border-t border-(--color-border-subtle) px-6 py-3.5 lg:px-7">
          <p className="mb-2 text-[11px] text-(--color-faint-foreground)">Recent activity</p>
          <MiniSessionTimeline events={session.recentEvents} maxEvents={5} />
        </div>
      )}

      <button
        onClick={onDismiss}
        className="absolute right-3 top-3 rounded p-1 text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
        aria-label="Dismiss"
      >
        <X className="h-4 w-4" strokeWidth={1.75} />
      </button>
    </Card>
  );
}
