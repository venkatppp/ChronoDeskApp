// ExecutionDashboard - live view of a plan execution: status bar, DAG
// progress, controls, timeline, and planner report. Listens to
// `execution:progress` and re-syncs on reconnect via `execution_get_progress`.

import { useState } from "react";
import { Activity } from "lucide-react";
import { useExecutionStream } from "@/hooks/useExecutionStream";
import { executionRepository } from "@/services/executionRepository";
import { ExecutionDagView } from "./ExecutionDagView";
import { ExecutionStatusPill } from "./ExecutionStatusPill";
import { ExecutionTimeline } from "./ExecutionTimeline";
import { ExecutionControls } from "./ExecutionControls";
import { PlannerReportPanel } from "./PlannerReportPanel";

interface ExecutionDashboardProps {
  executionId: string;
}

export function ExecutionDashboard({ executionId }: ExecutionDashboardProps) {
  const { progress, loading, error, refresh } = useExecutionStream(executionId);
  const [busy, setBusy] = useState(false);

  const runControl =
    (action: (id: string) => Promise<void>) =>
    async () => {
      setBusy(true);
      try {
        await action(executionId);
        await refresh();
      } finally {
        setBusy(false);
      }
    };

  const pause = runControl((id) => executionRepository.pause(id));
  const resume = runControl((id) => executionRepository.resume(id));
  const cancel = runControl((id) => executionRepository.cancel(id));

  if (loading && !progress) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="flex items-center gap-3 text-sm text-(--color-muted-foreground)">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-(--color-border) border-t-(--color-accent)" />
          Loading execution…
        </div>
      </div>
    );
  }

  if (error || !progress) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="max-w-md text-center text-sm text-(--color-muted-foreground)">
          {error ?? "Execution not found."}
        </p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl space-y-4 p-6">
      <header className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-2">
          <Activity className="h-5 w-5 text-(--color-accent)" />
          <div>
            <h1 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
              {progress.plan?.goal ?? "Execution"}
            </h1>
            <p className="text-xs font-mono text-(--color-faint-foreground)">{executionId}</p>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <ExecutionStatusPill status={progress.status} />
          <ExecutionControls
            status={progress.status}
            onPause={pause}
            onResume={resume}
            onCancel={cancel}
            busy={busy}
          />
        </div>
      </header>

      {/* Progress summary */}
      <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
        <div className="mb-2 flex items-baseline justify-between text-sm">
          <span className="text-(--color-muted-foreground)">
            Step {Math.min(progress.current_step + 1, progress.total_steps)} of {progress.total_steps}
          </span>
          <span className="font-medium tabular-nums text-(--color-foreground)">
            {Math.round(progress.progress_percentage)}%
          </span>
        </div>
        <div className="h-2 w-full overflow-hidden rounded-full bg-(--color-surface-hover)">
          <div
            className="h-full rounded-full bg-(--color-accent) transition-all"
            style={{ width: `${progress.progress_percentage}%` }}
            data-testid="progress-bar"
          />
        </div>
      </div>

      <ExecutionDagView progress={progress} />

      <div className="grid gap-4 md:grid-cols-2">
        <ExecutionTimeline events={progress.recent_events} />
        {progress.planner_report && <PlannerReportPanel report={progress.planner_report} />}
      </div>

      <p className="text-xs text-(--color-faint-foreground)">
        Live updates stream over <code className="font-mono">execution:progress</code>; this view stays in
        sync without polling.
      </p>
    </div>
  );
}