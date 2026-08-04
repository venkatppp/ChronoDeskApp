import {
  LayoutDashboard,
  FolderKanban,
  History,
  Share2,
  BarChart3,
  Settings,
  Brain,
  Sparkles,
  PlayCircle,
  Bot,
  BrainCircuit,
  Gauge,
  ActivitySquare,
  ShieldCheck,
  DatabaseBackup,
} from "lucide-react";
import { NavItem } from "@/components/navigation/NavItem";

const PRIMARY_NAV = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/workspaces", label: "Workspaces", icon: FolderKanban },
  { to: "/timeline", label: "Timeline", icon: History },
  { to: "/graph", label: "Knowledge Graph", icon: Share2 },
  { to: "/graph/performance", label: "Graph Performance", icon: Gauge },
  { to: "/analytics", label: "Analytics", icon: BarChart3 },
  { to: "/learning", label: "Learning", icon: Brain },
  { to: "/copilot", label: "AI Copilot", icon: Sparkles },
  { to: "/executions", label: "Executions", icon: PlayCircle },
  { to: "/autonomous", label: "Autonomous", icon: Bot },
  { to: "/memory", label: "Memory", icon: BrainCircuit },
  { to: "/performance", label: "Performance", icon: ActivitySquare },
  { to: "/recovery", label: "Recovery", icon: ShieldCheck },
  { to: "/maintenance", label: "Maintenance", icon: DatabaseBackup },
] as const;

export function Sidebar() {
  return (
    <aside className="flex h-full w-64 shrink-0 flex-col border-r border-(--color-border) bg-(--color-surface) px-3 py-4">
      <div className="mb-6 flex items-center gap-2 px-2">
        <div className="relative flex h-7 w-7 items-center justify-center overflow-hidden rounded-lg bg-(--color-accent-muted)">
          <span className="font-(family-name:--font-display) text-sm font-bold text-(--color-accent)">C</span>
          {/* Ambient time-pulse, the product's signature motion motif. */}
          <span
            className="pointer-events-none absolute inset-x-0 bottom-0 h-[2px] animate-(--animate-pulse-line) bg-[linear-gradient(90deg,transparent,var(--color-accent),transparent)] bg-size-[200%_100%]"
            aria-hidden="true"
          />
        </div>
        <div className="flex flex-col leading-none">
          <span className="font-(family-name:--font-display) text-sm font-bold">ChronoDesk</span>
          <span className="text-[11px] text-(--color-faint-foreground)">Workspace Layer</span>
        </div>
      </div>

      <nav className="flex flex-1 flex-col gap-0.5">
        {PRIMARY_NAV.map((item) => (
          <NavItem key={item.to} {...item} />
        ))}
      </nav>

      <div className="mt-auto flex flex-col gap-0.5 border-t border-(--color-border-subtle) pt-2">
        <NavItem to="/settings" label="Settings" icon={Settings} />
      </div>
    </aside>
  );
}
