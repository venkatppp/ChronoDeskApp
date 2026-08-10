import { Search, Sun, Moon } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { useTheme } from "@/hooks/useTheme";
import { useNavigate, useLocation } from "react-router-dom";

const ROUTE_TITLES: Record<string, string> = {
  "/": "Dashboard",
  "/workspaces": "Workspaces",
  "/timeline": "Timeline",
  "/graph": "Knowledge Graph",
  "/graph/performance": "Graph Performance",
  "/search": "Search",
  "/learning": "Learning",
  "/memory": "Memory",
  "/performance": "Performance",
  "/recovery": "Recovery",
  "/maintenance": "Maintenance",
  "/settings": "Settings",
};

/**
 * Floating macOS-style toolbar — translucent chrome hovering above the
 * canvas with a native-feeling search field.
 */
export function Topbar() {
  const { resolvedTheme, setPreference } = useTheme();
  const isLight = resolvedTheme === "light";
  const navigate = useNavigate();
  const location = useLocation();

  const title = ROUTE_TITLES[location.pathname] ?? "ChronoDesk";

  return (
    <GlassSurface
      material="chrome"
      as="header"
      className="relative z-10 flex h-12 shrink-0 items-center gap-3 overflow-hidden rounded-xl px-3.5"
    >
      <div className="flex min-w-0 flex-1 items-center">
        <span className="font-(family-name:--font-display) truncate text-[13px] font-semibold tracking-tight text-(--color-foreground)">
          {title}
        </span>
      </div>

      {/* macOS-style search field */}
      <div
        className="glass-well group relative flex h-8 w-64 cursor-text items-center rounded-[var(--radius-control)] transition-all duration-200 hover:shadow-[inset_0_1px_2px_rgba(0,0,0,0.25),0_0_0_1px_rgba(255,255,255,0.08)] focus-within:shadow-[inset_0_1px_2px_rgba(0,0,0,0.25),0_0_0_1px_rgba(10,132,255,0.45)]"
        onClick={() => navigate("/search")}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter") navigate("/search");
        }}
      >
        <Search
          className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-(--color-faint-foreground)"
          strokeWidth={1.75}
        />
        <span className="pointer-events-none pl-8 pr-3 text-[13px] text-(--color-faint-foreground)">Search</span>
        <kbd className="pointer-events-none absolute right-2 rounded-[5px] border border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground)">
          ⌘K
        </kbd>
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          aria-label={isLight ? "Switch to dark mode" : "Switch to light mode"}
          onClick={() => setPreference(isLight ? "dark" : "light")}
          className="h-7 w-7 text-(--color-faint-foreground)"
        >
          {isLight ? <Moon className="h-4 w-4" strokeWidth={1.75} /> : <Sun className="h-4 w-4" strokeWidth={1.75} />}
        </Button>
        <div className="ml-0.5 flex h-7 w-7 items-center justify-center rounded-full bg-(--color-surface-raised) ring-1 ring-(--color-border-subtle) shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
          <span className="font-(family-name:--font-display) text-[11px] font-semibold text-(--color-muted-foreground)">
            U
          </span>
        </div>
      </div>
    </GlassSurface>
  );
}
