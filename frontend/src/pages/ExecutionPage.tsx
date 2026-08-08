// ExecutionPage - lists recent plan executions and shows the live dashboard
// for the selected one. On mount it pulls `execution_list_recent` to
// re-attach (reconnect) to the last in-flight run.

import { useCallback, useEffect, useState } from "react";
import { ListChecks } from "lucide-react";
import { executionRepository } from "@/services/executionRepository";
import { ExecutionDashboard } from "@/features/execution/ExecutionDashboard";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
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
    <PageContainer>
      <PageHeader
        eyebrow="Runs"
        title="Executions"
        description="Live status of approved plan runs — DAG progress, controls, and planner reports."
      />

      {recent.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
            Recent
          </span>
          <span className="h-3 w-px bg-(--color-border)" />
          {recent.map((execution) => (
            <button
              key={execution.execution_id}
              onClick={() => setSelectedId(execution.execution_id)}
              data-selected={execution.execution_id === selectedId}
              className={
                "rounded-full border px-3 py-1.5 text-xs transition-all duration-200 " +
                (execution.execution_id === selectedId
                  ? "border-(--color-accent)/50 bg-(--color-accent)/10 text-(--color-accent)"
                  : "border-(--color-border) bg-(--color-surface) text-(--color-muted-foreground) hover:border-(--color-border-subtle) hover:text-(--color-foreground)")
              }
            >
              {execution.plan?.goal ?? execution.execution_id.slice(0, 8)} · {execution.status}
            </button>
          ))}
        </div>
      )}

      {loading && (
        <div className="space-y-4">
          <div className="h-16 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
          <div className="h-72 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
        </div>
      )}

      {!loading && recent.length === 0 && (
        <EmptyState
          icon={<ListChecks className="h-4 w-4" strokeWidth={1.75} />}
          title="No executions yet"
          description="Approve a plan in the AI Copilot to start one — its DAG progress and planner reports will appear here."
        />
      )}

      {selectedId && <ExecutionDashboard executionId={selectedId} />}
    </PageContainer>
  );
}
