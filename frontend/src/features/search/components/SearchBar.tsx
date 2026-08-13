import { useState, useEffect, useCallback, useRef } from "react";
import { Search, X, Loader2 } from "lucide-react";
import { GlassInput } from "@/components/ui/GlassInput";

interface SearchBarProps {
  onSearch: (query: string) => void;
  isLoading?: boolean;
  /** External query to reflect (e.g. history/saved-search selection). */
  value?: string;
}

/** Prominent Liquid Glass search field — the page's primary interaction. */
export function SearchBar({ onSearch, isLoading, value }: SearchBarProps) {
  const [query, setQuery] = useState("");

  const onSearchRef = useRef(onSearch);
  useEffect(() => {
    onSearchRef.current = onSearch;
  });

  useEffect(() => {
    if (value !== undefined && document.activeElement !== document.getElementById("search-input")) {
      setQuery(value);
    }
  }, [value]);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      onSearchRef.current(query);
    }, 300);
    return () => clearTimeout(timeoutId);
  }, [query]);

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
      <GlassInput
        id="search-input"
        size="lg"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search files and workspaces…"
        aria-label="Search files and workspaces"
        icon={isLoading ? <Loader2 className="h-5 w-5 animate-spin" strokeWidth={1.75} /> : <Search className="h-5 w-5" strokeWidth={1.75} />}
      />
      {query && (
        <button
          onClick={handleClear}
          className="absolute inset-y-3 right-3 flex aspect-square items-center justify-center rounded-lg bg-(--color-surface-raised) px-1.5 text-(--color-faint-foreground) transition-all hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          aria-label="Clear search"
          title="Clear"
        >
          <X className="h-4 w-4" strokeWidth={1.75} />
        </button>
      )}
    </div>
  );
}
