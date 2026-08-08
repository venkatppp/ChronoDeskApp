import { Search, Bell, Sun, Moon } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { useTheme } from "@/hooks/useTheme";
import { useNavigate } from "react-router-dom";

export function Topbar() {
  const { resolvedTheme, setPreference } = useTheme();
  const isLight = resolvedTheme === "light";
  const navigate = useNavigate();

  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-(--color-border-subtle) bg-(--color-background) px-5">
      <div
        className="relative flex max-w-md flex-1 cursor-text items-center rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) transition-colors duration-200 hover:border-(--color-border) focus-within:border-(--color-accent)/60 focus-within:shadow-[var(--shadow-glow)]"
        onClick={() => navigate("/search")}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter") navigate("/search");
        }}
      >
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-faint-foreground)" strokeWidth={1.75} />
        <span className="pointer-events-none pl-9 pr-3 text-sm text-(--color-faint-foreground)">Search everything…</span>
        <kbd className="pointer-events-none absolute right-2.5 rounded border border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground)">
          ⌘K
        </kbd>
      </div>

      <div className="ml-auto flex items-center gap-1.5">
        <Button
          variant="ghost"
          size="icon"
          aria-label={isLight ? "Switch to dark mode" : "Switch to light mode"}
          onClick={() => setPreference(isLight ? "dark" : "light")}
        >
          {isLight ? <Moon className="h-4 w-4" strokeWidth={1.75} /> : <Sun className="h-4 w-4" strokeWidth={1.75} />}
        </Button>
        <Button variant="ghost" size="icon" aria-label="Notifications">
          <Bell className="h-4 w-4" strokeWidth={1.75} />
        </Button>
        <div className="ml-1 flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-(--color-accent)/30 to-(--color-surface-raised) ring-1 ring-(--color-border-subtle)">
          <span className="font-(family-name:--font-display) text-xs font-semibold text-(--color-accent)">U</span>
        </div>
      </div>
    </header>
  );
}