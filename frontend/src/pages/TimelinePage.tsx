import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Clock, FilePlus, FileEdit, Trash2, ArrowRightLeft, Eye, Camera, RefreshCw, Search, ChevronDown, ExternalLink, MoveRight, Code, FileJson, FileType, FileImage, PanelTopDashed, AlignJustify, FolderOpen, Copy, ChevronUp } from "lucide-react";
import { getTimelineRepository } from "@/services/timelineRepository";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { TimelineEvent, TimelineEventType } from "@/types/timeline";
import type { Workspace } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

type Density = "compact" | "comfortable";

const EVENT_ICONS: Record<TimelineEventType, React.ReactNode> = {
  create: <FilePlus className="h-4 w-4" />,
  open: <Eye className="h-4 w-4" />,
  close: <Clock className="h-4 w-4" />,
  edit: <FileEdit className="h-4 w-4" />,
  move: <MoveRight className="h-4 w-4" />,
  delete: <Trash2 className="h-4 w-4" />,
  commit: <Clock className="h-4 w-4" />,
  visit: <Eye className="h-4 w-4" />,
  screenshot: <Camera className="h-4 w-4" />,
  workspace_switch: <ArrowRightLeft className="h-4 w-4" />,
};

const EVENT_COLORS: Record<TimelineEventType, string> = {
  create: "bg-(--color-success)/10 text-(--color-success)",
  edit: "bg-(--color-warning)/10 text-(--color-warning)",
  delete: "bg-(--color-danger)/10 text-(--color-danger)",
  move: "bg-(--color-accent)/10 text-(--color-accent)",
  workspace_switch: "bg-(--color-accent)/10 text-(--color-accent)",
  open: "bg-(--color-surface-hover) text-(--color-muted-foreground)",
  close: "bg-(--color-surface-hover) text-(--color-muted-foreground)",
  commit: "bg-(--color-accent-muted) text-(--color-accent)",
  visit: "bg-(--color-surface-hover) text-(--color-muted-foreground)",
  screenshot: "bg-(--color-warning)/10 text-(--color-warning)",
};

const EVENT_LABELS: Record<TimelineEventType, string> = {
  create: "Created", edit: "Edited", delete: "Deleted", move: "Moved",
  workspace_switch: "Switched workspace", open: "Opened", close: "Closed",
  commit: "Committed", visit: "Visited", screenshot: "Captured",
};

const EXT_ICONS: Record<string, React.ReactNode> = {
  ts: <Code className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />,
  tsx: <Code className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />,
  js: <FileJson className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />,
  jsx: <Code className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />,
  rs: <Code className="h-4 w-4 text-(--color-danger)" strokeWidth={1.75} />,
  py: <FileType className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />,
  css: <FileType className="h-4 w-4 text-(--color-accent-muted)" strokeWidth={1.75} />,
  html: <FileType className="h-4 w-4 text-(--color-danger)" strokeWidth={1.75} />,
  json: <FileJson className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />,
  md: <FileType className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />,
  png: <FileImage className="h-4 w-4 text-(--color-danger)" strokeWidth={1.75} />,
  svg: <FileImage className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />,
};

function extIcon(path: string | null): React.ReactNode | null {
  if (!path) return null;
  const ext = path.split(".").pop()?.toLowerCase();
  return ext ? EXT_ICONS[ext] ?? null : null;
}

function eventFilePath(event: TimelineEvent): string | null {
  const path = event.metadata?.path;
  if (typeof path === "string") return path;
  const from = event.metadata?.from;
  if (typeof from === "string") return from;
  return event.fileId;
}

function eventFileName(event: TimelineEvent): string | null {
  const path = eventFilePath(event);
  if (!path) return null;
  return path.split(/[/\\]/).pop() ?? path;
}

function groupByDay(events: TimelineEvent[]): Map<string, TimelineEvent[]> {
  const groups = new Map<string, TimelineEvent[]>();
  for (const event of events) {
    const day = event.occurredAt.slice(0, 10);
    if (!groups.has(day)) groups.set(day, []);
    groups.get(day)!.push(event);
  }
  return groups;
}

