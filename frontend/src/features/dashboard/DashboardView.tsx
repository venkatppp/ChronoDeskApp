import { useState, useEffect, useMemo } from "react";
import { Plus, Files, Timeline as TimelineIcon, Activity, Pin, PinOff, ExternalLink, Clock, FileText, LayoutDashboard, Search, Sparkles, ListTree, ArrowRight, FileCode } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { Badge } from "@/components/ui/Badge";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { useDashboardData } from "@/features/dashboard/hooks/useDashboardData";
import { BriefingBanner } from "@/features/dashboard/components/BriefingBanner";
import { SmartResumeBanner } from "@/features/dashboard/components/SmartResumeBanner";
import { WorkspaceCard } from "@/features/dashboard/components/WorkspaceCard";
import { RecommendationsPanel } from "@/features/dashboard/components/RecommendationsPanel";
import { RecentActivityFeed } from "@/features/dashboard/components/RecentActivityFeed";
import { DailyBriefing } from "@/features/dashboard/components/DailyBriefing";
import { ActivitySummary } from "@/features/dashboard/components/ActivitySummary";
import { TrendIndicator } from "@/features/dashboard/components/TrendIndicator";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import { getSearchRepository } from "@/services/searchRepository";
import { useNavigate } from "react-router-dom";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import type { SearchResult } from "@/types/search";

const PINNED_KEY = "chronodesk:pinned-workspaces";

function loadPinned(): Set<string> {
  try {
    const raw = localStorage.getItem(PINNED_KEY);
    return new Set(raw ? JSON.parse(raw) : []);
  } catch {
    return new Set();
  }
}

function savePinned(pinned: Set<string>) {
  localStorage.setItem(PINNED_KEY, JSON.stringify([...pinned]));
}

function parseFilePath(path: string): { name: string; folder: string; ext: string } {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const parts = path.split(/[/\\]/);
  const name = parts.pop() ?? path;
  const folder = parts.join("/");
  return { name, folder, ext };
}

interface QuickAction {
  label: string;
  icon: React.ReactNode;
  shortcut?: string;
  action: () => void;
}

