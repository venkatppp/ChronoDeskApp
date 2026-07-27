import { useState } from "react";
import { Plus, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { useDashboardData } from "@/features/dashboard/hooks/useDashboardData";
import { BriefingBanner } from "@/features/dashboard/components/BriefingBanner";
import { WorkspaceCard } from "@/features/dashboard/components/WorkspaceCard";
import { RecommendationsPanel } from "@/features/dashboard/components/RecommendationsPanel";
import { RecentActivityFeed } from "@/features/dashboard/components/RecentActivityFeed";
import { getWorkspaceRepository } from "@/services/workspaceRepository";

export function DashboardView() {
  const { workspaces, briefing, recommendations, recentActivity, isLoading, error } = useDashboardData();
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // A prompt-based flow rather than a full creation form: Phase 3's
  // scope is wiring the dashboard to real data end-to-end, not building
  // out every workspace-management screen. The workspace list refreshes
  // itself via the `workspace:created` event once this succeeds — no
  // manual reload call needed here.
  async function handleCreateWorkspace() {
    const name = window.prompt("Workspace name");
    if (!name || !name.trim()) return;

    setIsCreating(true);
    setCreateError(null);
    try {
      await getWorkspaceRepository().createWorkspace({ name: name.trim() });
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : "Failed to create workspace.");
    } finally {
      setIsCreating(false);
    }
  }

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-(family-name:--font-display) text-xl font-bold">Dashboard</h1>
          <p className="text-sm text-(--color-muted-foreground)">
            Everything you were working on, picked up where you left off.
          </p>
        </div>
        <Button onClick={handleCreateWorkspace} disabled={isCreating}>
          <Plus className="h-4 w-4" strokeWidth={2} />
          {isCreating ? "Creating…" : "New workspace"}
        </Button>
      </div>

      {(error || createError) && (
        <div className="flex items-center gap-2.5 rounded-[var(--radius-card)] border border-(--color-danger)/40 bg-(--color-danger)/10 px-4 py-3 text-sm text-(--color-danger)">
          <AlertTriangle className="h-4 w-4 shrink-0" strokeWidth={1.75} />
          <span>{error ?? createError}</span>
        </div>
      )}

      <BriefingBanner briefing={briefing} isLoading={isLoading} />

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-[1fr_320px]">
        <section>
          <h2 className="mb-3 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            Active workspaces
          </h2>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {isLoading &&
              [0, 1, 2].map((i) => (
                <div key={i} className="h-32 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
              ))}
            {!isLoading && workspaces.length === 0 && (
              <p className="col-span-full py-2 text-sm text-(--color-faint-foreground)">
                No active workspaces yet. Create one, or watch a folder from Settings once file watching is
                configured.
              </p>
            )}
            {!isLoading && workspaces.map((workspace) => <WorkspaceCard key={workspace.id} workspace={workspace} />)}
          </div>
        </section>

        <div className="flex flex-col gap-6">
          <RecommendationsPanel recommendations={recommendations} isLoading={isLoading} />
          <RecentActivityFeed events={recentActivity} isLoading={isLoading} />
        </div>
      </div>
    </div>
  );
}
