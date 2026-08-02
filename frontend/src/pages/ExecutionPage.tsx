// ExecutionPage - lists recent plan executions and shows the live dashboard
// for the selected one. On mount it pulls `execution_list_recent` to
// re-attach (reconnect) to the last in-flight run.

import { useCallback, useEffect, useState } from "react";
import { executionRepository } from "@/services/executionRepository";
import { ExecutionDashboard } from "@/features/execution/ExecutionDashboard";
import type { ExecutionProgress } from "@/types/execution";

export function ExecutionPage() {
  const [recent, setRecent] = useState<ExecutionProgress[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const loadRecent = useCallback(async () => {
    try {
      const executions = await executionRepository.listRecent(10);
      setRecent(executions);
      setSelectedId((prev) => prev ?? executions[0]?.execution_id ?? null);
    } catch (err) {
      console.error("Failed to load recent executions:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadRecent();
  }, [loadRecent]);

  return (
    <div className="mx-auto max-w-5xl space-y-4 p-6">
      <header>
        <h1 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
          Executions
        </h1>
        <p className="text-sm text-(--color-muted-foreground)">
          Live status of approved plan runs — DAG progress, controls, and planner reports.
        </p>
      </header>

      {recent.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs uppercase tracking-wider text-(--color-faint-foreground)">
            Recent:
          </span>
          {recent.map((execution) => (
            <button
              key={execution.execution_id}
              onClick={() => setSelectedId(execution.execution_id)}
              data-selected={execution.execution_id === selectedId}
              className={
                "rounded-md border px-2.5 py-1 text-xs transition-colors " +
                (execution.execution_id === selectedId
                  ? "border-(--color-accent) bg-(--color-accent-muted) text-(--color-accent)"
                  : "border-(--color-border) bg-(--color-surface-raised) text-(--color-muted-foreground) hover:text-(--color-foreground)")
              }
            >
              {execution.plan?.goal ?? execution.execution_id.slice(0, 8)} · {execution.status}
            </button>
          ))}
        </div>
      )}

      {loading && (
        <p className="text-sm text-(--color-muted-foreground)">Loading recent executions…</p>
      )}

      {!loading && recent.length === 0 && (
        <p className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-sm text-(--color-muted-foreground)">
          No executions yet. Approve a plan in the AI Copilot to start one.
        </p>
      )}

      {selectedId && <ExecutionDashboard executionId={selectedId} />}
    </div>
  );
}