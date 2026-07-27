import { Search, Bell, Sun, Moon } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { useTheme } from "@/hooks/useTheme";

export function Topbar() {
  const { resolvedTheme, setPreference } = useTheme();
  const isLight = resolvedTheme === "light";

  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-(--color-border) bg-(--color-background) px-5">
      <div className="relative flex-1 max-w-md">
        <Search
          className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-(--color-faint-foreground)"
          strokeWidth={1.75}
        />
        <input
          type="text"
          placeholder="Search everything…"
          className="h-9 w-full rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) pl-9 pr-3
                     text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground)
                     outline-none transition-colors focus:border-(--color-accent)"
        />
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
        <div className="ml-1 h-8 w-8 rounded-full bg-(--color-accent-muted) flex items-center justify-center">
          <span className="font-(family-name:--font-display) text-xs font-semibold text-(--color-accent)">U</span>
        </div>
      </div>
    </header>
  );
}
