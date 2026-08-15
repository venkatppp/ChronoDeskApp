import {
  LayoutDashboard,
  FolderKanban,
  History,
  Share2,
  Gauge,
  Brain,
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
import { GlassSurface } from "@/components/ui/GlassSurface";

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
      { to: "/memory", label: "Memory", icon: BrainCircuit },
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

/**
 * Level 1 floating chrome — translucent glass above the canvas, with real
 * SVG-displacement refraction on Chromium and a dense frosted blur on
 * WKWebView. The environment visibly bends at its rim.
 *
 * Geometry: 280px floating pane, rounded-3xl, bright specular top edge,
 * deep soft shadow — reads as a pane of glass, not a dark rectangle.
 */
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
    <GlassSurface
      material="chrome"
      as="aside"
      className="relative z-10 flex h-full w-[280px] shrink-0 flex-col overflow-hidden rounded-3xl px-3.5 pb-4 pt-5"
    >
      {/* Specular sheen along the top edge — the light catches the pane. */}
      <div
        className="pointer-events-none absolute inset-x-3 top-0 h-px bg-gradient-to-r from-transparent via-white/40 to-transparent"
        aria-hidden="true"
      />

      {/* Brand — 15px semibold, reads as the pane's title. */}
      <div className="mb-5 flex items-center gap-2.5 px-2 pb-3.5">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[9px] bg-(--color-accent-muted) shadow-[inset_0_1px_0_rgba(255,255,255,0.14)] ring-1 ring-(--color-accent)/30">
          <Command className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
        </div>
        <div className="flex flex-col leading-tight">
          <span className="font-(family-name:--font-display) text-[15px] font-semibold tracking-tight text-(--color-foreground)">
            ChronoDesk
          </span>
          <span className="text-[11px] text-(--color-faint-foreground)">Workspace Layer</span>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-5 overflow-y-auto">
        {NAV_GROUPS.map((group) => (
          <div key={group.label}>
            <p className="mb-1.5 px-2.5 text-[10px] font-semibold uppercase tracking-[0.13em] text-(--color-faint-foreground)">
              {group.label}
            </p>
            <div className="flex flex-col gap-0.5">
              {group.scopes.map((item) => (
                <NavItem key={item.to} {...item} />
              ))}
            </div>
          </div>
        ))}
      </nav>

      <div className="mt-auto flex flex-col gap-3.5">
        {/* Bottom status bar with subtle glass separation */}
        <div className="relative flex items-center gap-2.5 rounded-xl px-2.5 py-1.5">
          <div className="pointer-events-none absolute inset-x-1 top-0 h-px bg-gradient-to-r from-transparent via-white/20 to-transparent" aria-hidden="true" />
          <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${statusTone.dot}`} />
          <div className="flex min-w-0 flex-col leading-tight">
            <span className="text-[11px] font-medium text-(--color-foreground) text-on-glass">{statusTone.label}</span>
            <span className="truncate text-[10px] text-(--color-faint-foreground)">
              {health
                ? `${health.workersActive} component${health.workersActive === 1 ? "" : "s"} · ${Math.round(health.uptimeSeconds / 60)}m uptime`
                : "Runtime status unavailable"}
            </span>
          </div>
        </div>
        <div className="flex flex-col gap-0.5 border-t border-(--color-border-subtle) pt-2.5">
          <NavItem to="/settings" label="Settings" icon={Settings} />
        </div>
      </div>
    </GlassSurface>
  );
}
