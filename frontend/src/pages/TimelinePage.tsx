import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { Clock, FilePlus, FileEdit, Trash2, ArrowRightLeft, Eye, Camera, RefreshCw, Search, ChevronDown, ExternalLink, MoveRight, Code, FileJson, FileType, FileImage, FolderOpen, Copy, GitCommit, Zap } from "lucide-react";
import { getTimelineRepository } from "@/services/timelineRepository";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { TimelineEvent, TimelineEventType } from "@/types/timeline";
import type { Workspace } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

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
  const flushBuffer = (buffer: TimelineEvent[]) => {
    if (buffer.length > 1) result.push({ ...buffer[0], collapsedCount: buffer.length });
    else if (buffer.length === 1) result.push(buffer[0]);
  };
  let buffer: TimelineEvent[] = [];
  const sameKind = (a: TimelineEvent, b: TimelineEvent) =>
    a.eventType === b.eventType && eventFilePath(a) === eventFilePath(b);
  for (const event of events) {
    const last = buffer[buffer.length - 1];
    if (last && sameKind(event, last)) {
      // Merge identical consecutive events (e.g. a burst of edits to one
      // file, or repeated workspace switches to the same target) into a
      // single annotated row. Edits merge within a 5-minute window;
      // other repeated events merge regardless of spacing so repeated
      // workspace-switch noise can never dominate the timeline.
      if (event.eventType === "edit") {
        const gap = Math.abs(new Date(event.occurredAt).getTime() - new Date(last.occurredAt).getTime());
        if (gap < 300000) {
          buffer.push(event);
          continue;
        }
      } else {
        buffer.push(event);
        continue;
      }
      flushBuffer(buffer);
      buffer = [event];
    } else {
      flushBuffer(buffer);
      buffer = [event];
    }
  }
  flushBuffer(buffer);
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

const SESSION_GAP_MS = 45 * 60 * 1000;

const TIME_FORMATTER = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatTime(date: Date): string {
  return TIME_FORMATTER.format(date);
}

function groupIntoSessions<T extends TimelineEvent>(events: T[]): { start: Date; end: Date; events: T[] }[] {
  const sorted = [...events].sort(
    (a, b) => new Date(a.occurredAt).getTime() - new Date(b.occurredAt).getTime(),
  );
  const sessions: { start: Date; end: Date; events: T[] }[] = [];
  let current: { start: Date; end: Date; events: T[] } | null = null;
  let prevTime: number | null = null;
  for (const event of sorted) {
    const t = new Date(event.occurredAt).getTime();
    if (!current || (prevTime !== null && t - prevTime > SESSION_GAP_MS)) {
      current = { start: new Date(t), end: new Date(t), events: [] };
      sessions.push(current);
    }
    current.events.push(event);
    current.end = new Date(t);
    prevTime = t;
  }
  return sessions;
}

/* ------------------------------------------------------------------ *
 * Session summaries — turn raw event streams into readable work-session
 * cards (time range, duration, edited files, commits, productivity).
 * Repeated events collapse into a single annotated row, so a stream of
 * `workspace_switch` events renders as "Switched workspace ×3".
 * ------------------------------------------------------------------ */

function sessionName(date: Date): string {
  const h = date.getHours();
  if (h < 12) return "Morning Session";
  if (h < 17) return "Afternoon Session";
  if (h < 21) return "Evening Session";
  return "Night Session";
}