function collapseEdits(events: TimelineEvent[]): (TimelineEvent & { collapsedCount?: number })[] {
  if (events.length === 0) return events;
  const result: (TimelineEvent & { collapsedCount?: number })[] = [];
  let buffer: TimelineEvent[] = [];
  for (const event of events) {
    if (event.eventType === "edit") {
      const last = buffer[buffer.length - 1];
      if (last && eventFilePath(event) === eventFilePath(last) && Math.abs(new Date(event.occurredAt).getTime() - new Date(last.occurredAt).getTime()) < 300000) {
        buffer.push(event);
        continue;
      }
      if (buffer.length > 1) result.push({ ...buffer[0], collapsedCount: buffer.length });
      else if (buffer.length === 1) result.push(buffer[0]);
      buffer = [event];
    } else {
      if (buffer.length > 1) result.push({ ...buffer[0], collapsedCount: buffer.length });
      else if (buffer.length === 1) result.push(buffer[0]);
      buffer = [];
      result.push(event);
    }
  }
  if (buffer.length > 1) result.push({ ...buffer[0], collapsedCount: buffer.length });
  else if (buffer.length === 1) result.push(buffer[0]);
  return result;
}

function formatDayHeader(iso: string): string {
  const date = new Date(iso + "T00:00:00");
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "Today";
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  return date.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
}

interface ContextMenuState {
  x: number;
  y: number;
  filePath: string | null;
}

