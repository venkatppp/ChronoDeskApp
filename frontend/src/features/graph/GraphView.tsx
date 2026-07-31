import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import type { GraphView as GraphViewType, GraphNode, GraphEdge } from "@/types/graph";
import { ZoomIn, ZoomOut, Maximize, Focus, Search, X, Keyboard, Minimize2, Expand } from "lucide-react";

interface PositionedNode extends GraphNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
}

const NODE_RADIUS: Record<string, number> = {
  workspace: 34,
  folder: 28,
  file: 24,
  language: 20,
  project: 30,
};
const NODE_DEFAULT_RADIUS = 24;

const NODE_COLORS: Record<string, string> = {
  workspace: "var(--color-accent)",
  folder: "var(--color-warning)",
  file: "var(--color-success)",
  language: "var(--color-danger)",
  project: "var(--color-accent)",
};

const EDGE_COLORS: Record<string, string> = {
  co_occurrence: "var(--color-border)",
  semantic_similarity: "var(--color-accent)",
  explicit_reference: "var(--color-warning)",
  derivation: "var(--color-success)",
};

const REPULSION_STRENGTH = 6000;
const ATTRACTION_STRENGTH = 0.04;
const CENTER_GRAVITY = 0.008;
const DAMPING = 0.8;
const COLLISION_RADIUS = 40;
const ITERATIONS = 100;

function nodeRadius(entityType: string): number {
  return NODE_RADIUS[entityType] ?? NODE_DEFAULT_RADIUS;
}

function nodeColor(entityType: string): string {
  return NODE_COLORS[entityType] ?? "var(--color-accent)";
}

function edgeColor(edgeType: string): string {
  return EDGE_COLORS[edgeType] ?? "var(--color-border)";
}

function runSimulation(nodes: GraphNode[], edges: GraphEdge[], width: number, height: number): PositionedNode[] {
  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(width, height) * 0.35;

  const positioned: PositionedNode[] = nodes.map((n, i) => {
    const angle = (i / nodes.length) * 2 * Math.PI;
    return {
      ...n,
      x: cx + radius * Math.cos(angle) + (Math.random() - 0.5) * 40,
      y: cy + radius * Math.sin(angle) + (Math.random() - 0.5) * 40,
      vx: 0,
      vy: 0,
    };
  });

  const nodeMap = new Map(positioned.map((n) => [`${n.entityType}-${n.entityId}`, n]));

  for (let iter = 0; iter < ITERATIONS; iter++) {
    const alpha = 1 - iter / ITERATIONS;

    for (const node of positioned) {
      let fx = 0;
      let fy = 0;

      for (const other of positioned) {
        if (node === other) continue;
        const dx = node.x - other.x;
        const dy = node.y - other.y;
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const r = nodeRadius(node.entityType) + nodeRadius(other.entityType);
        if (dist < r + COLLISION_RADIUS) {
          const push = (r + COLLISION_RADIUS - dist) * 0.3;
          fx += (dx / dist) * push;
          fy += (dy / dist) * push;
        }
        const force = REPULSION_STRENGTH / (dist * dist);
        fx += (dx / dist) * force * alpha;
        fy += (dy / dist) * force * alpha;
      }

      for (const edge of edges) {
        const srcKey = `${edge.sourceEntityType}-${edge.sourceEntityId}`;
        const tgtKey = `${edge.targetEntityType}-${edge.targetEntityId}`;
        const nodeKey = `${node.entityType}-${node.entityId}`;
        let other: PositionedNode | undefined;
        if (srcKey === nodeKey) other = nodeMap.get(tgtKey);
        if (tgtKey === nodeKey) other = nodeMap.get(srcKey);
        if (!other) continue;
        const dx = other.x - node.x;
        const dy = other.y - node.y;
        fx += dx * ATTRACTION_STRENGTH * edge.weight;
        fy += dy * ATTRACTION_STRENGTH * edge.weight;
      }

      fx += (cx - node.x) * CENTER_GRAVITY;
      fy += (cy - node.y) * CENTER_GRAVITY;

      node.vx = (node.vx + fx) * DAMPING;
      node.vy = (node.vy + fy) * DAMPING;
      node.x += node.vx;
      node.y += node.vy;
    }
  }

  return positioned;
}

