import { useCallback, useEffect, useRef, useState } from "react";
import type { KgNode, GraphNodeType } from "@/types/graph";
import { Loader2 } from "lucide-react";

interface VirtualizedNodeListProps {
  /** Nodes loaded so far (progressive pages appended). */
  nodes: KgNode[];
  /** Total node count for the current filter. */
  total: number;
  loading: boolean;
  onLoadMore: () => void;
  onSelect: (node: KgNode) => void;
  selectedId?: string | null;
  /** Row height in px — the virtualization window unit. */
  rowHeight?: number;
  /** Node-type accent color map (the colored dot per row). */
  typeColors: Record<GraphNodeType, string>;
}

const OVERSCAN_ROWS = 6;
const LOAD_MORE_MARGIN_ROWS = 4;

/**
 * Virtualized + progressively loaded node list for the graph
 * performance page. Only the rows inside the viewport window are
 * rendered; scrolling past the loaded region triggers `onLoadMore`
 * until `nodes.length >= total`.
 */
export function VirtualizedNodeList({
  nodes,
  total,
  loading,
  onLoadMore,
  onSelect,
  selectedId,
  rowHeight = 48,
  typeColors,
}: VirtualizedNodeListProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(480);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const measure = () => setViewportHeight(el.clientHeight || 480);
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  const handleScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      const el = event.currentTarget;
      setScrollTop(el.scrollTop);
      const nearBottom =
        el.scrollTop + el.clientHeight >= el.scrollHeight - LOAD_MORE_MARGIN_ROWS * rowHeight;
      if (nearBottom && nodes.length < total && !loading) {
        onLoadMore();
      }
    },
    [rowHeight, nodes.length, total, loading, onLoadMore],
  );

  const startIndex = Math.max(0, Math.floor(scrollTop / rowHeight) - OVERSCAN_ROWS);
  const endIndex = Math.min(
    nodes.length,
    Math.ceil((scrollTop + viewportHeight) / rowHeight) + OVERSCAN_ROWS,
  );
  const visibleNodes = nodes.slice(startIndex, endIndex);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="glass-panel min-h-0 max-h-[480px] flex-1 overflow-y-auto rounded-[var(--radius-control)]"
        data-testid="virtualized-node-list"
      >
        <div style={{ height: nodes.length * rowHeight, position: "relative" }}>
          {visibleNodes.map((node, i) => {
            const absoluteIndex = startIndex + i;
            return (
              <button
                key={`${node.nodeType}-${node.entityId}`}
                onClick={() => onSelect(node)}
                className={`absolute left-0 right-0 flex w-full items-center gap-2.5 border-b border-(--color-border-subtle)/60 px-3 text-left transition-colors hover:bg-(--color-surface-hover) ${
                  selectedId === node.entityId ? "bg-(--color-accent)/10" : ""
                }`}
                style={{ top: absoluteIndex * rowHeight, height: rowHeight }}
              >
                <span
                  className="h-2 w-2 shrink-0 rounded-full"
                  style={{ backgroundColor: typeColors[node.nodeType] }}
                />
                <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">
                  {node.title}
                </span>
                <span className="shrink-0 rounded px-1.5 py-0.5 text-[9px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                  {node.nodeType.replace("_", " ")}
                </span>
              </button>
            );
          })}
        </div>
        {loading && (
          <div className="flex items-center justify-center gap-2 py-3 text-xs text-(--color-faint-foreground)">
            <Loader2 className="h-3.5 w-3.5 animate-spin" strokeWidth={1.75} />
            Loading more nodes…
          </div>
        )}
        {!loading && nodes.length >= total && nodes.length > 0 && (
          <div className="py-3 text-center text-[10px] text-(--color-faint-foreground)">
            All {total} nodes loaded
          </div>
        )}
      </div>
      <p className="mt-1.5 text-[10px] text-(--color-faint-foreground)">
        Showing {Math.min(nodes.length, total)} of {total} nodes
        {nodes.length < total ? " · scroll to load more" : ""}
      </p>
    </div>
  );
}
