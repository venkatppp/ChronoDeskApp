import { useState, useEffect, useCallback } from "react";
import { Search, X, Loader2 } from "lucide-react";

interface SearchBarProps {
  onSearch: (query: string) => void;
  isLoading?: boolean;
}

/** Prominent Liquid Glass search field — the page's primary interaction. */
export function SearchBar({ onSearch, isLoading }: SearchBarProps) {
  const [query, setQuery] = useState("");

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      onSearch(query);
    }, 300);
    return () => clearTimeout(timeoutId);
  }, [query, onSearch]);

  const handleClear = useCallback(() => {
    setQuery("");
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        document.getElementById("search-input")?.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <div className="group relative w-full">
      <div className="pointer-events-none absolute inset-y-0 left-0 flex w-12 items-center justify-center rounded-l-[var(--radius-control)] text-(--color-faint-foreground) transition-colors group-focus-within:text-(--color-accent)">
        {isLoading ? (
          <Loader2 className="h-5 w-5 animate-spin" />
        ) : (
          <Search className="h-5 w-5" strokeWidth={1.75} />
        )}
      </div>
      <input
        id="search-input"
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search files and workspaces…"
        aria-label="Search files and workspaces"
        className="glass-well h-14 w-full rounded-[var(--radius-control)] pl-12 pr-12 text-[15px] text-(--color-foreground) outline-none transition-all duration-200 ease-[var(--ease-premium)] placeholder:text-(--color-faint-foreground) focus:shadow-[inset_0_1px_2px_rgba(0,0,0,0.25),0_0_0_1px_rgba(10,132,255,0.5)]"
      />
      {query && (
        <button
          onClick={handleClear}
          className="absolute inset-y-3 right-3 flex aspect-square items-center justify-center rounded-lg bg-(--color-surface-raised) px-1.5 text-(--color-faint-foreground) transition-all hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          aria-label="Clear search"
          title="Clear"
        >
          <X className="h-4 w-4" strokeWidth={2} />
        </button>
      )}
    </div>
  );
}
