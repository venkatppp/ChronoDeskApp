import { useState, useEffect, useMemo } from "react";
import { Plus, Timeline as TimelineIcon, Pin, PinOff, ExternalLink, FileText, LayoutDashboard, Search, ListTree, ArrowRight, FileCode, TrendingUp, TrendingDown } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import { GlassInput } from "@/components/ui/GlassInput";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { Dialog } from "@/components/ui/Dialog";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { useDashboardData } from "@/features/dashboard/hooks/useDashboardData";
import { BriefingBanner } from "@/features/dashboard/components/BriefingBanner";
import { SmartResumeBanner } from "@/features/dashboard/components/SmartResumeBanner";
import { WorkspaceCard } from "@/features/dashboard/components/WorkspaceCard";
import { RecommendationsPanel } from "@/features/dashboard/components/RecommendationsPanel";
import { RecentActivityFeed } from "@/features/dashboard/components/RecentActivityFeed";
import { DailyBriefing } from "@/features/dashboard/components/DailyBriefing";
import { ContextMemoryCard } from "@/features/dashboard/components/ContextMemoryCard";
import { RelatedWorkCard } from "@/features/dashboard/components/RelatedWorkCard";
import { PredictiveCard } from "@/features/dashboard/components/PredictiveCard";
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

function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

interface QuickAction {
  label: string;
  icon: React.ReactNode;
  shortcut?: string;
  action: () => void;
}

interface QuietMetricProps {
  label: string;
  value: string;
  hint?: React.ReactNode;
  dot?: string;
}

function QuietMetric({ label, value, hint, dot }: QuietMetricProps) {
  return (
    <div className="flex min-w-[7rem] flex-col gap-0.5">
      <span className="flex items-center gap-1.5 text-[11px] font-medium text-(--color-faint-foreground)">
        {dot && <span className={`h-1 w-1 rounded-full ${dot}`} />}
        {label}
      </span>
      <span className="font-(family-name:--font-display) text-lg font-semibold tabular-nums tracking-tight text-(--color-foreground)">
        {value}
      </span>
      {hint && <span className="text-[11px] text-(--color-faint-foreground)">{hint}</span>}
    </div>
  );
}

