// ExecutionControls - pause/resume/cancel buttons, disabled by execution status.

import { Ban, Pause, Play } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { ExecutionStatus } from "@/types/execution";

interface ExecutionControlsProps {
  status: ExecutionStatus;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  busy: boolean;
}

const isTerminal = (status: ExecutionStatus) =>
  status === "completed" || status === "failed" || status === "cancelled";

export function ExecutionControls({
  status,
  onPause,
  onResume,
  onCancel,
  busy,
}: ExecutionControlsProps) {
  const running = status === "running";
  const paused = status === "paused";

  if (isTerminal(status)) {
    return (
      <span className="text-sm font-medium text-(--color-muted-foreground)" data-testid="terminal-note">
        {status === "completed"
          ? "Execution completed"
          : status === "cancelled"
            ? "Execution cancelled"
            : "Execution failed"}
      </span>
    );
  }

  return (
    <div className="flex items-center gap-2" data-testid="execution-controls">
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