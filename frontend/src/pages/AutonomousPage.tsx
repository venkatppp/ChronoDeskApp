// AutonomousPage - lists recent autonomous sessions and shows the live
// dashboard for the selected one. On mount it pulls `autonomous_list_recent`
// to re-attach (reconnect) to the last in-flight run.

import { useCallback, useEffect, useState } from "react";
import { Bot } from "lucide-react";
import { autonomousRepository } from "@/services/autonomousRepository";
import { AutonomousDashboard } from "@/features/autonomous/AutonomousDashboard";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import type { AutonomousSessionProgress } from "@/types/autonomous";

export function AutonomousPage() {
  const [recent, setRecent] = useState<AutonomousSessionProgress[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const loadRecent = useCallback(async () => {
    try {
      const sessions = await autonomousRepository.listRecent(10);
      setRecent(sessions);
      setSelectedId((prev) => prev ?? sessions[0]?.session_id ?? null);
    } catch (err) {
      console.error("Failed to load recent sessions:", err);
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
        title="Autonomous Sessions"
        description="Live status of autonomous agent runs — budgets, reasoning, approvals, and controls."
      />

      {recent.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
            Recent
          </span>
          <span className="h-3 w-px bg-(--color-border)" />
          {recent.map((session) => (
            <button
              key={session.session_id}
              onClick={() => setSelectedId(session.session_id)}
              data-selected={session.session_id === selectedId}
              className={
                "rounded-full border px-3 py-1.5 text-xs transition-all duration-200 " +
                (session.session_id === selectedId
                  ? "border-(--color-accent)/50 bg-(--color-accent)/10 text-(--color-accent)"
                  : "border-(--color-border) bg-(--color-surface) text-(--color-muted-foreground) hover:border-(--color-border-subtle) hover:text-(--color-foreground)")
              }
            >
              {session.goal.slice(0, 40)} · {session.status}
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
          icon={<Bot className="h-4 w-4" strokeWidth={1.75} />}
          title="No autonomous sessions yet"
          description="Start one from the AI Copilot or via API — its live progress, budget, and reasoning will appear here."
        />
      )}

      {selectedId && <AutonomousDashboard sessionId={selectedId} />}
    </PageContainer>
  );
}
