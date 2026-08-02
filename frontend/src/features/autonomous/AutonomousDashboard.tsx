// AutonomousDashboard - live view of one autonomous session: status,
// budgets, reasoning timeline, approval checkpoints, and stop controls.
// Listens to `autonomous:session` + `autonomous:reasoning` and re-syncs on
// reconnect via `autonomous_get_progress`.

import { useState } from "react";
import { Bot } from "lucide-react";
import { useAutonomousStream } from "@/hooks/useAutonomousStream";
import { autonomousRepository } from "@/services/autonomousRepository";
import { AutonomousControls } from "./AutonomousControls";
import { AutonomousReasonLog } from "./AutonomousReasonLog";
import { ApprovalGate } from "./AutonomousApprovalGate";
import { ExecutionDigest } from "./ExecutionDigest";
import { cn } from "@/utils/cn";
import type { AutonomousStatus } from "@/types/autonomous";

const STATUS_STYLE: Record<AutonomousStatus, string> = {
  running: "border-(--color-accent) bg-(--color-accent-muted) text-(--color-accent)",
  paused: "border-amber-500/60 bg-amber-500/10 text-amber-500",
  waiting_approval: "border-yellow-400/60 bg-yellow-400/10 text-yellow-400",
  completed: "border-(--color-success) bg-(--color-success-muted) text-(--color-success)",
  failed: "border-(--color-destructive) bg-(--color-destructive-muted) text-(--color-destructive)",
  cancelled: "border-(--color-border) text-(--color-muted-foreground)",
};

interface AutonomousDashboardProps {
  sessionId: string;
}

export function AutonomousDashboard({ sessionId }: AutonomousDashboardProps) {
  const { progress, reasoning, loading, error, refresh } = useAutonomousStream(sessionId);
  const [busy, setBusy] = useState(false);

  const act = (action: (id: string) => Promise<unknown>) => async () => {
    setBusy(true);
    try {
      await action(sessionId);
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const pause = act((id) => autonomousRepository.pause(id));
  const resume = act((id) => autonomousRepository.resume(id));
  const cancel = act((id) => autonomousRepository.cancel(id));
  const approve = act((id) => autonomousRepository.approve(id));
  const reject = act((id) => autonomousRepository.reject(id));

  if (loading && !progress) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="flex items-center gap-3 text-sm text-(--color-muted-foreground)">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-(--color-border) border-t-(--color-accent)" />
          Loading session…
        </div>
      </div>
    );
  }

  if (error || !progress) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="max-w-md text-center text-sm text-(--color-muted-foreground)">
          {error ?? "Session not found."}
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4" data-testid="autonomous-dashboard">
      <header className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          <Bot className="h-5 w-5 text-(--color-accent)" />
          <div>
            <h2 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
              {progress.goal}
            </h2>
            <p className="text-xs font-mono text-(--color-faint-foreground)">{sessionId}</p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <span
            className={cn(
              "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-medium capitalize",
              STATUS_STYLE[progress.status]
            )}
            data-status={progress.status}
          >
            {progress.status.replace("_", " ")}
          </span>
          <AutonomousControls
            status={progress.status}
            onPause={pause}
            onResume={resume}
            onCancel={cancel}
            busy={busy}
          />
        </div>
      </header>

      <ExecutionDigest
        plansAttempted={progress.plans_attempted}
        plansCompleted={progress.plans_completed}
        stepsCompleted={progress.steps_completed}
        stepsLeft={progress.steps_left}
        retriesUsed={progress.retries_used}
        replansUsed={progress.replans_used}
      />

      {progress.pending_approval && (
        <ApprovalGate
          request={progress.pending_approval}
          onApprove={approve}
          onReject={reject}
          busy={busy}
        />
      )}

      {progress.error && (
        <p className="rounded-lg border border-(--color-destructive) bg-(--color-destructive-muted) p-3 text-sm text-(--color-destructive)">
          {progress.error}
        </p>
      )}

      <AutonomousReasonLog events={reasoning.length > 0 ? reasoning : progress.reasoning} />
    </div>
  );
}