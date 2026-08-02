// AutonomousControls - pause/resume/cancel buttons for an autonomous session.

import { Ban, Pause, Play } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { AutonomousStatus } from "@/types/autonomous";

const isTerminal = (status: AutonomousStatus) =>
  status === "completed" || status === "failed" || status === "cancelled";

export function AutonomousControls({
  status,
  onPause,
  onResume,
  onCancel,
  busy,
}: {
  status: AutonomousStatus;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  busy: boolean;
}) {
  const running = status === "running";
  const paused = status === "paused";

  if (isTerminal(status)) {
    return (
      <span className="text-sm font-medium text-(--color-muted-foreground)" data-testid="terminal-note">
        {status === "completed"
          ? "Session completed"
          : status === "cancelled"
          ? "Session cancelled"
          : "Session failed"}
      </span>
    );
  }

  return (
    <div className="flex items-center gap-2" data-testid="autonomous-controls">
      {running && (
        <Button
          variant="secondary"
          size="sm"
          onClick={onPause}
          disabled={busy}
          data-testid="pause-button"
        >
          <Pause className="h-4 w-4" />
          Pause
        </Button>
      )}
      {paused && (
        <Button
          variant="secondary"
          size="sm"
          onClick={onResume}
          disabled={busy}
          data-testid="resume-button"
        >
          <Play className="h-4 w-4" />
          Resume
        </Button>
      )}
      {!isTerminal(status) && (
        <Button
          variant="danger"
          size="sm"
          onClick={onCancel}
          disabled={busy}
          data-testid="cancel-button"
        >
          <Ban className="h-4 w-4" />
          Cancel
        </Button>
      )}
    </div>
  );
}