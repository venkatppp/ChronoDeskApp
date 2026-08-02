// AutonomousPage - lists recent autonomous sessions and shows the live
// dashboard for the selected one. On mount it pulls `autonomous_list_recent`
// to re-attach (reconnect) to the last in-flight run.

import { useCallback, useEffect, useState } from "react";
import { Bot } from "lucide-react";
import { autonomousRepository } from "@/services/autonomousRepository";
import { AutonomousDashboard } from "@/features/autonomous/AutonomousDashboard";
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
    <div className="mx-auto max-w-5xl space-y-4 p-6">
      <header>
        <div className="flex items-center gap-2 mb-2">
          <Bot className="h-5 w-5 text-(--color-accent)" />
          <h1 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
            Autonomous Sessions
          </h1>
        </div>
        <p className="text-sm text-(--color-muted-foreground)">
          Live status of autonomous agent runs — budgets, reasoning, approvals, and controls.
        </p>
      </header>

      {recent.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-xs uppercase tracking-wider text-(--color-faint-foreground)">
            Recent:
          </span>
          {recent.map((session) => (
            <button
              key={session.session_id}
              onClick={() => setSelectedId(session.session_id)}
              data-selected={session.session_id === selectedId}
              className={
                "rounded-md border px-2.5 py-1 text-xs transition-colors " +
                (session.session_id === selectedId
                  ? "border-(--color-accent) bg-(--color-accent-muted) text-(--color-accent)"
                  : "border-(--color-border) bg-(--color-surface-raised) text-(--color-muted-foreground) hover:text-(--color-foreground)")
              }
            >
              {session.goal.slice(0, 40)} · {session.status}
            </button>
          ))}
        </div>
      )}

      {loading && (
        <p className="text-sm text-(--color-muted-foreground)">Loading recent sessions…</p>
      )}

      {!loading && recent.length === 0 && (
        <p className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-sm text-(--color-muted-foreground)">
          No autonomous sessions yet. Start one from the AI Copilot or via API.
        </p>
      )}

      {selectedId && <AutonomousDashboard sessionId={selectedId} />}
    </div>
  );
}