export function DashboardView() {
  const { workspaces, briefing, recommendations, workspaceStats, recentActivity, smartResumeSession, dailyBriefing, todaySummary, yesterdaySummary, isLoading, error } = useDashboardData();
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [workspaceName, setWorkspaceName] = useState("");
  const [pinnedIds, setPinnedIds] = useState<Set<string>>(loadPinned);
  const [recentFiles, setRecentFiles] = useState<SearchResult[]>([]);
  const [dismissedSmartResume, setDismissedSmartResume] = useState(false);

  const workspaceRepo = getWorkspaceRepository();
  const searchRepo = getSearchRepository();
  const navigate = useNavigate();

  useEffect(() => {
    if (workspaces.length === 0) return;
    const mostRecent = [...workspaces].sort(
      (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
    )[0];
    searchRepo.getRecentFiles(mostRecent.id, 8).then(setRecentFiles).catch(() => {});
  }, [workspaces, searchRepo]);

  function togglePin(id: string) {
    setPinnedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      savePinned(next);
      return next;
    });
  }

  function closeCreateDialog() {
    setShowCreateDialog(false);
    setWorkspaceName("");
    setCreateError(null);
  }

  async function handleCreateWorkspace() {
    const name = workspaceName.trim();
    if (!name) return;
    setIsCreating(true);
    setCreateError(null);
    try {
      await workspaceRepo.createWorkspace({ name });
      closeCreateDialog();
    } catch (err) {
      setCreateError(err instanceof Error ? err.message : "Failed to create workspace.");
    } finally {
      setIsCreating(false);
    }
  }

  const pinnedWorkspaces = workspaces.filter((w) => pinnedIds.has(w.id));
  const unpinnedWorkspaces = workspaces.filter((w) => !pinnedIds.has(w.id));

  const mostRecentWorkspace = useMemo(() => {
    return [...workspaces].sort(
      (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
    )[0] ?? null;
  }, [workspaces]);

  const quickActions: QuickAction[] = [
    {
      label: "Resume workspace",
      icon: <ArrowRight className="h-4 w-4" strokeWidth={1.75} />,
      shortcut: "R",
      action: () => {
        if (mostRecentWorkspace) {
          workspaceRepo.switchWorkspace(mostRecentWorkspace.id).then(() => {
            localStorage.setItem("activeWorkspaceId", mostRecentWorkspace.id);
            navigate("/timeline");
          }).catch(() => {});
        }
      },
    },
    {
      label: "Open Timeline",
      icon: <TimelineIcon className="h-4 w-4" strokeWidth={1.75} />,
      shortcut: "T",
      action: () => navigate("/timeline"),
    },
    {
      label: "Knowledge Graph",
      icon: <ListTree className="h-4 w-4" strokeWidth={1.75} />,
      shortcut: "G",
      action: () => navigate("/graph"),
    },
    {
      label: "Search files",
      icon: <Search className="h-4 w-4" strokeWidth={1.75} />,
      shortcut: "/",
      action: () => navigate("/search"),
    },
    {
      label: "New workspace",
      icon: <Plus className="h-4 w-4" strokeWidth={1.75} />,
      shortcut: "N",
      action: () => setShowCreateDialog(true),
    },
  ];

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-(family-name:--font-display) text-xl font-bold">Dashboard</h1>
          <p className="text-sm text-(--color-muted-foreground)">
            Everything you were working on, picked up where you left off.
          </p>
        </div>
        <Button onClick={() => setShowCreateDialog(true)} disabled={isCreating}>
          <Plus className="h-4 w-4" />
          New workspace
        </Button>
      </div>

      {(error || createError) && (
        <div className="flex items-center gap-2.5 rounded-[var(--radius-card)] border border-(--color-danger)/40 bg-(--color-danger)/10 px-4 py-3 text-sm text-(--color-danger)">
          <span>{error ?? createError}</span>
        </div>
      )}

      {smartResumeSession && !dismissedSmartResume && !isLoading && (
        <SmartResumeBanner
          session={smartResumeSession}
          onResume={() => {
            workspaceRepo.switchWorkspace(smartResumeSession.workspaceId).then(() => {
              localStorage.setItem("activeWorkspaceId", smartResumeSession.workspaceId);
              navigate("/timeline");
            }).catch(() => {});
          }}
          onDismiss={() => setDismissedSmartResume(true)}
        />
      )}

      {(!smartResumeSession || dismissedSmartResume) && (
        <BriefingBanner briefing={briefing} isLoading={isLoading} />
      )}

      {dailyBriefing && !isLoading && (
        <DailyBriefing briefing={dailyBriefing} />
      )}

      {todaySummary && yesterdaySummary && !isLoading && (
        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <div>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Today
            </h3>
            <ActivitySummary
              summary={{
                timeRange: "Today",
                durationSeconds: todaySummary.totalDurationSeconds,
                sessionCount: todaySummary.sessionCount,
                workspaceCount: todaySummary.workspaceCount,
                fileCount: todaySummary.fileCount,
                editCount: todaySummary.editCount,
                commitCount: todaySummary.commitCount,
                primaryLanguage: todaySummary.languages[0]?.language,
              }}
            />
          </div>
          <div>
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Yesterday
            </h3>
            <ActivitySummary
              summary={{
                timeRange: "Yesterday",
                durationSeconds: yesterdaySummary.totalDurationSeconds,
                sessionCount: yesterdaySummary.sessionCount,
                workspaceCount: yesterdaySummary.workspaceCount,
                fileCount: yesterdaySummary.fileCount,
                editCount: yesterdaySummary.editCount,
                commitCount: yesterdaySummary.commitCount,
                primaryLanguage: yesterdaySummary.languages[0]?.language,
              }}
            />
          </div>
        </div>
      )}

      {todaySummary && yesterdaySummary && !isLoading && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <TrendIndicator
            label="Duration"
            current={todaySummary.totalDurationSeconds}
            previous={yesterdaySummary.totalDurationSeconds}
            format={(val) => {
              const hours = Math.floor(val / 3600);
              const minutes = Math.floor((val % 3600) / 60);
              return hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
            }}
          />
          <TrendIndicator
            label="Files"
            current={todaySummary.fileCount}
            previous={yesterdaySummary.fileCount}
          />
          <TrendIndicator
            label="Edits"
            current={todaySummary.editCount}
            previous={yesterdaySummary.editCount}
          />
          <TrendIndicator
            label="Commits"
            current={todaySummary.commitCount}
            previous={yesterdaySummary.commitCount}
          />
        </div>
      )}

      {mostRecentWorkspace && workspaceStats[mostRecentWorkspace.id] && !isLoading && (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Card className="flex items-center gap-3 p-4">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
              <Files className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
            </div>
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Files</p>
              <p className="text-lg font-bold text-(--color-foreground)">{workspaceStats[mostRecentWorkspace.id].fileCount}</p>
            </div>
          </Card>
          <Card className="flex items-center gap-3 p-4">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
              <Activity className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
            </div>
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Events</p>
              <p className="text-lg font-bold text-(--color-foreground)">{workspaceStats[mostRecentWorkspace.id].timelineEventCount}</p>
            </div>
          </Card>
          <Card className="flex items-center gap-3 p-4">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg">
              <ProgressRing value={workspaceStats[mostRecentWorkspace.id].healthScore} size={36} strokeWidth={3} />
            </div>
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Health</p>
              <div className="flex items-center gap-2">
                <p className="text-lg font-bold text-(--color-foreground)">{workspaceStats[mostRecentWorkspace.id].healthScore}%</p>
                <span
                  className={`inline-flex items-center gap-0.5 text-[10px] font-medium ${
                    workspaceStats[mostRecentWorkspace.id].healthScore >= 70
                      ? "text-(--color-success)"
                      : workspaceStats[mostRecentWorkspace.id].healthScore >= 40
                        ? "text-(--color-warning)"
                        : "text-(--color-danger)"
                  }`}
                >
                  {workspaceStats[mostRecentWorkspace.id].healthScore >= 70 ? "Good" : workspaceStats[mostRecentWorkspace.id].healthScore >= 40 ? "Fair" : "Low"}
                </span>
              </div>
              <p className="text-xs text-(--color-faint-foreground)">{formatRelativeTime(workspaceStats[mostRecentWorkspace.id].lastActivity)}</p>
            </div>
          </Card>
          <Card className="flex items-center gap-3 p-4">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-(--color-warning)/20">
              <Sparkles className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />
            </div>
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                {recommendations.length > 0 ? "Items pending" : "All clear"}
              </p>
              <p className="text-lg font-bold text-(--color-foreground)">
                {recommendations.length > 0
                  ? `${recommendations.length} item${recommendations.length > 1 ? "s" : ""}`
                  : "No issues"}
              </p>
            </div>
          </Card>
        </div>
      )}

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-[1fr_300px]">
        <div className="flex flex-col gap-6">
          <section>
            <h2 className="mb-3 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              <LayoutDashboard className="h-3.5 w-3.5" strokeWidth={1.75} />
              Quick actions
            </h2>
            <div className="flex flex-wrap gap-2">
              {quickActions.map((action) => (
                <button
                  key={action.label}
                  onClick={action.action}
                  className="inline-flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) px-3.5 py-2 text-sm text-(--color-foreground) transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:border-(--color-accent) hover:bg-(--color-surface-hover) hover:shadow-[0_2px_6px_rgba(0,0,0,0.3)] focus-visible:border-(--color-accent) active:scale-[0.97]"
                >
                  {action.icon}
                  <span>{action.label}</span>
                  {action.shortcut && (
                    <kbd className="ml-1 rounded border border-(--color-border-subtle) bg-(--color-background) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground)">
                      {action.shortcut}
                    </kbd>
                  )}
                </button>
              ))}
            </div>
          </section>

          {recentFiles.length > 0 && (
            <section>
              <h2 className="mb-3 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                <FileText className="h-3.5 w-3.5" strokeWidth={1.75} />
                Recently edited files
              </h2>
              <div className="flex flex-col gap-px">
                {recentFiles.map((file) => {
                  const { name, folder, ext } = parseFilePath(file.title);
                  return (
                    <button
                      key={file.entityId}
                      onClick={() => workspaceRepo.openFile(file.title).catch(() => {})}
                      className="group/recent flex items-center gap-2.5 rounded-[var(--radius-control)] px-2.5 py-2 text-sm text-(--color-foreground) transition-colors duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover)"
                    >
                      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-(--color-surface-hover)">
                        <FileCode className="h-3.5 w-3.5 text-(--color-muted-foreground)" strokeWidth={1.75} />
                      </div>
                      <div className="flex min-w-0 flex-1 flex-col items-start gap-0">
                        <div className="flex items-center gap-2">
                          <span className="truncate font-(family-name:--font-mono) text-xs font-medium">
                            {name}
                          </span>
                          {ext && (
                            <Badge variant="neutral" className="shrink-0 px-1 py-0 text-[9px] leading-none">
                              {ext}
                            </Badge>
                          )}
                        </div>
                        {folder && (
                          <span className="truncate text-[11px] text-(--color-faint-foreground)">
                            {folder}
                          </span>
                        )}
                      </div>
                      <ExternalLink className="h-3 w-3 shrink-0 text-(--color-faint-foreground) opacity-0 transition-opacity group-hover/recent:opacity-60" strokeWidth={1.75} />
                    </button>
                  );
                })}
              </div>
            </section>
          )}

          {pinnedWorkspaces.length > 0 && (
            <section>
              <h2 className="mb-3 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                <Pin className="h-3.5 w-3.5" strokeWidth={1.75} />
                Pinned
              </h2>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {pinnedWorkspaces.map((w) => (
                  <div key={w.id} className="group relative">
                    <WorkspaceCard workspace={w} stats={workspaceStats[w.id]} />
                    <button
                      onClick={() => togglePin(w.id)}
                      className="absolute right-2 top-2 z-10 rounded p-2 text-(--color-faint-foreground) opacity-0 transition-opacity hover:text-(--color-accent) group-hover:opacity-100"
                      aria-label="Unpin workspace"
                    >
                      <PinOff className="h-3.5 w-3.5" strokeWidth={1.75} />
                    </button>
                  </div>
                ))}
              </div>
            </section>
          )}

          <section>
            <h2 className="mb-3 flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              <Clock className="h-3.5 w-3.5" strokeWidth={1.75} />
              Active workspaces
            </h2>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {isLoading &&
                [0, 1, 2].map((i) => (
                  <div key={i} className="h-32 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
                ))}
              {!isLoading && workspaces.length === 0 && (
                <p className="col-span-full py-2 text-sm text-(--color-faint-foreground)">
                  No active workspaces yet. Create one, or watch a folder from Settings once file watching is configured.
                </p>
              )}
              {!isLoading &&
                unpinnedWorkspaces.map((workspace) => (
                  <div key={workspace.id} className="group relative">
                    <WorkspaceCard workspace={workspace} />
                    <button
                      onClick={() => togglePin(workspace.id)}
                      className="absolute right-2 top-2 z-10 rounded p-2 text-(--color-faint-foreground) opacity-0 transition-opacity hover:text-(--color-accent) group-hover:opacity-100"
                      aria-label="Pin workspace"
                    >
                      <Pin className="h-3.5 w-3.5" strokeWidth={1.75} />
                    </button>
                  </div>
                ))}
            </div>
          </section>
        </div>

        <div className="flex flex-col gap-6">
          <RecommendationsPanel recommendations={recommendations} isLoading={isLoading} />
          <RecentActivityFeed events={recentActivity} isLoading={isLoading} />
        </div>
      </div>

      {showCreateDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
          role="dialog"
          aria-modal="true"
          aria-labelledby="dashboard-create-dialog-title"
          onClick={closeCreateDialog}
          tabIndex={-1}
          onKeyDown={(e) => {
            if (e.key === "Escape") closeCreateDialog();
          }}
        >
          <div className="w-full max-w-md animate-scale-in rounded-xl bg-(--color-surface) p-6" onClick={(e) => e.stopPropagation()}>
            <h2 id="dashboard-create-dialog-title" className="mb-4 text-xl font-bold text-(--color-foreground)">Create Workspace</h2>
            <input
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleCreateWorkspace();
                else if (e.key === "Escape") closeCreateDialog();
              }}
              className="mb-4 w-full rounded border border-(--color-border) bg-(--color-surface-hover) p-2 text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:border-(--color-accent) focus:outline-none"
              placeholder="Workspace name"
              value={workspaceName}
              onChange={(e) => setWorkspaceName(e.target.value)}
            />
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={closeCreateDialog} disabled={isCreating}>
                Cancel
              </Button>
              <Button onClick={handleCreateWorkspace} disabled={isCreating || !workspaceName.trim()}>
                {isCreating ? "Creating\u2026" : "Create"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
