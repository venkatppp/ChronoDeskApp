import {
  LayoutDashboard,
  FolderKanban,
  History,
  Share2,
  Gauge,
  Brain,
  Sparkles,
  PlayCircle,
  Bot,
  BrainCircuit,
  ActivitySquare,
  ShieldCheck,
  DatabaseBackup,
  Settings,
  Search,
  Command,
} from "lucide-react";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { NavItem } from "@/components/navigation/NavItem";

const NAV_GROUPS: { label: string; scopes: { to: string; label: string; icon: typeof LayoutDashboard; end?: boolean }[] }[] = [
  {
    label: "Overview",
    scopes: [
      { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
      { to: "/workspaces", label: "Workspaces", icon: FolderKanban },
      { to: "/timeline", label: "Timeline", icon: History },
      { to: "/graph", label: "Knowledge Graph", icon: Share2 },
      { to: "/graph/performance", label: "Graph Performance", icon: Gauge },
    ],
  },
  {
    label: "Intelligence",
    scopes: [
      { to: "/search", label: "Search", icon: Search },
      { to: "/learning", label: "Learning", icon: Brain },
      { to: "/copilot", label: "AI Copilot", icon: Sparkles, end: true },
      { to: "/memory", label: "Memory", icon: BrainCircuit },
    ],
  },
  {
    label: "Runs",
    scopes: [
      { to: "/executions", label: "Executions", icon: PlayCircle },
      { to: "/autonomous", label: "Autonomous", icon: Bot },
    ],
  },
  {
    label: "System",
    scopes: [
      { to: "/performance", label: "Performance", icon: ActivitySquare },
      { to: "/recovery", label: "Recovery", icon: ShieldCheck },
      { to: "/maintenance", label: "Maintenance", icon: DatabaseBackup },
    ],
  },
];

interface RuntimeHealth {
  status: "Healthy" | "Degraded" | "Unhealthy";
  workersActive: number;
  uptimeSeconds: number;
  checkedAt: string;
}

export function Sidebar() {
  const [health, setHealth] = useState<RuntimeHealth | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const h = await invoke<RuntimeHealth>("get_runtime_health");
        if (!cancelled) setHealth(h);
      } catch {
        if (!cancelled) setHealth(null);
      }
    };
    void load();
    const timer = window.setInterval(load, 30_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const statusTone =
    health?.status === "Unhealthy"
      ? { dot: "bg-(--color-danger)", label: "System degraded" }
      : health?.status === "Degraded"
        ? { dot: "bg-(--color-warning)", label: "System degraded" }
        : { dot: "bg-(--color-success)", label: "System nominal" };

  return (
    <aside className="flex h-full w-60 shrink-0 flex-col border-r border-(--color-border-subtle) bg-(--color-background) px-3 py-4">
      <div className="mb-6 flex items-center gap-2.5 px-2">
        <div className="relative flex h-8 w-8 items-center justify-center overflow-hidden rounded-xl border border-(--color-border-subtle) bg-gradient-to-br from-(--color-accent)/25 to-(--color-surface-raised)">
          <span className="font-(family-name:--font-display) text-sm font-bold text-(--color-accent)">C</span>
          <span
            className="pointer-events-none absolute inset-x-0 bottom-0 h-[2px] animate-(--animate-pulse-line) bg-[linear-gradient(90deg,transparent,var(--color-accent),transparent)] bg-size-[200%_100%]"
            aria-hidden="true"
          />
        </div>
        <div className="flex flex-col leading-none">
          <span className="font-(family-name:--font-display) text-sm font-bold tracking-tight">ChronoDesk</span>
          <span className="mt-1 flex items-center gap-1 text-[10px] text-(--color-faint-foreground)">
            <Command className="h-2.5 w-2.5" strokeWidth={2} />
            Workspace Layer
          </span>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-5 overflow-y-auto">
        {NAV_GROUPS.map((group) => (
          <div key={group.label}>
            <p className="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-[0.16em] text-(--color-faint-foreground)">
              {group.label}
            </p>
            <div className="flex flex-col gap-px">
              {group.scopes.map((item) => (
                <NavItem key={item.to} {...item} />
              ))}
            </div>
          </div>
        ))}
      </nav>

      <div className="mt-auto flex flex-col gap-4">
        <div className="flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-3 py-2">
          <span className="relative flex h-2 w-2 shrink-0">
            <span className={`absolute inline-flex h-full w-full animate-ping rounded-full opacity-40 ${statusTone.dot}`} />
            <span className={`relative inline-flex h-2 w-2 rounded-full ${statusTone.dot}`} />
          </span>
          <div className="flex min-w-0 flex-col leading-tight">
            <span className="text-[11px] font-medium text-(--color-foreground)">{statusTone.label}</span>
            <span className="text-[10px] text-(--color-faint-foreground)">
              {health
                ? `${health.workersActive} monitored component${health.workersActive === 1 ? "" : "s"} · ${Math.round(health.uptimeSeconds / 60)}m uptime`
                : "Runtime status unavailable"}
            </span>
          </div>
        </div>
        <div className="flex flex-col gap-px border-t border-(--color-border-subtle) pt-2">
          <NavItem to="/settings" label="Settings" icon={Settings} />
        </div>
      </div>
    </aside>
  );
}