function formatDuration(ms: number): string {
  const mins = Math.max(1, Math.round(ms / 60000));
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h${String(m).padStart(2, "0")}m`;
}

function productivityOf(edits: number, commits: number, creates: number): { label: string; tone: string } {
  const score = edits * 2 + commits * 3 + creates;
  if (score >= 14) return { label: "High", tone: "text-(--color-success)" };
  if (score >= 7) return { label: "Medium", tone: "text-(--color-warning)" };
  return { label: "Low", tone: "text-(--color-faint-foreground)" };
}

interface SessionSummary {
  start: Date;
  end: Date;
  durationMs: number;
  title: string;
  events: TimelineEvent[];
  editedFiles: Map<string, number>;
  filesChanged: Set<string>;
  counts: Partial<Record<TimelineEventType, number>>;
  productivity: { label: string; tone: string };
  firstEventId: string;
}

function summarizeSession(session: { start: Date; end: Date; events: TimelineEvent[] }, index: number): SessionSummary {
  const counts: Partial<Record<TimelineEventType, number>> = {};
  const editedFiles = new Map<string, number>();
  const filesChanged = new Set<string>();
  const perFileMinutes = new Map<string, number>();
  for (const e of session.events) {
    counts[e.eventType] = (counts[e.eventType] ?? 0) + 1;
    const file = eventFileName(e);
    if (file) filesChanged.add(file);
    if (e.eventType === "edit" && file) {
      editedFiles.set(file, (editedFiles.get(file) ?? 0) + 1);
      const prev = perFileMinutes.get(file) ?? new Date(session.start).getTime();
      if (new Date(e.occurredAt).getTime() - prev > 300000) {
        perFileMinutes.set(file, new Date(e.occurredAt).getTime());
      }
    }
  }
  const base = sessionName(session.start);
  const title =
    index === 0 ? base : `${base} ${index + 1}`;
  return {
    start: session.start,
    end: session.end,
    durationMs: session.end.getTime() - session.start.getTime(),
    title,
    events: session.events,
    editedFiles,
    filesChanged,
    counts,
    productivity: productivityOf(counts.edit ?? 0, counts.commit ?? 0, counts.create ?? 0),
    firstEventId: session.events[0]?.id ?? `${session.start.getTime()}`,
  };
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
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
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

  const daySessions = useMemo(() => {
    return [...grouped.entries()].map(([day, dayEvents]) => ({
      day,
      sessions: groupIntoSessions(dayEvents).map((s, i) => summarizeSession(s, i)),
    }));
  }, [grouped]);

  const [expandedSessions, setExpandedSessions] = useState<Set<string>>(new Set());

  useEffect(() => {
    const first = daySessions[0]?.sessions[0]?.firstEventId;
    setExpandedSessions(new Set(first ? [first] : []));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [events]);

  const toggleSession = useCallback((id: string) => {
    setExpandedSessions((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

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


  useEffect(() => {
    const handleClick = () => setContextMenu(null);
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
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
    <div ref={containerRef} className="mx-auto flex w-full max-w-6xl flex-col gap-8 px-8 py-8 lg:px-10">
      <div className="mb-8 flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div className="animate-fade-in">
          <p className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-(--color-accent)">
            <span className="h-1 w-1 rounded-full bg-(--color-accent)" />
            Activity
          </p>
          <h1 className="font-(family-name:--font-display) text-3xl font-bold tracking-tight text-(--color-foreground)">Timeline</h1>
          <p className="mt-1 text-sm text-(--color-muted-foreground)">Every edit, creation, and action across your workspaces, grouped into sessions.</p>
        </div>
        <div className="flex animate-fade-in items-center gap-2">
          <button
            onClick={fetchEvents}
            className="flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) px-3 py-2 text-sm text-(--color-muted-foreground) transition-all duration-200 hover:bg-(--color-surface-hover) hover:text-(--color-foreground) active:scale-[0.98]"
            title="Refresh"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isLoading ? "animate-spin" : ""}`} strokeWidth={1.75} />
            Refresh
          </button>
        </div>
      </div>

      <div className="mb-8 flex flex-wrap items-center gap-3">
        <div className="relative min-w-[180px] flex-[2]">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-muted-foreground)" strokeWidth={1.75} />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search files..."
            aria-label="Search timeline events by file name"
            className="w-full rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) py-2 pl-9 pr-3 text-sm text-(--color-foreground) shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] placeholder:text-(--color-faint-foreground) transition-colors focus:border-(--color-accent)/60 focus:outline-none focus:ring-2 focus:ring-(--color-accent)/15"
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
          className="rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) px-3 py-2 text-sm text-(--color-foreground) transition-colors focus:border-(--color-accent)/60 focus:outline-none focus:ring-2 focus:ring-(--color-accent)/15"
        >
          <option value="" disabled>Workspace</option>
          {workspaces.map((w) => (<option key={w.id} value={w.id}>{w.name}</option>))}
        </select>
        <div className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) p-1 shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
          {relevantTypes.map((type) => (
            <button
              key={type}
              onClick={() => setFilterType(type as TimelineEventType | "all")}
              className={`rounded-[calc(var(--radius-control)-4px)] px-2.5 py-1 text-xs font-medium capitalize whitespace-nowrap transition-all duration-200 ${
                filterType === type
                  ? "bg-(--color-accent) text-(--color-accent-foreground) shadow-[0_1px_8px_rgba(10,132,255,0.35)]"
                  : "text-(--color-muted-foreground) hover:text-(--color-foreground)"
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
              <div className="h-9 w-9 shrink-0 rounded-full bg-(--color-surface-hover)" />
              <div className="h-14 flex-1 rounded-[var(--radius-card)] bg-(--color-surface)" />
            </div>
          ))}
        </div>
      ) : error ? (
        <div className="rounded-[var(--radius-card)] border border-(--color-danger)/20 bg-(--color-danger)/5 px-6 py-12 text-center">
          <p className="mb-2 font-medium text-(--color-danger)">{error}</p>
          <button onClick={fetchEvents} className="text-sm text-(--color-accent) hover:underline">Try again</button>
        </div>
      ) : filtered.length === 0 ? (
        <div className="relative overflow-hidden rounded-[var(--radius-card)] border border-(--color-border-subtle) px-6 py-20 text-center">
          <div className="pointer-events-none absolute inset-0 bg-dotgrid opacity-30" aria-hidden="true" />
          <div className="relative mx-auto flex h-14 w-14 items-center justify-center rounded-2xl border border-(--color-border-subtle) bg-(--color-surface-raised)">
            <Clock className="h-6 w-6 text-(--color-faint-foreground)" strokeWidth={1.5} />
          </div>
          <h3 className="relative mt-4 mb-1 font-(family-name:--font-display) font-semibold text-(--color-foreground)">No events found</h3>
          <p className="relative text-sm text-(--color-muted-foreground)">
            {searchQuery ? "Try a different search term." : "Events will appear here as you work."}
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-10">
          {daySessions.map(({ day, sessions }) => (
            <section key={day} className="animate-fade-in">
              <div className="sticky top-0 z-10 -mx-8 mb-4 flex items-center gap-2.5 bg-(--color-background)/85 px-8 py-2 backdrop-blur-md lg:-mx-10 lg:px-10">
                <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-accent)" />
                <h2 className="font-(family-name:--font-display) text-[11px] font-semibold uppercase tracking-[0.18em] text-(--color-faint-foreground)">
                  {formatDayHeader(day)}
                </h2>
                <span className="rounded-full border border-(--color-border-subtle) bg-(--color-surface) px-1.5 py-px font-mono text-[10px] tabular-nums text-(--color-muted-foreground)">
                  {sessions.length} session{sessions.length !== 1 ? "s" : ""}
                </span>
                <div className="h-px flex-1 bg-gradient-to-r from-(--color-border-subtle) to-transparent" />
              </div>

              <div className="flex flex-col gap-4">
                {sessions.map((session, si) => {
                  const expanded = expandedSessions.has(session.firstEventId);
                  return (
                    <div
                      key={`${day}-${si}`}
                      className="overflow-hidden rounded-[var(--radius-card)] border border-(--color-border-subtle) bg-(--color-surface) shadow-[var(--shadow-card)] transition-colors duration-300 hover:border-(--color-border)"
                    >
                      <button
                        onClick={() => toggleSession(session.firstEventId)}
                        className="group flex w-full items-start gap-4 p-5 text-left"
                        aria-expanded={expanded}
                      >
                        <div className="flex w-14 shrink-0 flex-col gap-0.5 pt-0.5 sm:w-20">
                          <span className="font-mono text-[13px] font-semibold tabular-nums text-(--color-foreground)">
                            {formatTime(session.start)}
                          </span>
                          {session.events.length > 1 && (
                            <span className="font-mono text-[10px] tabular-nums text-(--color-faint-foreground)">
                              → {formatTime(session.end)}
                            </span>
                          )}
                        </div>

                        <div className="min-w-0 flex-1">
                          <div className="flex flex-wrap items-center gap-2.5">
                            <h3 className="font-(family-name:--font-display) text-[15px] font-semibold tracking-tight text-(--color-foreground)">
                              {session.title}
                            </h3>
                            <span className="inline-flex items-center gap-1 rounded-full border border-(--color-border-subtle) bg-(--color-surface-raised) px-2 py-0.5 text-[11px] font-medium tabular-nums text-(--color-muted-foreground)">
                              <Clock className="h-3 w-3" strokeWidth={1.75} />
                              {formatDuration(session.durationMs)}
                            </span>
                            <span className={`inline-flex items-center gap-1 text-[11px] font-semibold ${session.productivity.tone}`}>
                              <Zap className="h-3 w-3" strokeWidth={2} />
                              {session.productivity.label} productivity
                            </span>
                          </div>

                          {session.editedFiles.size > 0 && (
                            <div className="mt-3 flex flex-wrap items-center gap-1.5">
                              <span className="text-[10px] font-semibold uppercase tracking-wider text-(--color-faint-foreground)">
                                Edited
                              </span>
                              {[...session.editedFiles.entries()].slice(0, 6).map(([file, count]) => (
                                <span
                                  key={file}
                                  className="rounded-md border border-(--color-border-subtle) bg-(--color-surface-raised) px-2 py-0.5 font-(family-name:--font-mono) text-[11px] text-(--color-foreground)"
                                >
                                  {file}
                                  {count > 1 && <span className="ml-1 text-(--color-faint-foreground)">×{count}</span>}
                                </span>
                              ))}
                              {session.editedFiles.size > 6 && (
                                <span className="text-[11px] text-(--color-faint-foreground)">
                                  +{session.editedFiles.size - 6} more
                                </span>
                              )}
                            </div>
                          )}

                          <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-[11px] text-(--color-muted-foreground)">
                            {(session.counts.commit ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <GitCommit className="h-3 w-3 text-(--color-accent)" strokeWidth={2} />
                                {session.counts.commit} commit{session.counts.commit !== 1 ? "s" : ""}
                              </span>
                            )}
                            <span className="inline-flex items-center gap-1">
                              <FileEdit className="h-3 w-3 text-(--color-warning)" strokeWidth={2} />
                              {session.filesChanged.size} file{session.filesChanged.size !== 1 ? "s" : ""}
                            </span>
                            {(session.counts.create ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <FilePlus className="h-3 w-3 text-(--color-success)" strokeWidth={2} />
                                {session.counts.create} created
                              </span>
                            )}
                            {(session.counts.delete ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <Trash2 className="h-3 w-3 text-(--color-danger)" strokeWidth={2} />
                                {session.counts.delete} deleted
                              </span>
                            )}
                            {(session.counts.workspace_switch ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <ArrowRightLeft className="h-3 w-3 text-(--color-accent)" strokeWidth={2} />
                                Workspace switched ×{session.counts.workspace_switch}
                              </span>
                            )}
                            {(session.counts.open ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <Eye className="h-3 w-3" strokeWidth={2} />
                                {session.counts.open} opened
                              </span>
                            )}
                            {(session.counts.visit ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <Eye className="h-3 w-3" strokeWidth={2} />
                                {session.counts.visit} visited
                              </span>
                            )}
                            {(session.counts.screenshot ?? 0) > 0 && (
                              <span className="inline-flex items-center gap-1">
                                <Camera className="h-3 w-3 text-(--color-warning)" strokeWidth={2} />
                                {session.counts.screenshot} captured
                              </span>
                            )}
                          </div>
                        </div>

                        <ChevronDown
                          className={`mt-0.5 h-4 w-4 shrink-0 text-(--color-faint-foreground) transition-transform duration-300 ease-(--ease-premium) ${expanded ? "rotate-180" : ""}`}
                          strokeWidth={2}
                        />
                      </button>

                      {expanded && (
                        <div className="border-t border-(--color-border-subtle) bg-(--color-background)/40 px-5 py-4">
                          <div className="relative space-y-2.5 pl-8 before:absolute before:bottom-2 before:left-[13px] before:top-2 before:w-px before:bg-(--color-border)/40">
                            {session.events.map((event) => {
                              const colorClasses = EVENT_COLORS[event.eventType];
                              const fileName = eventFileName(event);
                              const filePath = eventFilePath(event);
                              const icon = extIcon(filePath);
                              return (
                                <div
                                  key={event.id}
                                  className="group relative flex items-start gap-4"
                                  onContextMenu={(e) => handleContextMenu(e, filePath)}
                                >
                                  <div className={`absolute left-0 top-1/2 z-10 flex h-7 w-7 -translate-y-1/2 items-center justify-center rounded-full border border-(--color-border-subtle) bg-(--color-surface) shadow-[0_1px_4px_rgba(0,0,0,0.4)] ${colorClasses}`}>
                                    {icon ?? EVENT_ICONS[event.eventType]}
                                  </div>
                                  <div className="flex min-w-0 flex-1 flex-col py-0.5">
                                    <div className="flex min-w-0 items-baseline gap-2">
                                      {fileName ? (
                                        <button
                                          onClick={() => { if (filePath) workspaceRepo.openFile(filePath).catch(() => {}); }}
                                          className="group/file flex items-center gap-1 truncate font-(family-name:--font-mono) text-sm font-medium text-(--color-foreground) hover:text-(--color-accent)"
                                        >
                                          {fileName}
                                          <ExternalLink className="h-3 w-3 shrink-0 opacity-0 transition-opacity group-hover/file:opacity-60" strokeWidth={1.75} />
                                        </button>
                                      ) : (
                                        <span className="text-sm font-medium text-(--color-muted-foreground)">
                                          {EVENT_LABELS[event.eventType]}
                                        </span>
                                      )}
                                      <span className="shrink-0 text-xs text-(--color-muted-foreground)">{EVENT_LABELS[event.eventType]}</span>
                                    </div>
                                    {filePath && (
                                      <p className="mt-0.5 truncate text-xs text-(--color-faint-foreground)">{filePath}</p>
                                    )}
                                    {typeof event.metadata?.from === "string" && typeof event.metadata?.to === "string" && (
                                      <div className="mt-0.5 flex items-center gap-1.5 text-xs text-(--color-muted-foreground)">
                                        <MoveRight className="h-3 w-3 shrink-0" strokeWidth={1.75} />
                                        <span className="truncate font-(family-name:--font-mono)">{event.metadata?.from as string}</span>
                                        <span className="text-(--color-faint-foreground)">→</span>
                                        <span className="truncate font-(family-name:--font-mono)">{event.metadata?.to as string}</span>
                                      </div>
                                    )}
                                  </div>
                                  <span className="ml-auto shrink-0 whitespace-nowrap pt-1 text-xs tabular-nums text-(--color-faint-foreground)">
                                    {formatRelativeTime(event.occurredAt)}
                                  </span>
                                </div>
                              );
                            })}
                          </div>
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
