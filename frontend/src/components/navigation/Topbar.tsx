import { Search, Sun, Moon, Settings, Monitor, Check } from "lucide-react";
import { useEffect, type RefObject } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Button } from "@/components/ui/Button";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { GlassInput } from "@/components/ui/GlassInput";
import { GlassMenu } from "@/components/ui/GlassMenu";
import { ScrollEdge } from "@/components/ui/ScrollEdge";
import { useTheme } from "@/hooks/useTheme";
import type { ThemePreference } from "@/contexts/ThemeContext";
import { useNavigate, useLocation } from "react-router-dom";
import { cn } from "@/utils/cn";
import { springs } from "@/lib/springs";

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
 *
 * The bottom edge carries a scroll-edge treatment driven by the page's
 * scroll container: soft dissolve normally, switching to the dimming
 * treatment when darker content passes underneath.
 */
export function Topbar({ scrollRef }: { scrollRef?: RefObject<HTMLElement | null> }) {
  const { resolvedTheme, preference, setPreference } = useTheme();
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

      {/* Scroll edge — content dissolving under the toolbar (auto: dims
          when darker content scrolls beneath). */}
      {scrollRef && <ScrollEdge containerRef={scrollRef} mode="auto" />}

      <div className="flex min-w-0 flex-1 items-center" data-tauri-drag-region>
        <AnimatePresence mode="wait" initial={false}>
          <motion.span
            key={title}
            className="font-(family-name:--font-display) truncate text-[13px] font-semibold tracking-tight text-(--color-foreground)"
            data-tauri-drag-region
            initial={{ opacity: 0, y: 3 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -3 }}
            transition={springs.default}
          >
            {title}
          </motion.span>
        </AnimatePresence>
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
        <GlassMenu
          label="User menu"
          align="end"
          closeOnItemClick
          trigger={
            <button
              aria-label="Open user menu"
              className="ml-0.5 flex h-7 w-7 items-center justify-center rounded-full glass-control ring-1 ring-(--color-border-subtle) shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]"
            >
              <span className="font-(family-name:--font-display) text-[11px] font-semibold text-(--color-muted-foreground)">
                U
              </span>
            </button>
          }
        >
          <div className="p-1">
            <button
              role="menuitem"
              onClick={() => navigate("/settings")}
              className="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-[13px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
            >
              <Settings className="h-3.5 w-3.5" strokeWidth={1.75} />
              Settings
            </button>
            <div className="separator mx-2 my-1" role="separator" />
            <p className="px-2.5 pb-1 pt-1.5 text-[10px] font-semibold uppercase tracking-[0.13em] text-(--color-faint-foreground)">
              Appearance
            </p>
            {(
              [
                ["dark", "Dark", Sun],
                ["light", "Light", Moon],
                ["system", "System", Monitor],
              ] as const
            ).map(([value, label, Icon]) => (
              <button
                key={value}
                role="menuitemradio"
                aria-checked={preference === value}
                onClick={() => setPreference(value as ThemePreference)}
                className="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-[13px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
              >
                <Icon className="h-3.5 w-3.5" strokeWidth={1.75} />
                {label}
                {preference === value && (
                  <Check className="ml-auto h-3.5 w-3.5 text-(--color-accent)" strokeWidth={2.25} />
                )}
              </button>
            ))}
          </div>
        </GlassMenu>
      </div>
    </GlassSurface>
  );
}