function getNeighborIds(node: GraphNode, edges: GraphEdge[]): Set<string> {
  const ids = new Set<string>();
  const nodeKey = `${node.entityType}-${node.entityId}`;
  for (const edge of edges) {
    const srcKey = `${edge.sourceEntityType}-${edge.sourceEntityId}`;
    const tgtKey = `${edge.targetEntityType}-${edge.targetEntityId}`;
    if (srcKey === nodeKey) ids.add(tgtKey);
    if (tgtKey === nodeKey) ids.add(srcKey);
  }
  return ids;
}

function curvePath(
  x1: number, y1: number, x2: number, y2: number, weight: number,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  const curvature = Math.min(weight * 30, 60);
  const cx = (x1 + x2) / 2 + (dy / dist) * curvature;
  const cy = (y1 + y2) / 2 - (dx / dist) * curvature;
  return `M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`;
}

function nodeIcon(type: string): React.ReactNode {
  if (type === "workspace") {
    return <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />;
  }
  if (type === "folder") {
    return <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />;
  }
  if (type === "language") {
    return <><circle cx="12" cy="12" r="10" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></>;
  }
  return <><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" /><polyline points="14 2 14 8 20 8" /></>;
}

interface FocusedSearchResult {
  node: PositionedNode;
  index: number;
}

interface GraphViewProps {
  data: GraphViewType;
  onNodeSelect: (node: GraphNode) => void;
  selectedNodeId?: string;
}

