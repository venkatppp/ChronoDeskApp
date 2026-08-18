import { useCallback, useMemo, useRef, useState } from "react";
import { Search, X } from "lucide-react";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { CONTEXT_EDGES, CONTEXT_NODES, ENTRY_CONNECTIONS } from "@/features/context/model";
import { computeContextLayout } from "@/features/context/layout";
import {
  WorkspaceContextView,
  type WorkspaceContextViewHandle,
} from "@/features/context/WorkspaceContextView";

/**
 * Workspace Context — the app's own structure as a live graph.
 *
 * App.tsx floats at the center as the workspace entry point, ringed by
 * the active context (components, services, hooks, styles, tests) and
 * the wider workspace artifacts beyond them. The graph is the hero; the
 * header is a quiet title + search, and the App.tsx inspector floats
 * bottom-right, subtle and secondary.
 *
 * State lives here (selection, search); the view owns the camera and
 * rendering so pan/zoom never re-renders this page.
 */
export function ContextPage() {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const viewRef = useRef<WorkspaceContextViewHandle | null>(null);

  const layout = useMemo(() => computeContextLayout(CONTEXT_NODES), []);
  const trimmed = query.trim().toLowerCase();

  const searchMatches = useMemo(() => {
    if (!trimmed) return new Set<string>();
    const hits = new Set<string>();
    for (const node of CONTEXT_NODES) {
      if (
        node.label.toLowerCase().includes(trimmed) ||
        node.path.toLowerCase().includes(trimmed) ||
        node.role.toLowerCase().includes(trimmed)
      ) {
        hits.add(node.id);
      }
    }
    return hits;
  }, [trimmed]);

  const firstMatch = useMemo(() => {
    if (!trimmed) return null;
    return CONTEXT_NODES.find((n) => searchMatches.has(n.id)) ?? null;
  }, [trimmed, searchMatches]);

  const handleSelect = useCallback((id: string) => setSelectedId(id), []);
  const handleDeselect = useCallback(() => setSelectedId(null), []);

  const handleSearchKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Escape") {
        setQuery("");
        return;
      }
      if (e.key === "Enter" && firstMatch) {
        setSelectedId(firstMatch.id);
        viewRef.current?.flyToNode(firstMatch.id, 1.55);
      }
    },
    [firstMatch],
  );

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header — the graph stays the hero; this bar is quiet. */}
      <GlassSurface
        material="chrome"
        className="relative z-10 flex shrink-0 items-center justify-between gap-6 border-b border-(--color-border-subtle) px-6 py-4"
      >
        <div className="min-w-0">
          <h1 className="font-(family-name:--font-display) text-[1.75rem] font-semibold tracking-[-0.02em] text-(--color-foreground)">
            Workspace Context
          </h1>
          <p className="mt-0.5 text-sm leading-relaxed text-(--color-muted-foreground)">
            Relationships discovered across your workspace
          </p>
        </div>

        <div className="relative w-64 shrink-0 sm:w-72">
          <div className="glass-control flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) px-3 py-2">
            <Search className="h-3.5 w-3.5 shrink-0 text-(--color-faint-foreground)" strokeWidth={1.75} />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleSearchKeyDown}
              placeholder="Search context"
              aria-label="Search workspace context"
              className="w-full bg-transparent text-xs text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:outline-none"
            />
            {query && (
              <button
                onClick={() => setQuery("")}
                className="rounded p-0.5 text-(--color-faint-foreground) hover:text-(--color-foreground)"
                aria-label="Clear search"
              >
                <X className="h-3 w-3" strokeWidth={1.75} />
              </button>
            )}
          </div>
          {trimmed && searchMatches.size === 0 && (
            <p className="absolute right-0 top-full mt-1.5 text-[10px] text-(--color-faint-foreground)">
              No context matches “{query}”
            </p>
          )}
        </div>
      </GlassSurface>

      {/* The graph — full-bleed canvas below the header. */}
      <div className="relative min-h-0 flex-1">
        <WorkspaceContextView
          ref={viewRef}
          edges={CONTEXT_EDGES}
          layout={layout}
          selectedId={selectedId}
          onSelect={handleSelect}
          onDeselect={handleDeselect}
          searchActive={trimmed.length > 0}
          searchMatches={searchMatches}
        />

        {/* Floating App.tsx inspector — subtle, secondary, always present:
            the active context of the workspace. */}
        <GlassSurface
          material="panel"
          refraction={true}
          className="absolute bottom-5 right-5 z-20 w-[208px] animate-fade-in rounded-[var(--radius-card)] p-4"
        >
          <div className="flex items-center justify-between gap-2">
            <p className="font-(family-name:--font-display) text-sm font-semibold tracking-tight text-(--color-foreground)">
              App.tsx
            </p>
            <span className="inline-flex items-center gap-1 rounded-full border border-(--color-border-subtle) bg-(--color-surface-hover) px-2 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-(--color-muted-foreground)">
              <span className="h-1.5 w-1.5 rounded-full bg-(--color-success)" />
              Active
            </span>
          </div>
          <p className="mt-0.5 text-[11px] text-(--color-muted-foreground)">React Component</p>

          <div className="my-2.5 h-px bg-gradient-to-r from-transparent via-(--color-border) to-transparent" aria-hidden="true" />

          <div className="flex items-center justify-between">
            <span className="text-[11px] text-(--color-muted-foreground)">Connections</span>
            <span className="font-(family-name:--font-mono) text-[11px] tabular-nums text-(--color-foreground)">
              {ENTRY_CONNECTIONS}
            </span>
          </div>
          <p className="mt-1.5 text-[11px] leading-relaxed text-(--color-faint-foreground)">
            Workspace Entry Point
          </p>
        </GlassSurface>
      </div>
    </div>
  );
}