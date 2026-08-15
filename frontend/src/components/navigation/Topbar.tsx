import { Search, Sun, Moon } from "lucide-react";
import { useEffect } from "react";
import { Button } from "@/components/ui/Button";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { GlassInput } from "@/components/ui/GlassInput";
import { useTheme } from "@/hooks/useTheme";
import { useNavigate, useLocation } from "react-router-dom";
import { cn } from "@/utils/cn";

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
 * Floating macOS-style toolbar — the same chrome material as the sidebar,
 * hovering above the canvas with a specular top edge and a native-feeling
 * search field.
 *
 * Spans the window's top edge: under the macOS overlay titlebar the traffic
 * lights float on this glass (reserved by the left padding), and the bar
 * itself drags the window (`data-tauri-drag-region`). Non-macOS platforms
 * simply get a full-width toolbar with no traffic-light reservation.
 */
export function Topbar() {
  const { resolvedTheme, setPreference } = useTheme();
  const isLight = resolvedTheme === "light";
  const navigate = useNavigate();
  const location = useLocation();

  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const isMacOS =
    isTauri &&
    typeof navigator !== "undefined" &&
    /Mac/i.test(navigator.platform || navigator.userAgent);

  const title = ROUTE_TITLES[location.pathname] ?? "ChronoDesk";

  // The visible ⌘K hint on the search field is a real shortcut: pressing
  // ⌘K anywhere jumps to Search (which then owns the input's focus).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        navigate("/search");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate]);

  return (
    <GlassSurface
      material="chrome"
      as="header"
      data-tauri-drag-region
      className={cn(
        "relative z-10 flex h-12 shrink-0 select-none items-center gap-3 overflow-hidden px-4",
        isMacOS ? "rounded-b-2xl pl-[76px]" : "rounded-b-2xl",
      )}
    >
      {/* Specular top edge — matches the sidebar's light catch. */}
      <div
        className="pointer-events-none absolute inset-x-2 top-0 h-px bg-gradient-to-r from-transparent via-white/40 to-transparent"
        aria-hidden="true"
      />

      <div className="flex min-w-0 flex-1 items-center" data-tauri-drag-region>
        <span className="font-(family-name:--font-display) truncate text-[13px] font-semibold tracking-tight text-(--color-foreground)" data-tauri-drag-region>
          {title}
        </span>
      </div>

      {/* macOS-style search field — an inset well in the chrome. */}
      <div className="relative flex h-8 w-52 min-w-0 md:w-64">
        <GlassInput
          size="md"
          placeholder="Search"
          onClick={() => navigate("/search")}
          onKeyDown={(e) => {
            if (e.key === "Enter") navigate("/search");
          }}
          icon={<Search className="h-3.5 w-3.5" strokeWidth={1.75} />}
          readOnly
          tabIndex={0}
          role="button"
          aria-keyshortcuts="Meta+K"
          aria-label="Search files and workspaces"
        />
        <kbd className="pointer-events-none absolute right-2 top-1/2 hidden -translate-y-1/2 rounded-[5px] border border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground) sm:block">
          ⌘K
        </kbd>
      </div>

      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="icon"
          aria-label={isLight ? "Switch to dark mode" : "Switch to light mode"}
          onClick={() => setPreference(isLight ? "dark" : "light")}
          className="h-7 w-7 text-(--color-faint-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
        >
          {isLight ? <Moon className="h-4 w-4" strokeWidth={1.75} /> : <Sun className="h-4 w-4" strokeWidth={1.75} />}
        </Button>
        <div className="ml-0.5 flex h-7 w-7 items-center justify-center rounded-full glass-control ring-1 ring-(--color-border-subtle) shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]">
          <span className="font-(family-name:--font-display) text-[11px] font-semibold text-(--color-muted-foreground)">
            U
          </span>
        </div>
      </div>
    </GlassSurface>
  );
}