export function GraphView({ data, onNodeSelect, selectedNodeId }: GraphViewProps) {
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [dimensions, setDimensions] = useState({ width: 1200, height: 800 });
  const momentumRef = useRef({ vx: 0, vy: 0 });
  const animRef = useRef<number>(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [showSearch, setShowSearch] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [focusedSearchIndex, setFocusedSearchIndex] = useState(0);
  const [focusMode, setFocusMode] = useState(false);
  const [focusNodeId, setFocusNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      setDimensions({ width: rect.width || 1200, height: rect.height || 800 });
    }
  }, []);

  const layoutNodes = useMemo(
    () => runSimulation(data.nodes, data.edges, dimensions.width, dimensions.height),
    [data.nodes, data.edges, dimensions],
  );

  const nodesMap = useMemo(
    () => new Map(layoutNodes.map((n) => [`${n.entityType}-${n.entityId}`, n])),
    [layoutNodes],
  );

  const selectedNeighbors = useMemo(() => {
    if (!selectedNodeId) return null;
    const selected = data.nodes.find((n) => n.entityId === selectedNodeId);
    if (!selected) return null;
    return getNeighborIds(selected, data.edges);
  }, [selectedNodeId, data.nodes, data.edges]);

  const visibleNodeIds: Set<string> = useMemo(() => {
    if (!focusMode || !focusNodeId) return new Set(data.nodes.map((n) => `${n.entityType}-${n.entityId}`));
    const ids = new Set<string>();
    const focusKey = data.nodes.find((n) => n.entityId === focusNodeId);
    if (!focusKey) return ids;
    const fk = `${focusKey.entityType}-${focusNodeId}`;
    ids.add(fk);
    const neighbors = getNeighborIds(focusKey, data.edges);
    for (const nid of neighbors) ids.add(nid);
    return ids;
  }, [focusMode, focusNodeId, data.nodes, data.edges]);

  const isDimmed = useCallback(
    (node: GraphNode) => {
      if (focusMode && focusNodeId) {
        const key = `${node.entityType}-${node.entityId}`;
        return !visibleNodeIds.has(key);
      }
      if (!selectedNeighbors || !selectedNodeId) return false;
      const key = `${node.entityType}-${node.entityId}`;
      const selected = data.nodes.find((n) => n.entityId === selectedNodeId);
      if (!selected) return false;
      const selectedKey = `${selected.entityType}-${selectedNodeId}`;
      if (key === selectedKey) return false;
      return !selectedNeighbors.has(key);
    },
    [focusMode, focusNodeId, visibleNodeIds, selectedNeighbors, selectedNodeId, data.nodes],
  );

  const searchResults: FocusedSearchResult[] = useMemo(() => {
    if (!searchQuery.trim()) return [];
    const q = searchQuery.toLowerCase();
    return layoutNodes
      .map((n, i) => ({ node: n, index: i }))
      .filter(({ node }) => node.title.toLowerCase().includes(q));
  }, [searchQuery, layoutNodes]);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.92 : 1.08;
    setZoom((prev) => Math.min(Math.max(prev * delta, 0.15), 8));
  }, []);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button === 0) {
        setIsDragging(true);
        setDragStart({ x: e.clientX - offset.x, y: e.clientY - offset.y });
        momentumRef.current = { vx: 0, vy: 0 };
        cancelAnimationFrame(animRef.current);
      }
    },
    [offset],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (isDragging) {
        const dx = e.clientX - dragStart.x - offset.x;
        const dy = e.clientY - dragStart.y - offset.y;
        momentumRef.current = { vx: dx, vy: dy };
        setOffset({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
      }
    },
    [isDragging, dragStart, offset],
  );

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    const m = momentumRef.current;
    if (Math.abs(m.vx) > 2 || Math.abs(m.vy) > 2) {
      const decay = () => {
        momentumRef.current.vx *= 0.92;
        momentumRef.current.vy *= 0.92;
        if (Math.abs(momentumRef.current.vx) < 0.5 && Math.abs(momentumRef.current.vy) < 0.5) return;
        setOffset((prev) => ({
          x: prev.x + momentumRef.current.vx,
          y: prev.y + momentumRef.current.vy,
        }));
        animRef.current = requestAnimationFrame(decay);
      };
      animRef.current = requestAnimationFrame(decay);
    }
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (showSearch) {
        if (e.key === "Escape") {
          setShowSearch(false);
          setSearchQuery("");
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          setFocusedSearchIndex((prev) => Math.min(prev + 1, searchResults.length - 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          setFocusedSearchIndex((prev) => Math.max(prev - 1, 0));
        } else if (e.key === "Enter" && searchResults.length > 0) {
          const result = searchResults[focusedSearchIndex];
          if (result) {
            onNodeSelect(result.node);
            setZoom(2);
            setOffset({
              x: -(result.node.x * 2 - dimensions.width / 2),
              y: -(result.node.y * 2 - dimensions.height / 2),
            });
            setShowSearch(false);
            setSearchQuery("");
          }
        }
      }
    },
    [showSearch, searchResults, focusedSearchIndex, onNodeSelect, dimensions],
  );

  useEffect(() => {
    return () => cancelAnimationFrame(animRef.current);
  }, []);

  useEffect(() => {
    if (showSearch && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [showSearch]);

  const viewportW = dimensions.width;
  const viewportH = dimensions.height;
  const mmScale = 0.15;
  const mmW = viewportW * mmScale;
  const mmH = viewportH * mmScale;

  if (data.nodes.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-10 text-center opacity-50">
        <h3 className="mb-2 text-xl font-semibold">No graph data yet</h3>
        <p className="max-w-xs text-sm">
          Create workspaces and files to build connections. The graph will link related entities automatically.
        </p>
      </div>
    );
  }

  const selectedNode = selectedNodeId ? data.nodes.find((n) => n.entityId === selectedNodeId) : null;

  const toggleFocusMode = useCallback(() => {
    if (selectedNodeId) {
      if (focusMode && focusNodeId === selectedNodeId) {
        setFocusMode(false);
        setFocusNodeId(null);
      } else {
        setFocusMode(true);
        setFocusNodeId(selectedNodeId);
      }
    }
  }, [selectedNodeId, focusMode, focusNodeId]);

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full cursor-grab overflow-hidden bg-(--color-background) active:cursor-grabbing"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      onKeyDown={handleKeyDown}
      tabIndex={0}
    >
      <div
        className="absolute inset-0 transition-transform duration-75 ease-out"
        style={{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${zoom})` }}
      >
        <svg width={dimensions.width} height={dimensions.height} className="overflow-visible">
          <defs>
            <filter id="glow">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
            <filter id="edgeGlow">
              <feGaussianBlur stdDeviation="2" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {data.edges.map((edge) => {
            const source = nodesMap.get(`${edge.sourceEntityType}-${edge.sourceEntityId}`);
            const target = nodesMap.get(`${edge.targetEntityType}-${edge.targetEntityId}`);
            if (!source || !target) return null;

            const isHighlighted =
              selectedNodeId &&
              (edge.sourceEntityId === selectedNodeId || edge.targetEntityId === selectedNodeId);
            const isHovered = hoveredEdge === edge.id;
            const showGlow = isHighlighted || isHovered;
            const ec = edgeColor(edge.edgeType);

            return (
              <g
                key={edge.id}
                onMouseEnter={() => setHoveredEdge(edge.id)}
                onMouseLeave={() => setHoveredEdge(null)}
              >
                {showGlow && (
                  <path
                    d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                    fill="none"
                    stroke={isHighlighted ? nodeColor(edge.sourceEntityType) : ec}
                    strokeWidth={isHighlighted ? Math.max(3, edge.weight * 5) : Math.max(2, edge.weight * 3)}
                    strokeOpacity={isHighlighted ? 0.4 : 0.3}
                    filter="url(#edgeGlow)"
                    className="pointer-events-none"
                  />
                )}
                <path
                  d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                  fill="none"
                  stroke={isHighlighted ? nodeColor(edge.sourceEntityType) : ec}
                  strokeWidth={isHighlighted ? Math.max(1.5, edge.weight * 2.5) : Math.max(0.5, edge.weight * 1.5)}
                  strokeOpacity={isHighlighted ? 0.8 : isHovered ? 0.7 : 0.25}
                  className="pointer-events-none transition-all duration-300 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                />
              </g>
            );
          })}

          {layoutNodes.map((node) => {
            const key = `${node.entityType}-${node.entityId}`;
            const isSelected = selectedNodeId === node.entityId;
            const dimmed = isDimmed(node);
            const r = nodeRadius(node.entityType);
            const col = nodeColor(node.entityType);

            return (
              <g
                key={key}
                transform={`translate(${node.x}, ${node.y})`}
                className="cursor-pointer transition-transform duration-300 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                onClick={(e) => {
                  e.stopPropagation();
                  onNodeSelect(node);
                }}
                onDoubleClick={() => {
                  onNodeSelect(node);
                  setZoom(2.5);
                  setOffset({
                    x: -(node.x * 2.5 - dimensions.width / 2),
                    y: -(node.y * 2.5 - dimensions.height / 2),
                  });
                }}
              >
                <circle
                  r={r}
                  fill={isSelected ? col : dimmed ? "var(--color-background)" : "var(--color-surface)"}
                  stroke={isSelected ? "var(--color-accent-foreground)" : dimmed ? "var(--color-border)" : col}
                  strokeWidth={isSelected ? 3 : dimmed ? 0.5 : 2}
                  className="transition-all duration-500 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                  filter={isSelected ? "url(#glow)" : undefined}
                  style={{ opacity: dimmed ? 0.25 : 1 }}
                />
                <foreignObject x={-r * 0.4} y={-r * 0.4} width={r * 0.8} height={r * 0.8} className="pointer-events-none">
                  <div
                    className="flex h-full w-full items-center justify-center"
                    style={{ opacity: dimmed ? 0.25 : 1 }}
                  >
                    <svg
                      width={r * 0.5}
                      height={r * 0.5}
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke={isSelected ? "var(--color-accent-foreground)" : col}
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      {nodeIcon(node.entityType)}
                    </svg>
                  </div>
                </foreignObject>

                {isSelected && (
                  <circle
                    r={r + 5}
                    fill="none"
                    stroke={col}
                    strokeWidth={1.5}
                    strokeDasharray="4 3"
                    className="animate-[spin_8s_linear_infinite] origin-center"
                    style={{ opacity: 0.5 }}
                  />
                )}

                <text
                  y={r + 14}
                  textAnchor="middle"
                  className="fill-(--color-muted-foreground) text-[9px] font-bold uppercase tracking-tight"
                  style={{ opacity: dimmed ? 0.25 : 1 }}
                >
                  {node.title.length > 14 ? node.title.slice(0, 12) + "\u2026" : node.title}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {selectedNode && (() => {
        const edgeTypeCounts: Record<string, number> = {};
        let strongestEdge: GraphEdge | null = null;
        for (const edge of data.edges) {
          if (edge.sourceEntityId === selectedNode.entityId || edge.targetEntityId === selectedNode.entityId) {
            edgeTypeCounts[edge.edgeType] = (edgeTypeCounts[edge.edgeType] || 0) + 1;
            if (!strongestEdge || edge.weight > strongestEdge.weight) strongestEdge = edge;
          }
        }
        return (
        <div className="absolute left-4 top-4 z-20 w-[260px] animate-fade-in rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-3 shadow-[0_4px_12px_rgba(0,0,0,0.4),0_2px_6px_rgba(0,0,0,0.2)]">
          <div className="flex items-center justify-between gap-2">
            <p className="truncate text-sm font-semibold text-(--color-foreground)">{selectedNode.title}</p>
            <div className="flex items-center gap-1">
              <button
                onClick={toggleFocusMode}
                className={`rounded p-1 transition-colors ${focusMode && focusNodeId === selectedNode.entityId ? "text-(--color-accent) bg-(--color-accent)/10" : "text-(--color-faint-foreground) hover:text-(--color-foreground)"}`}
                title={focusMode && focusNodeId === selectedNode.entityId ? "Exit focus mode" : "Focus on this node"}
              >
                <Focus className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
              <button
                onClick={() => onNodeSelect(selectedNode)}
                className="shrink-0 rounded p-0.5 text-(--color-faint-foreground) hover:text-(--color-foreground)"
                title="Center"
              >
                <Maximize className="h-3 w-3" strokeWidth={1.75} />
              </button>
            </div>
          </div>
          <div className="mt-1.5 flex items-center gap-2 text-xs text-(--color-muted-foreground)">
            <span
              className="rounded px-1.5 py-0.5 font-medium capitalize"
              style={{
                backgroundColor: `${nodeColor(selectedNode.entityType)}20`,
                color: nodeColor(selectedNode.entityType),
              }}
            >
              {selectedNode.entityType}
            </span>
            {selectedNode.workspaceId && (
              <span className="truncate font-(family-name:--font-mono) text-[10px]">
                {selectedNode.workspaceId.slice(0, 8)}
              </span>
            )}
          </div>
          {selectedNeighbors && (
            <p className="mt-1 text-[10px] text-(--color-faint-foreground)">
              {selectedNeighbors.size} connection{selectedNeighbors.size !== 1 ? "s" : ""}
            </p>
          )}

          {Object.keys(edgeTypeCounts).length > 0 && (
            <div className="mt-2 border-t border-(--color-border-subtle) pt-2">
              <p className="mb-1 text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Relationships</p>
              <div className="flex flex-wrap gap-1">
                {Object.entries(edgeTypeCounts).map(([type, count]) => (
                  <span
                    key={type}
                    className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] font-medium"
                    style={{ backgroundColor: `${edgeColor(type)}20`, color: edgeColor(type) }}
                  >
                    {type.split("_")[0]} {count}
                  </span>
                ))}
              </div>
            </div>
          )}

          {strongestEdge && (
            <p className="mt-1 text-[9px] text-(--color-faint-foreground)">
              Strongest: {(strongestEdge.weight * 100).toFixed(0)}% {strongestEdge.edgeType.split("_")[0]}
            </p>
          )}

          <div className="mt-2 flex items-center gap-2 border-t border-(--color-border-subtle) pt-2">
            <button
              onClick={toggleFocusMode}
              className={`flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium transition-colors ${focusMode && focusNodeId === selectedNode.entityId ? "bg-(--color-accent)/10 text-(--color-accent)" : "bg-(--color-surface-hover) text-(--color-muted-foreground) hover:text-(--color-foreground)"}`}
            >
              {focusMode && focusNodeId === selectedNode.entityId ? <Minimize2 className="h-3 w-3" strokeWidth={1.75} /> : <Expand className="h-3 w-3" strokeWidth={1.75} />}
              {focusMode && focusNodeId === selectedNode.entityId ? "Restore" : "Focus"}
            </button>
            <button
              onClick={() => { setShowSearch(true); setSearchQuery(""); }}
              className="flex items-center gap-1 rounded bg-(--color-surface-hover) px-2 py-1 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            >
              <Search className="h-3 w-3" strokeWidth={1.75} />
              Search
            </button>
          </div>
        </div>
        );
      })()}

      {showSearch && (
        <div className="absolute left-1/2 top-4 z-30 w-72 -translate-x-1/2 animate-slide-down">
          <div className="flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) px-3 py-2 shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
            <Search className="h-4 w-4 shrink-0 text-(--color-muted-foreground)" strokeWidth={1.75} />
            <input
              ref={searchInputRef}
              value={searchQuery}
              onChange={(e) => { setSearchQuery(e.target.value); setFocusedSearchIndex(0); }}
              placeholder="Search nodes..."
              className="flex-1 bg-transparent text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:outline-none"
              onKeyDown={(e) => {
                if (e.key === "Escape") { setShowSearch(false); setSearchQuery(""); }
              }}
            />
            {searchQuery && (
              <button
                onClick={() => { setSearchQuery(""); setFocusedSearchIndex(0); }}
                className="rounded p-0.5 text-(--color-faint-foreground) hover:text-(--color-foreground)"
              >
                <X className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
            )}
            <kbd className="hidden rounded border border-(--color-border-subtle) bg-(--color-background) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground) sm:inline">
              ESC
            </kbd>
          </div>
          {searchResults.length > 0 && (
            <div className="mt-1 max-h-48 overflow-y-auto rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) py-1 shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
              {searchResults.map((result, i) => (
                <button
                  key={`${result.node.entityType}-${result.node.entityId}`}
                  onClick={() => {
                    onNodeSelect(result.node);
                    setZoom(2);
                    setOffset({
                      x: -(result.node.x * 2 - dimensions.width / 2),
                      y: -(result.node.y * 2 - dimensions.height / 2),
                    });
                    setShowSearch(false);
                    setSearchQuery("");
                  }}
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm transition-colors ${
                    i === focusedSearchIndex
                      ? "bg-(--color-accent)/10 text-(--color-accent)"
                      : "text-(--color-foreground) hover:bg-(--color-surface-hover)"
                  }`}
                >
                  <span
                    className="h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: nodeColor(result.node.entityType) }}
                  />
                  <span className="truncate">{result.node.title}</span>
                  <span className="ml-auto shrink-0 text-[10px] text-(--color-faint-foreground)">
                    {result.node.entityType}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="absolute bottom-6 right-6 flex flex-col gap-2">
        <button
          onClick={() => setZoom((prev) => prev * 1.3)}
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) hover:shadow-[0_2px_6px_rgba(0,0,0,0.3)] active:scale-95"
          title="Zoom in"
        >
          <ZoomIn className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => setZoom((prev) => prev * 0.77)}
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) hover:shadow-[0_2px_6px_rgba(0,0,0,0.3)] active:scale-95"
          title="Zoom out"
        >
          <ZoomOut className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => {
            setZoom(1);
            setOffset({ x: 0, y: 0 });
            momentumRef.current = { vx: 0, vy: 0 };
          }}
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) hover:shadow-[0_2px_6px_rgba(0,0,0,0.3)] active:scale-95"
          title="Reset view"
        >
          <Maximize className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => setShowSearch((p) => !p)}
          className={`rounded-lg border p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] active:scale-95 ${
            showSearch
              ? "border-(--color-accent) bg-(--color-accent)/10 text-(--color-accent)"
              : "border-(--color-border) bg-(--color-surface) text-(--color-muted-foreground) hover:bg-(--color-surface-hover)"
          }`}
          title="Search"
        >
          <Search className="h-4 w-4" strokeWidth={1.75} />
        </button>
      </div>

      <div className="absolute bottom-6 left-6 flex items-center gap-2 rounded-lg border border-(--color-border) bg-(--color-surface) px-3 py-1.5 text-xs text-(--color-muted-foreground)">
        <span>{data.nodes.length} nodes</span>
        <span className="text-(--color-border-subtle)">|</span>
        <span>{data.edges.length} edges</span>
        <span className="text-(--color-border-subtle)">|</span>
        <span>{Math.round(zoom * 100)}%</span>
      </div>

      <div
        className="absolute bottom-6 left-1/2 z-20 hidden -translate-x-1/2 overflow-hidden rounded-lg border border-(--color-border) bg-(--color-surface) opacity-60 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:opacity-100 xl:block"
        style={{ width: mmW + 16, height: mmH + 16, cursor: "pointer" }}
        onClick={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const nx = (e.clientX - rect.left) / (mmW + 16);
          const ny = (e.clientY - rect.top) / (mmH + 16);
          setOffset({
            x: -(nx * viewportW - viewportW / 2 / zoom),
            y: -(ny * viewportH - viewportH / 2 / zoom),
          });
        }}
      >
        <svg viewBox={`0 0 ${viewportW} ${viewportH}`} width={mmW + 16} height={mmH + 16}>
          {layoutNodes.map((node) => (
            <circle
              key={`${node.entityType}-${node.entityId}`}
              cx={node.x}
              cy={node.y}
              r={Math.max(2, nodeRadius(node.entityType) * mmScale)}
              fill={nodeColor(node.entityType)}
              opacity={0.5}
            />
          ))}
          <rect
            x={-offset.x / zoom}
            y={-offset.y / zoom}
            width={viewportW / zoom}
            height={viewportH / zoom}
            fill="rgba(66,153,225,0.08)"
            stroke="var(--color-accent)"
            strokeWidth={2 / zoom}
            rx={4 / zoom}
            className="transition-all duration-200"
          />
        </svg>
      </div>

      <div className="absolute left-1/2 top-16 z-20 hidden -translate-x-1/2 items-center gap-1.5 rounded-lg border border-(--color-border-subtle) bg-(--color-surface)/80 px-3 py-1 text-[10px] text-(--color-faint-foreground) backdrop-blur-sm xl:flex">
        <Keyboard className="h-3 w-3" strokeWidth={1.75} />
        Scroll to zoom &middot; Drag to pan &middot; Click node to select &middot; {showSearch ? "Type to search" : "Cmd+K to search"}
      </div>
    </div>
  );
}
