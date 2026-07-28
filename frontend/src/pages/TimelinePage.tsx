import { useState, useEffect, useCallback } from "react";
import { Clock, Filter, FilePlus, FileEdit, Trash2, ArrowRightLeft, Eye, Camera, RefreshCw } from "lucide-react";
import { getTimelineRepository } from "@/services/timelineRepository";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { TimelineEvent, TimelineEventType } from "@/types/timeline";
import type { Workspace } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

export function TimelinePage() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string>("");
  const [events, setEvents] = useState<TimelineEvent[]>([]);
  const [filterType, setFilterType] = useState<TimelineEventType | "all">("all");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const timelineRepo = getTimelineRepository();
  const workspaceRepo = getWorkspaceRepository();

  useEffect(() => {
    workspaceRepo.listActiveWorkspaces().then((ws) => {
      setWorkspaces(ws);
      if (ws.length > 0) {
        // Try to restore last active workspace from localStorage
        const storedId = localStorage.getItem('activeWorkspaceId');
        if (storedId && ws.some(w => w.id === storedId)) {
          setSelectedWorkspaceId(storedId);
        } else {
          setSelectedWorkspaceId(ws[0].id);
        }
      }
    });
  }, [workspaceRepo]);

  const fetchEvents = useCallback(async () => {
    if (!selectedWorkspaceId) return;
    setIsLoading(true);
    setError(null);
    try {
      const allEvents = await timelineRepo.listWorkspaceTimeline(selectedWorkspaceId, 50);
      setEvents(allEvents);
    } catch (err) {
      console.error("Failed to fetch timeline events:", err);
      setError("Failed to load timeline. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [timelineRepo, selectedWorkspaceId]);

  useEffect(() => {
    fetchEvents();
  }, [fetchEvents]);

  const filteredEvents = events.filter((e) => filterType === "all" || e.eventType === filterType);

  const getEventIcon = (type: TimelineEventType) => {
    switch (type) {
      case "create": return <FilePlus className="h-4 w-4" />;
      case "edit": return <FileEdit className="h-4 w-4" />;
      case "delete": return <Trash2 className="h-4 w-4" />;
      case "workspace_switch": return <ArrowRightLeft className="h-4 w-4" />;
      case "visit":
      case "open": return <Eye className="h-4 w-4" />;
      case "screenshot": return <Camera className="h-4 w-4" />;
      default: return <Clock className="h-4 w-4" />;
    }
  };

  const getEventColor = (type: TimelineEventType) => {
    switch (type) {
      case "create": return "bg-emerald-500/10 text-emerald-500";
      case "edit": return "bg-amber-500/10 text-amber-500";
      case "delete": return "bg-destructive/10 text-destructive";
      case "workspace_switch": return "bg-blue-500/10 text-blue-500";
      default: return "bg-primary/10 text-primary";
    }
  };

  return (
    <div className="max-w-4xl mx-auto px-6 py-10">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 mb-10">
        <div>
          <h1 className="text-4xl font-bold text-foreground mb-2">Timeline</h1>
          <p className="text-muted-foreground text-lg">Trace your digital journey across time.</p>
        </div>
        <button 
          onClick={fetchEvents}
          className="p-3 bg-background-secondary border border-border rounded-xl hover:bg-background-tertiary transition-colors"
          title="Refresh"
        >
          <RefreshCw className={`h-5 w-5 ${isLoading ? 'animate-spin' : ''}`} />
        </button>
      </div>

      <div className="flex flex-wrap gap-4 mb-10">
        <div className="relative flex-1 min-w-[200px]">
          <select
            value={selectedWorkspaceId}
            onChange={async (e) => {
              const id = e.target.value;
              setSelectedWorkspaceId(id);
              localStorage.setItem('activeWorkspaceId', id);
              try {
                await workspaceRepo.switchWorkspace(id);
              } catch (err) {
                console.error('Failed to switch workspace:', err);
              }
            }}
            className="w-full h-12 pl-4 pr-10 bg-background-secondary border border-border rounded-xl appearance-none focus:outline-none focus:ring-2 focus:ring-primary font-medium text-foreground"
          >
            <option value="" disabled>Select Workspace</option>
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>{w.name}</option>
            ))}
          </select>
          <Filter className="absolute right-4 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
        </div>

        <div className="flex bg-background-secondary p-1 rounded-xl border border-border overflow-x-auto no-scrollbar">
          {(["all", "create", "edit", "delete", "visit"] as const).map((type) => (
            <button
              key={type}
              onClick={() => setFilterType(type)}
              className={`px-5 py-2 text-sm font-bold rounded-lg transition-all capitalize whitespace-nowrap ${
                filterType === type
                  ? "bg-primary text-primary-foreground shadow-md"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {type}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-6">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="flex gap-6 animate-pulse">
              <div className="w-px bg-border relative"><div className="absolute top-0 left-1/2 -translate-x-1/2 w-4 h-4 bg-background-tertiary rounded-full border-4 border-background" /></div>
              <div className="flex-1 h-24 bg-background-secondary rounded-2xl" />
            </div>
          ))}
        </div>
      ) : error ? (
        <div className="py-20 text-center bg-destructive/5 border border-destructive/10 rounded-3xl">
          <h3 className="text-xl font-bold text-foreground mb-2">{error}</h3>
          <button onClick={fetchEvents} className="text-primary font-bold hover:underline">Try again</button>
        </div>
      ) : filteredEvents.length === 0 ? (
        <div className="py-32 text-center bg-background-secondary/30 border-2 border-dashed border-border rounded-3xl">
          <Clock className="h-16 w-16 mx-auto mb-6 text-muted-foreground opacity-20" />
          <h3 className="text-2xl font-bold text-foreground mb-2">No events recorded</h3>
          <p className="text-muted-foreground max-w-sm mx-auto">
            Try selecting a different workspace or filter. Activities appear here as you work.
          </p>
        </div>
      ) : (
        <div className="relative space-y-8 before:absolute before:inset-y-0 before:left-4 before:w-0.5 before:bg-border/50">
          {filteredEvents.map((event, index) => (
            <div key={event.id} className="relative flex gap-8 group animate-in fade-in slide-in-from-left-4" style={{ animationDelay: `${index * 50}ms` }}>
              <div className={`z-10 mt-1 flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center border-4 border-background transition-all group-hover:scale-110 ${getEventColor(event.eventType)}`}>
                {getEventIcon(event.eventType)}
              </div>
              
              <div className="flex-1 bg-background-secondary border border-border rounded-2xl p-5 hover:border-primary/40 hover:shadow-xl transition-all">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-xs font-bold text-muted-foreground uppercase tracking-widest">
                    {event.eventType}
                  </div>
                  <div className="text-xs text-muted-foreground font-medium">
                    {formatRelativeTime(event.occurredAt)}
                  </div>
                </div>
                
                <h4 className="text-lg font-bold text-foreground mb-1">
                  {event.fileId ? event.fileId.split("/").pop() : "System Event"}
                </h4>
                
                {event.fileId && (
                  <p className="text-sm text-muted-foreground font-mono truncate opacity-60">
                    {event.fileId}
                  </p>
                )}

                {event.metadata && Object.keys(event.metadata).length > 0 && (
                  <div className="mt-4 pt-4 border-t border-border/50 grid grid-cols-2 gap-4">
                    {Object.entries(event.metadata).map(([key, value]) => (
                      <div key={key}>
                        <div className="text-[10px] text-muted-foreground font-bold uppercase">{key}</div>
                        <div className="text-xs text-foreground font-medium truncate">{String(value)}</div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
