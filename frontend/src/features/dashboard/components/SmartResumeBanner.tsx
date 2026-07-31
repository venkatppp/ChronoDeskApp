// Smart Resume Banner Component
//
// Displays the "Continue Working" banner when a recent session is available.
// Allows users to quickly resume their last work context.

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
    <Card className="relative overflow-hidden border-(--color-accent)/40 bg-gradient-to-br from-(--color-accent)/5 to-transparent">
      <div className="flex items-start gap-4 p-5">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-(--color-accent)">
          <ArrowRight className="h-6 w-6 text-white" strokeWidth={1.75} />
        </div>

        <div className="min-w-0 flex-1">
          <div className="mb-1 flex items-center gap-2">
            <h3 className="font-(family-name:--font-display) text-lg font-bold text-(--color-foreground)">
              Continue Working
            </h3>
            <Badge variant="accent" className="shrink-0">
              {formatDuration(session.durationSeconds)}
            </Badge>
          </div>

          <p className="mb-3 text-sm text-(--color-muted-foreground)">
            {session.workspaceName} · {session.fileCount} file{session.fileCount !== 1 ? "s" : ""}{" "}
            {session.languages.length > 0 && (
              <>
                · {session.languages.slice(0, 3).join(", ")}
                {session.languages.length > 3 && ` +${session.languages.length - 3}`}
              </>
            )}
          </p>

          <MiniSessionTimeline events={session.recentEvents} maxEvents={5} />

          <div className="mt-4 flex items-center gap-2">
            <Button onClick={onResume} variant="primary" size="sm">
              <ArrowRight className="mr-2 h-4 w-4" strokeWidth={1.75} />
              Resume Session
            </Button>
            <Button onClick={onDismiss} variant="ghost" size="sm">
              Start Fresh
            </Button>
          </div>
        </div>

        <div className="flex shrink-0 flex-col items-center gap-2">
          <ProgressRing value={session.productivityScore} size={56} strokeWidth={4} />
          <span className="text-xs font-medium text-(--color-muted-foreground)">
            {Math.round(session.productivityScore)}% productive
          </span>
        </div>

        <button
          onClick={onDismiss}
          className="absolute right-3 top-3 rounded p-1 text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          aria-label="Dismiss"
        >
          <X className="h-4 w-4" strokeWidth={1.75} />
        </button>
      </div>
    </Card>
  );
}