export function DashboardView() {
  const dashboardData = useDashboardData();
  const { workspaces, briefing, recommendations, workspaceStats, recentActivity, smartResumeSession, dailyBriefing, todaySummary, yesterdaySummary, latestSnapshot, relatedWorkspaces, predictions, isLoading, error, refresh } = dashboardData;
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

  const handleActionSuccess = () => {
    void refresh();
  };

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
      label: "Timeline",
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

  const handleResume = () => {
    if (!resumeWorkspace) return;
    workspaceRepo.switchWorkspace(resumeWorkspace.id).then(() => {
      localStorage.setItem("activeWorkspaceId", resumeWorkspace.id);
      navigate("/timeline");
    }).catch(() => {});
  };

  const hour = new Date().getHours();
  const greeting = hour < 5 ? "Working late" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  const todayLabel = new Date().toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
  const resumeWorkspace = smartResumeSession
    ? workspaces.find((w) => w.id === smartResumeSession.workspaceId) ?? mostRecentWorkspace
    : mostRecentWorkspace;

  const activeStats = mostRecentWorkspace ? workspaceStats[mostRecentWorkspace.id] : undefined;

  const trendHint = (
    current: number,
    previous: number,
    format?: (v: number) => string,
  ): React.ReactNode | undefined => {
    if (previous <= 0) return undefined;
    const pct = ((current - previous) / previous) * 100;
    const delta = current - previous;
    if (Math.abs(pct) < 0.1) return "Same as yesterday";
    const up = pct > 0;
    const detail = format ? format(delta > 0 ? delta : -delta) : `${Math.abs(pct).toFixed(0)}%`;
    return (
      <span className="inline-flex items-center gap-1 text-(--color-faint-foreground)">
        {up ? (
          <TrendingUp className="h-3 w-3 text-(--color-success)" strokeWidth={1.75} />
        ) : (
          <TrendingDown className="h-3 w-3 text-(--color-danger)" strokeWidth={1.75} />
        )}
        vs yesterday {detail}
      </span>
    );
  };

  const surfaceFiles = recentFiles.slice(0, 4);

  const renderFileChips = () => (
    <div className="flex items-center gap-2 overflow-x-auto">
      {surfaceFiles.map((file) => {
        const { name, ext } = parseFilePath(file.title);
        return (
          <button
            key={file.entityId}
            onClick={() => workspaceRepo.openFile(file.title).catch(() => {})}
            className="glass-control group/file flex shrink-0 items-center gap-1.5 rounded-[var(--radius-control)] px-2.5 py-1.5 text-xs text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
          >
            <FileCode className="h-3 w-3 text-(--color-faint-foreground)" strokeWidth={1.75} />
            <span className="max-w-40 truncate font-(family-name:--font-mono) text-[11px] font-medium text-(--color-foreground)">
              {name}
            </span>
            {ext && (
              <Badge variant="neutral" className="px-1 py-px text-[9px] leading-none">
                {ext}
              </Badge>
            )}
          </button>
        );
      })}
    </div>
  );

  return (
    <div className="flex w-full flex-col gap-6 px-6 py-7 lg:px-8">
      {/* Greeting — large, calm, directly on the canvas. No box. */}
      <section>
        <div className="flex flex-col gap-8 xl:flex-row xl:items-end xl:justify-between">
          <div className="max-w-2xl">
            <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
              {todayLabel}
              {resumeWorkspace ? ` — ${resumeWorkspace.name}` : ""}
            </p>
            <h1 className="mt-2 font-(family-name:--font-display) text-5xl font-semibold tracking-[-0.03em] text-(--color-foreground)">
              {greeting}.
            </h1>
            <p className="mt-3 max-w-xl text-[15px] leading-relaxed text-(--color-muted-foreground)">
              {resumeWorkspace
                ? `Pick up where you left off in ${resumeWorkspace.name}, or start something new.`
                : "Everything you were working on, picked up where you left off."}
            </p>
          </div>

          {/* Quick actions — a quiet toolbar, not a row of cards */}
          <div className="flex flex-wrap items-center gap-1.5">
            {quickActions.map((action) => (
              <button
                key={action.label}
                onClick={action.action}
                className="inline-flex h-8 items-center gap-2 rounded-[var(--radius-control)] px-3 text-[13px] font-medium text-(--color-muted-foreground) transition-all duration-100 ease-out hover:bg-(--color-surface-hover) hover:text-(--color-foreground) focus-visible:border-(--color-accent) motion-safe:active:scale-[0.97]"
              >
                {action.icon}
                <span>{action.label}</span>
                {action.shortcut && (
                  <kbd className="ml-0.5 rounded border border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground)">
                    {action.shortcut}
                  </kbd>
                )}
              </button>
            ))}
          </div>
        </div>
      </section>

      {/* Continue Working — the hero glass surface. Level 1 chrome with
          refraction; the environment visibly bends behind it. */}
      {!isLoading && smartResumeSession && !dismissedSmartResume && (
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

      {!isLoading && !(smartResumeSession && !dismissedSmartResume) && resumeWorkspace && (
        <GlassSurface
          material="surface"
          className="rounded-3xl"
          optics={{ scale: -96, blur: 5 }}
        >
          <div className="flex flex-col gap-6 p-6 lg:flex-row lg:items-center lg:justify-between lg:p-7">
            <div className="flex min-w-0 items-center gap-5">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-(--color-accent-muted) ring-1 ring-(--color-accent)/25 shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
                <ArrowRight className="h-5 w-5 text-(--color-accent)" strokeWidth={1.75} />
              </div>
              <div className="min-w-0">
                <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
                  Continue Working
                </p>
                <h2 className="mt-1 truncate font-(family-name:--font-display) text-2xl font-semibold tracking-tight text-(--color-foreground)">
                  {resumeWorkspace.name}
                </h2>
                <p className="mt-1 truncate text-[13px] text-(--color-muted-foreground)">
                  Last active {formatRelativeTime(resumeWorkspace.lastActiveAt)}
                  {activeStats ? ` · health ${Math.round(activeStats.healthScore)}%` : ""}
                  {surfaceFiles.length > 0 ? ` · ${surfaceFiles.length} recent file${surfaceFiles.length === 1 ? "" : "s"}` : ""}
                </p>
              </div>
            </div>
            <div className="flex shrink-0 flex-wrap items-center gap-2.5">
              <Button onClick={handleResume} variant="primary" size="lg">
                <ArrowRight className="h-4 w-4" strokeWidth={1.75} />
                <span className="max-w-44 truncate">Continue in {resumeWorkspace.name}</span>
              </Button>
              <Button onClick={() => navigate("/timeline")} variant="secondary" size="lg">
                <TimelineIcon className="h-4 w-4" strokeWidth={1.75} />
                Open timeline
              </Button>
            </div>
          </div>
          {surfaceFiles.length > 0 && (
            <div className="flex items-center gap-2.5 border-t border-(--color-border-subtle) px-6 py-3.5 lg:px-7">
              <span className="shrink-0 text-[11px] text-(--color-faint-foreground)">
                Recently in {resumeWorkspace.name}
              </span>
              {renderFileChips()}
            </div>
          )}
        </GlassSurface>
      )}

      {/* Workspaces — moved upward to occupy the empty central area */}
      <section className="flex flex-1 flex-col">
        <div className="mb-2 flex items-center justify-between">
          <h2 className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.12em] text-(--color-faint-foreground)">
            <LayoutDashboard className="h-3.5 w-3.5" strokeWidth={1.75} />
            Workspaces
          </h2>
          <span className="text-xs text-(--color-faint-foreground)">
            {workspaces.length} active
          </span>
        </div>

        {isLoading &&
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {[0, 1, 2, 3].map((i) => (
              <div key={i} className="h-32 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
            ))}
          </div>
        }

        {!isLoading && workspaces.length === 0 && (
          <Card className="px-6 py-10 text-center">
            <p className="text-sm text-(--color-muted-foreground)">
              No active workspaces yet. Create one, or watch a folder from Settings once file watching is configured.
            </p>
          </Card>
        )}

        {!isLoading && workspaces.length > 0 && (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {[...pinnedWorkspaces, ...unpinnedWorkspaces].map((w) => (
              <div key={w.id} className="group relative">
                <WorkspaceCard
                  workspace={w}
                  stats={workspaceStats[w.id]}
                  onOpen={async (ws) => {
                    try {
                      await workspaceRepo.switchWorkspace(ws.id);
                      localStorage.setItem("activeWorkspaceId", ws.id);
                      navigate("/timeline");
                    } catch {
                      /* no-op */
                    }
                  }}
                />
                <button
                  onClick={() => togglePin(w.id)}
                  className="absolute right-2 top-2 z-10 rounded p-1.5 text-(--color-faint-foreground) opacity-0 transition-opacity hover:text-(--color-accent) group-hover:opacity-100"
                  aria-label={pinnedIds.has(w.id) ? "Unpin workspace" : "Pin workspace"}
                >
                  {pinnedIds.has(w.id) ? <PinOff className="h-3.5 w-3.5" strokeWidth={1.75} /> : <Pin className="h-3.5 w-3.5" strokeWidth={1.75} />}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      {!isLoading && !(smartResumeSession && !dismissedSmartResume) && !resumeWorkspace && (
        <GlassSurface material="surface" className="rounded-3xl">
          <div className="flex flex-col gap-6 p-7 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex min-w-0 items-center gap-5">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-(--color-accent-muted) ring-1 ring-(--color-accent)/25 shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
                <Plus className="h-5 w-5 text-(--color-accent)" strokeWidth={1.75} />
              </div>
              <div className="min-w-0">
                <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
                  Start something new
                </p>
                <h2 className="mt-1 font-(family-name:--font-display) text-2xl font-semibold tracking-tight text-(--color-foreground)">
                  Create your first workspace
                </h2>
                <p className="mt-1 text-[13px] text-(--color-muted-foreground)">
                  ChronoDesk watches your folders and builds a timeline, a knowledge graph, and memory around them.
                </p>
              </div>
            </div>
            <Button onClick={() => setShowCreateDialog(true)} variant="primary" size="lg" disabled={isCreating}>
              <Plus className="h-4 w-4" strokeWidth={1.75} />
              New workspace
            </Button>
          </div>
        </GlassSurface>
      )}

      {(error || createError) && (
        <div className="flex items-center gap-2.5 rounded-[var(--radius-card)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-4 py-3 text-sm text-(--color-danger)">
          <span>{error ?? createError}</span>
        </div>
      )}

      {/* UPPER CONTENT HIERARCHY — daily context on the left, the two
          intelligence surfaces (Predictive Intelligence + Priority Queue)
          as floating chrome on the right, well above the fold. */}
      <div className="grid grid-cols-1 items-start gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(0,23rem)]">
        <div className="flex min-w-0 flex-col gap-6">
          {/* Today at a glance — compact activity summary, quiet metrics on
              the canvas. No boxes. */}
          {todaySummary && !isLoading && (
            <div className="flex flex-wrap items-stretch gap-x-6 gap-y-4 border-t border-(--color-border-subtle) pt-5">
              <QuietMetric
                label="Focus time"
                value={formatDuration(todaySummary.totalDurationSeconds)}
                dot="bg-(--color-accent)"
                hint={yesterdaySummary ? trendHint(todaySummary.totalDurationSeconds, yesterdaySummary.totalDurationSeconds, formatDuration) : undefined}
              />
              <div className="w-px bg-(--color-border-subtle)" aria-hidden="true" />
              <QuietMetric
                label="Files touched"
                value={String(todaySummary.fileCount)}
                dot="bg-(--color-success)"
                hint={yesterdaySummary ? trendHint(todaySummary.fileCount, yesterdaySummary.fileCount) : undefined}
              />
              <div className="w-px bg-(--color-border-subtle)" aria-hidden="true" />
              <QuietMetric
                label="Edits"
                value={String(todaySummary.editCount)}
                dot="bg-(--color-amber)"
                hint={yesterdaySummary ? trendHint(todaySummary.editCount, yesterdaySummary.editCount) : undefined}
              />
              {activeStats && (
                <>
                  <div className="w-px bg-(--color-border-subtle)" aria-hidden="true" />
                  <div className="flex min-w-[7rem] flex-col gap-0.5">
                    <span className="text-[11px] font-medium text-(--color-faint-foreground)">Workspace health</span>
                    <div className="flex items-center gap-2">
                      <ProgressRing value={activeStats.healthScore} size={28} strokeWidth={3} />
                      <span className="text-[13px] font-medium text-(--color-foreground)">
                        {activeStats.healthScore >= 70 ? "Good" : activeStats.healthScore >= 40 ? "Fair" : "Low"}
                      </span>
                    </div>
                    <span className="text-[11px] text-(--color-faint-foreground)">
                      {formatRelativeTime(activeStats.lastActivity)}
                    </span>
                  </div>
                </>
              )}
            </div>
          )}

          {(!smartResumeSession || dismissedSmartResume) && (
            <BriefingBanner briefing={briefing} isLoading={isLoading} />
          )}

          {dailyBriefing && !isLoading && (
            <DailyBriefing briefing={dailyBriefing} />
          )}
        </div>

        <div className="flex min-w-0 flex-col gap-4">
          <PredictiveCard predictions={predictions} isLoading={isLoading} />
          <RecommendationsPanel recommendations={recommendations} isLoading={isLoading} onActionSuccess={handleActionSuccess} />
        </div>
      </div>

      <div className="grid grid-cols-1 items-start gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="flex min-w-0 flex-col gap-6">
          {recentFiles.length > 0 && (
            <Card className="overflow-hidden">
              <div className="border-b border-(--color-border-subtle) px-5 py-3.5">
                <h2 className="flex items-center gap-2 text-[13px] font-semibold text-(--color-foreground)">
                  <FileText className="h-3.5 w-3.5 text-(--color-muted-foreground)" strokeWidth={1.75} />
                  Recently edited files
                </h2>
              </div>
              <div className="flex flex-col gap-px p-2">
                {recentFiles.map((file) => {
                  const { name, folder, ext } = parseFilePath(file.title);
                  return (
                    <button
                      key={file.entityId}
                      onClick={() => workspaceRepo.openFile(file.title).catch(() => {})}
                      className="group/recent flex items-center gap-3 rounded-[var(--radius-control)] px-2.5 py-2 text-sm text-(--color-foreground) transition-colors duration-200 hover:bg-(--color-surface-hover)"
                    >
                      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-(--color-surface-hover)">
                        <FileCode className="h-3.5 w-3.5 text-(--color-muted-foreground)" strokeWidth={1.75} />
                      </div>
                      <div className="flex min-w-0 flex-1 flex-col items-start">
                        <div className="flex items-center gap-2">
                          <span className="truncate font-(family-name:--font-mono) text-xs font-medium">{name}</span>
                          {ext && (
                            <Badge variant="neutral" className="shrink-0 px-1 py-px text-[9px] leading-none">
                              {ext}
                            </Badge>
                          )}
                        </div>
                        {folder && (
                          <span className="truncate text-[11px] text-(--color-faint-foreground)">{folder}</span>
                        )}
                      </div>
                      <ExternalLink className="h-3 w-3 shrink-0 text-(--color-faint-foreground) opacity-0 transition-opacity group-hover/recent:opacity-60" strokeWidth={1.75} />
                    </button>
                  );
                })}
              </div>
            </Card>
          )}

          <RecentActivityFeed events={recentActivity} isLoading={isLoading} />
        </div>

        <div className="flex flex-col gap-4">
          <ContextMemoryCard snapshot={latestSnapshot} isLoading={isLoading} />
          <RelatedWorkCard relatedWorkspaces={relatedWorkspaces} isLoading={isLoading} />
        </div>
      </div>

      <Dialog
        open={showCreateDialog}
        onClose={closeCreateDialog}
        title="New Workspace"
        description="Enter a name for your new workspace."
        footer={
          <>
            <Button variant="ghost" onClick={closeCreateDialog} disabled={isCreating}>
              Cancel
            </Button>
            <Button onClick={handleCreateWorkspace} disabled={isCreating || !workspaceName.trim()}>
              {isCreating ? "Creating…" : "Create"}
            </Button>
          </>
        }
      >
        <GlassInput
          autoFocus
          size="md"
          onKeyDown={(e) => {
            if (e.key === "Enter") void handleCreateWorkspace();
          }}
          placeholder="Workspace name"
          value={workspaceName}
          onChange={(e) => setWorkspaceName(e.target.value)}
        />
      </Dialog>
    </div>
  );
}