export function TimelinePage() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>("");
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [filterType, setFilterType] = useState<TimelineEventType | "all">("all");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedCollapsed, setExpandedCollapsed] = useState<Set<string>>(new Set());
  const [density, setDensity] = useState<Density>("comfortable");
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [allCollapsed, setAllCollapsed] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const timelineRepo = getTimelineRepository();
  const workspaceRepo = getWorkspaceRepository();

  useEffect(() => {
    workspaceRepo.listActiveWorkspaces().then((ws) => {
      setWorkspaces(ws);
      if (ws.length > 0) {
        const storedId = localStorage.getItem("activeWorkspaceId");
        if (storedId && ws.some((w) => w.id === storedId)) setSelectedWorkspaceId(storedId);
        else setSelectedWorkspaceId(ws[0].id);
      }
    });
  }, [workspaceRepo]);

  const fetchEvents = useCallback(async () => {
    if (!selectedWorkspaceId) return;
    setIsLoading(true);
    setError(null);
    try {
      setEvents(await timelineRepo.listWorkspaceTimeline(selectedWorkspaceId, 100));
    } catch (err) {
      console.error("Failed to fetch timeline events:", err);
      setError("Failed to load timeline.");
    } finally {
      setIsLoading(false);
    }
  }, [timelineRepo, selectedWorkspaceId]);

  useEffect(() => { fetchEvents(); }, [fetchEvents]);

  const filtered = useMemo(() => {
    let result = events.filter((e) => filterType === "all" || e.eventType === filterType);
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter((e) => {
        const name = eventFileName(e)?.toLowerCase() ?? "";
        const path = eventFilePath(e)?.toLowerCase() ?? "";
        return name.includes(q) || path.includes(q);
      });
    }
    return result;
  }, [events, filterType, searchQuery]);

  const grouped = useMemo(() => {
    const dayGroups = groupByDay(filtered);
    const sorted = [...dayGroups.entries()].sort(([a], [b]) => b.localeCompare(a));
    const result: Map<string, (TimelineEvent & { collapsedCount?: number })[]> = new Map();
    for (const [day, dayEvents] of sorted) result.set(day, collapseEdits(dayEvents));
    return result;
  }, [filtered]);

  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const e of events) counts[e.eventType] = (counts[e.eventType] || 0) + 1;
    return counts;
  }, [events]);

  const relevantTypes = useMemo(() => {
    return (["all", "create", "edit", "delete", "move", "workspace_switch"] as const).filter(
      (t) => t === "all" || typeCounts[t] > 0,
    );
  }, [typeCounts]);

  const vPadding = density === "compact" ? "py-2" : "py-3";
  const iconSize = density === "compact" ? "h-6 w-6" : "h-7 w-7";

  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
      if (e.key === "c" || e.key === "C") {
        setAllCollapsed((p) => !p);
      }
    };
    window.addEventListener("click", handleClick);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("click", handleClick);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent, filePathEl: string | null) => {
    e.preventDefault();
    if (filePathEl) {
      setContextMenu({ x: e.clientX, y: e.clientY, filePath: filePathEl });
    }
  }, []);

  return (
    <div ref={containerRef} className="mx-auto max-w-4xl px-6 py-10">
      <div className="mb-8 flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div>
          <h1 className="font-(family-name:--font-display) text-3xl font-bold tracking-tight">Timeline</h1>
          <p className="mt-1 text-(--color-muted-foreground)">Every edit, creation, and action across your workspaces.</p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) p-0.5">
            <button
              onClick={() => setDensity("compact")}
              className={`rounded-[calc(var(--radius-control)-2px)] p-1.5 transition-colors ${density === "compact" ? "bg-(--color-accent) text-(--color-accent-foreground)" : "text-(--color-muted-foreground) hover:text-(--color-foreground)"}`}
              title="Compact view"
            >
              <AlignJustify className="h-3.5 w-3.5" strokeWidth={1.75} />
            </button>
            <button
              onClick={() => setDensity("comfortable")}
              className={`rounded-[calc(var(--radius-control)-2px)] p-1.5 transition-colors ${density === "comfortable" ? "bg-(--color-accent) text-(--color-accent-foreground)" : "text-(--color-muted-foreground) hover:text-(--color-foreground)"}`}
              title="Comfortable view"
            >
              <PanelTopDashed className="h-3.5 w-3.5" strokeWidth={1.75} />
            </button>
          </div>
          <div className="flex rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) p-0.5">
            <button
              onClick={() => setAllCollapsed(true)}
              className={`rounded-[calc(var(--radius-control)-2px)] p-1.5 transition-colors ${allCollapsed ? "bg-(--color-accent) text-(--color-accent-foreground)" : "text-(--color-muted-foreground) hover:text-(--color-foreground)"}`}
              title="Collapse all edits"
            >
              <ChevronUp className="h-3.5 w-3.5" strokeWidth={1.75} />
            </button>
            <button
              onClick={() => setAllCollapsed(false)}
              className={`rounded-[calc(var(--radius-control)-2px)] p-1.5 transition-colors ${!allCollapsed ? "bg-(--color-accent) text-(--color-accent-foreground)" : "text-(--color-muted-foreground) hover:text-(--color-foreground)"}`}
              title="Expand all edits"
            >
              <ChevronDown className="h-3.5 w-3.5" strokeWidth={1.75} />
            </button>
          </div>
          <button
            onClick={fetchEvents}
            className="flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) px-3 py-2 text-sm text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
            title="Refresh"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isLoading ? "animate-spin" : ""}`} strokeWidth={1.75} />
          </button>
        </div>
      </div>

      <div className="mb-6 flex flex-wrap items-center gap-3">
        <div className="relative min-w-[180px] flex-[2]">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-muted-foreground)" strokeWidth={1.75} />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search files..."
            aria-label="Search timeline events by file name"
            className="w-full rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) py-2 pl-9 pr-3 text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:border-(--color-accent) focus:outline-none"
          />
        </div>
        <select
          value={selectedWorkspaceId}
          aria-label="Filter by workspace"
          onChange={async (e) => {
            const id = e.target.value;
            setSelectedWorkspaceId(id);
            localStorage.setItem("activeWorkspaceId", id);
            try { await workspaceRepo.switchWorkspace(id); } catch {}
          }}
          className="rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) px-3 py-2 text-sm text-(--color-foreground) focus:border-(--color-accent) focus:outline-none"
        >
          <option value="" disabled>Workspace</option>
          {workspaces.map((w) => (<option key={w.id} value={w.id}>{w.name}</option>))}
        </select>
        <div className="flex rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) p-0.5">
          {relevantTypes.map((type) => (
            <button
              key={type}
              onClick={() => setFilterType(type as TimelineEventType | "all")}
              className={`rounded-[calc(var(--radius-control)-2px)] px-2.5 py-1.5 text-xs font-medium capitalize whitespace-nowrap transition-all ${
                filterType === type ? "bg-(--color-accent) text-(--color-accent-foreground)" : "text-(--color-muted-foreground) hover:text-(--color-foreground)"
              }`}
            >
              {type === "all" ? `All (${events.length})` : `${type} (${typeCounts[type] ?? 0})`}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-4">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="flex animate-pulse gap-4">
              <div className="h-8 w-8 shrink-0 rounded-full bg-(--color-surface-hover)" />
              <div className="h-16 flex-1 rounded-[var(--radius-card)] bg-(--color-surface)" />
            </div>
          ))}
        </div>
      ) : error ? (
        <div className="rounded-[var(--radius-card)] border border-(--color-danger)/20 bg-(--color-danger)/5 px-6 py-12 text-center">
          <p className="mb-2 font-medium text-(--color-danger)">{error}</p>
          <button onClick={fetchEvents} className="text-sm text-(--color-accent) hover:underline">Try again</button>
        </div>
      ) : filtered.length === 0 ? (
        <div className="rounded-[var(--radius-card)] border-2 border-dashed border-(--color-border) px-6 py-20 text-center">
          <Clock className="mx-auto mb-4 h-12 w-12 text-(--color-faint-foreground)" strokeWidth={1.5} />
          <h3 className="mb-1 font-semibold text-(--color-foreground)">No events found</h3>
          <p className="text-sm text-(--color-muted-foreground)">
            {searchQuery ? "Try a different search term." : "Events will appear here as you work."}
          </p>
        </div>
      ) : (
        <div className="space-y-8">
          {[...grouped.entries()].map(([day, dayEvents]) => (
            <section key={day} className="animate-fade-in">
              <h2 className="sticky top-0 z-10 mb-3 bg-(--color-background)/80 backdrop-blur-sm py-2 font-(family-name:--font-display) text-sm font-semibold uppercase tracking-wider text-(--color-faint-foreground)">
                {formatDayHeader(day)}
                <span className="ml-2 font-mono text-[10px] font-normal text-(--color-border)">
                  {dayEvents.length} event{dayEvents.length !== 1 ? "s" : ""}
                </span>
              </h2>
              <div className="relative space-y-1 pl-8 before:absolute before:bottom-0 before:left-[15px] before:top-0 before:w-px before:bg-(--color-border)/30">
                {dayEvents.map((event) => {
                  const colorClasses = EVENT_COLORS[event.eventType];
                  const fileName = eventFileName(event);
                  const filePath = eventFilePath(event);
                  const isCollapsed = event.collapsedCount !== undefined && event.collapsedCount > 0;
                  const icon = extIcon(filePath);
                  const effectiveExpanded = allCollapsed ? false : expandedCollapsed.has(event.id);
                  const showCollapsed = isCollapsed && effectiveExpanded;

                  return (
                    <div
                      key={event.id}
                      className={`group rounded-[var(--radius-control)] transition-all duration-300 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface) ${vPadding} px-3`}
                      onContextMenu={(e) => handleContextMenu(e, filePath)}
                    >
                      <div className="relative flex gap-3">
                        <div className={`relative z-10 mt-0.5 flex ${iconSize} shrink-0 items-center justify-center rounded-full border-2 border-(--color-background) ${colorClasses}`}>
                          {icon ?? EVENT_ICONS[event.eventType]}
                        </div>
                        <div className="flex min-w-0 flex-1 items-start justify-between gap-2">
                          <div className="min-w-0">
                            <div className="flex items-center gap-2">
                              {fileName ? (
                                <button
                                  onClick={() => { if (filePath) workspaceRepo.openFile(filePath).catch(() => {}); }}
                                  className="group/file flex items-center gap-1 font-(family-name:--font-mono) text-sm font-medium text-(--color-foreground) hover:text-(--color-accent)"
                                >
                                  {fileName}
                                  <ExternalLink className="h-3 w-3 shrink-0 opacity-0 transition-opacity group-hover/file:opacity-60" strokeWidth={1.75} />
                                </button>
                              ) : (
                                <span className="text-sm font-medium text-(--color-muted-foreground)">{EVENT_LABELS[event.eventType]}</span>
                              )}
                              <span className="text-xs text-(--color-muted-foreground)">{EVENT_LABELS[event.eventType]}</span>
                            </div>
                            {filePath && <p className="mt-0.5 truncate text-xs text-(--color-faint-foreground)">{filePath}</p>}
                          </div>
                          <div className="flex shrink-0 items-center gap-2">
                            {isCollapsed && (
                              <button
                                onClick={() => setExpandedCollapsed((prev) => {
                                  const next = new Set(prev);
                                  if (next.has(event.id)) next.delete(event.id); else next.add(event.id);
                                  return next;
                                })}
                                className="flex items-center gap-0.5 rounded bg-(--color-surface-hover) px-1.5 py-0.5 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
                              >
                                <ChevronDown className={`h-3 w-3 transition-transform ${showCollapsed ? "rotate-180" : ""}`} strokeWidth={2} />
                                {event.collapsedCount}
                              </button>
                            )}
                            <span className="whitespace-nowrap text-xs tabular-nums text-(--color-faint-foreground)">{formatRelativeTime(event.occurredAt)}</span>
                          </div>
                        </div>
                      </div>
                      {event.metadata && Object.keys(event.metadata).length > 0 && !event.metadata?.path && (
                        <div className="ml-10 mt-1 flex flex-wrap gap-x-4 gap-y-0.5 text-xs text-(--color-muted-foreground)">
                          {Object.entries(event.metadata).map(([key, value]) => (
                            <span key={key} className="truncate"><span className="font-medium capitalize">{key.replace(/_/g, " ")}:</span> {String(value)}</span>
                          ))}
                        </div>
                      )}
                      {typeof event.metadata?.from === "string" && typeof event.metadata?.to === "string" && (
                        <div className="ml-10 mt-0.5 flex items-center gap-1.5 text-xs text-(--color-muted-foreground)">
                          <MoveRight className="h-3 w-3 shrink-0" strokeWidth={1.75} />
                          <span className="truncate font-(family-name:--font-mono)">{event.metadata?.from as string}</span>
                          <span className="text-(--color-faint-foreground)">→</span>
                          <span className="truncate font-(family-name:--font-mono)">{event.metadata?.to as string}</span>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      )}

      {contextMenu && (
        <div
          className="fixed z-50 w-48 animate-scale-in overflow-hidden rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) py-1 shadow-[0_4px_12px_rgba(0,0,0,0.4),0_2px_6px_rgba(0,0,0,0.2)]"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          role="menu"
          onKeyDown={(e) => { if (e.key === "Escape") setContextMenu(null); }}
        >
          <button
            role="menuitem"
            onClick={() => {
              if (contextMenu.filePath) workspaceRepo.openFile(contextMenu.filePath).catch(() => {});
              setContextMenu(null);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
          >
            <ExternalLink className="h-3.5 w-3.5" strokeWidth={1.75} />
            Open file
          </button>
          <button
            role="menuitem"
            onClick={() => {
              if (contextMenu.filePath) {
                const path = contextMenu.filePath;
                const fullPath = path.startsWith("/") ? path : `${workspaces.find((w) => w.id === selectedWorkspaceId)?.rootPath ?? ""}/${path}`;
                workspaceRepo.openFile(fullPath).catch(() => {});
              }
              setContextMenu(null);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
          >
            <FolderOpen className="h-3.5 w-3.5" strokeWidth={1.75} />
            Reveal in Finder
          </button>
          <button
            role="menuitem"
            onClick={() => {
              if (contextMenu.filePath) {
                navigator.clipboard.writeText(contextMenu.filePath).catch(() => {});
              }
              setContextMenu(null);
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
          >
            <Copy className="h-3.5 w-3.5" strokeWidth={1.75} />
            Copy path
          </button>
        </div>
      )}
    </div>
  );
}
