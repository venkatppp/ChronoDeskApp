import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import type { KgNode, KgEdge, GraphNodeType, GraphRelationshipType } from "@/types/graph";
import {
  ZoomIn,
  ZoomOut,
  Maximize,
  Focus,
  Search,
  X,
  Keyboard,
  Minimize2,
  Expand,
} from "lucide-react";

interface PositionedNode extends KgNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
}

const NODE_RADIUS: Record<GraphNodeType, number> = {
  workspace: 34,
  file: 24,
  planner_report: 22,
  execution: 26,
  memory_record: 24,
  autonomous_session: 28,
};

const NODE_COLORS: Record<GraphNodeType, string> = {
  workspace: "var(--color-accent)",
  file: "var(--color-success)",
  planner_report: "var(--color-warning)",
  execution: "var(--color-danger)",
  memory_record: "var(--color-accent-muted)",
  autonomous_session: "var(--color-warning-foreground)",
};

const EDGE_COLORS: Record<GraphRelationshipType, string> = {
  contains: "var(--color-border)",
  runs_in: "var(--color-accent)",
  reports_on: "var(--color-warning)",
  derived_from: "var(--color-success)",
  related_to: "var(--color-danger)",
};

const REPULSION_STRENGTH = 6000;
const ATTRACTION_STRENGTH = 0.04;
const CENTER_GRAVITY = 0.008;
const DAMPING = 0.8;
const COLLISION_RADIUS = 40;
const ITERATIONS = 100;

function nodeRadius(type: GraphNodeType): number {
  return NODE_RADIUS[type] ?? 24;
}

function nodeColor(type: GraphNodeType): string {
  return NODE_COLORS[type] ?? "var(--color-accent)";
}

function edgeColor(type: GraphRelationshipType): string {
  return EDGE_COLORS[type] ?? "var(--color-border)";
}

function runSimulation(nodes: KgNode[], edges: KgEdge[], width: number, height: number): PositionedNode[] {
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

  const nodeMap = new Map(positioned.map((n) => [`${n.nodeType}-${n.entityId}`, n]));

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
        const r = nodeRadius(node.nodeType) + nodeRadius(other.nodeType);
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
        const srcKey = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
        const tgtKey = `${edge.targetNodeType}-${edge.targetEntityId}`;
        const nodeKey = `${node.nodeType}-${node.entityId}`;
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

function getNeighborIds(node: KgNode, edges: KgEdge[]): Set<string> {
  const ids = new Set<string>();
  const nodeKey = `${node.nodeType}-${node.entityId}`;
  for (const edge of edges) {
    const srcKey = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
    const tgtKey = `${edge.targetNodeType}-${edge.targetEntityId}`;
    if (srcKey === nodeKey) ids.add(tgtKey);
    if (tgtKey === nodeKey) ids.add(srcKey);
  }
  return ids;
}

function curvePath(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  weight: number,
): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  const curvature = Math.min(weight * 30, 60);
  const cx = (x1 + x2) / 2 + (dy / dist) * curvature;
  const cy = (y1 + y2) / 2 - (dx / dist) * curvature;
  return `M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`;
}

function nodeIcon(type: GraphNodeType): React.ReactNode {
  switch (type) {
    case "workspace":
      return <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />;
    case "file":
      return <><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" /><polyline points="14 2 14 8 20 8" /></>;
    case "planner_report":
      return <><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" /><polyline points="14 2 14 8 20 8" /><line x1="9" y1="13" x2="15" y2="13" /><line x1="9" y1="17" x2="13" y2="17" /></>;
    case "execution":
      return <><circle cx="12" cy="12" r="10" /><polyline points="12 6 12 12 16 14" /></>;
    case "memory_record":
      return <><path d="M21 8V21H3V8" /><path d="M1 3h22v5H1z" /><path d="M10 12h4" /></>;
    default:
      return <><circle cx="12" cy="12" r="10" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></>;
  }
}

interface KnowledgeGraphViewProps {
  nodes: KgNode[];
  edges: KgEdge[];
  onNodeSelect: (node: KgNode) => void;
  selectedNodeId?: string;
  emptyMessage?: string;
  /** Total node count for progressive loading (RC-8 M4). */
  totalHint?: number;
  /** Fetches the next progressive page (RC-8 M4). */
  onLoadMore?: () => void;
}

export function KnowledgeGraphView({
  nodes,
  edges,
  onNodeSelect,
  selectedNodeId,
  emptyMessage,
  totalHint,
  onLoadMore,
}: KnowledgeGraphViewProps) {
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [dimensions, setDimensions] = useState({ width: 1200, height: 800 });
  const momentumRef = useRef({ vx: 0, vy: 0 });
  const animRef = useRef<number>(0);
  const [showSearch, setShowSearch] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [focusedSearchIndex, setFocusedSearchIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [focusMode, setFocusMode] = useState(false);
  const [focusNodeId, setFocusNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      setDimensions({ width: rect.width || 1200, height: rect.height || 800 });
    }
  }, []);

  const layoutNodes = useMemo(
    () => runSimulation(nodes, edges, dimensions.width, dimensions.height),
    [nodes, edges, dimensions],
  );

  const nodesMap = useMemo(
    () => new Map(layoutNodes.map((n) => [`${n.nodeType}-${n.entityId}`, n])),
    [layoutNodes],
  );

  const selectedNeighbors = useMemo(() => {
    if (!selectedNodeId) return null;
    const selected = nodes.find((n) => n.entityId === selectedNodeId);
    if (!selected) return null;
    return getNeighborIds(selected, edges);
  }, [selectedNodeId, nodes, edges]);

  const visibleNodeIds: Set<string> = useMemo(() => {
    if (!focusMode || !focusNodeId) return new Set(nodes.map((n) => `${n.nodeType}-${n.entityId}`));
    const ids = new Set<string>();
    const focusKey = nodes.find((n) => n.entityId === focusNodeId);
    if (!focusKey) return ids;
    const fk = `${focusKey.nodeType}-${focusNodeId}`;
    ids.add(fk);
    const neighbors = getNeighborIds(focusKey, edges);
    for (const nid of neighbors) ids.add(nid);
    return ids;
  }, [focusMode, focusNodeId, nodes, edges]);

  const isDimmed = useCallback(
    (node: KgNode) => {
      if (focusMode && focusNodeId) {
        const key = `${node.nodeType}-${node.entityId}`;
        return !visibleNodeIds.has(key);
      }
      if (!selectedNeighbors || !selectedNodeId) return false;
      const key = `${node.nodeType}-${node.entityId}`;
      const selected = nodes.find((n) => n.entityId === selectedNodeId);
      if (!selected) return false;
      const selectedKey = `${selected.nodeType}-${selectedNodeId}`;
      if (key === selectedKey) return false;
      return !selectedNeighbors.has(key);
    },
    [focusMode, focusNodeId, visibleNodeIds, selectedNeighbors, selectedNodeId, nodes],
  );

  const searchResults = useMemo(() => {
    if (!searchQuery.trim()) return [] as PositionedNode[];
    const q = searchQuery.toLowerCase();
    return layoutNodes.filter((n) => n.title.toLowerCase().includes(q));
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
            onNodeSelect(result);
            setZoom(2);
            setOffset({
              x: -(result.x * 2 - dimensions.width / 2),
              y: -(result.y * 2 - dimensions.height / 2),
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

  const jumpTo = useCallback(
    (node: PositionedNode) => {
      onNodeSelect(node);
      setZoom(2);
      setOffset({ x: -(node.x * 2 - dimensions.width / 2), y: -(node.y * 2 - dimensions.height / 2) });
    },
    [onNodeSelect, dimensions],
  );

  if (nodes.length === 0) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-10 text-center opacity-50">
        <h3 className="mb-2 text-xl font-semibold">No knowledge graph data yet</h3>
        <p className="max-w-xs text-sm">
          {emptyMessage ??
            "Workspaces, files, planner reports, executions, memory records, and autonomous sessions become graph nodes automatically."}
        </p>
      </div>
    );
  }

  const selectedNode = selectedNodeId ? nodes.find((n) => n.entityId === selectedNodeId) : null;

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
            <filter id="kg-glow">
              <feGaussianBlur stdDeviation="3" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
            <filter id="kg-edgeGlow">
              <feGaussianBlur stdDeviation="2" result="blur" />
              <feMerge>
                <feMergeNode in="blur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {edges.map((edge) => {
            const source = nodesMap.get(`${edge.sourceNodeType}-${edge.sourceEntityId}`);
            const target = nodesMap.get(`${edge.targetNodeType}-${edge.targetEntityId}`);
            if (!source || !target) return null;

            const isHighlighted =
              selectedNodeId &&
              (edge.sourceEntityId === selectedNodeId || edge.targetEntityId === selectedNodeId);
            const isHovered = hoveredEdge === edge.id;
            const showGlow = isHighlighted || isHovered;
            const ec = edgeColor(edge.relationshipType);
            const isSemantic = edge.relationshipType === "related_to";
            const confidenceOpacity = isSemantic ? 0.2 + edge.confidence * 0.8 : 1;

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
                    stroke={ec}
                    strokeWidth={Math.max(3, edge.weight * 5)}
                    strokeOpacity={0.3}
                    filter="url(#kg-edgeGlow)"
                    className="pointer-events-none"
                  />
                )}
                <path
                  d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                  fill="none"
                  stroke={isHighlighted ? nodeColor(source.nodeType) : ec}
                  strokeWidth={isHighlighted ? Math.max(1.5, edge.weight * 2.5) : Math.max(0.5, edge.weight * 1.5)}
                  strokeOpacity={isHighlighted ? 0.8 * confidenceOpacity : isHovered ? 0.7 * confidenceOpacity : 0.25 * confidenceOpacity}
                  className="pointer-events-none transition-all duration-300 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                >
                  {isSemantic && (
                    <title>{`${edge.relationshipType} · confidence ${(edge.confidence * 100).toFixed(0)}%`}</title>
                  )}
                </path>
              </g>
            );
          })}

          {layoutNodes.map((node) => {
            const key = `${node.nodeType}-${node.entityId}`;
            const isSelected = selectedNodeId === node.entityId;
            const dimmed = isDimmed(node);
            const r = nodeRadius(node.nodeType);
            const col = nodeColor(node.nodeType);

            return (
              <g
                key={key}
                transform={`translate(${node.x}, ${node.y})`}
                className="cursor-pointer transition-transform duration-300 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                onClick={(e) => {
                  e.stopPropagation();
                  onNodeSelect(node);
                }}
                onDoubleClick={() => jumpTo(node)}
              >
                <circle
                  r={r}
                  fill={isSelected ? col : dimmed ? "var(--color-background)" : "var(--color-surface)"}
                  stroke={isSelected ? "var(--color-accent-foreground)" : dimmed ? "var(--color-border)" : col}
                  strokeWidth={isSelected ? 3 : dimmed ? 0.5 : 2}
                  className="transition-all duration-500 ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                  filter={isSelected ? "url(#kg-glow)" : undefined}
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
                      {nodeIcon(node.nodeType)}
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
        const relationshipCounts: Record<string, number> = {};
        for (const edge of edges) {
          if (edge.sourceEntityId === selectedNode.entityId || edge.targetEntityId === selectedNode.entityId) {
            relationshipCounts[edge.relationshipType] =
              (relationshipCounts[edge.relationshipType] || 0) + 1;
          }
        }
        return (
          <div className="absolute left-4 top-4 z-20 w-[260px] animate-fade-in rounded-[var(--radius-card)] border border-(--color-border) bg-(--color-surface-raised) p-3 shadow-[0_4px_12px_rgba(0,0,0,0.4),0_2px_6px_rgba(0,0,0,0.2)]">
            <div className="flex items-center justify-between gap-2">
              <p className="truncate text-sm font-semibold text-(--color-foreground)">{selectedNode.title}</p>
              <button
                onClick={toggleFocusMode}
                className={`rounded p-1 transition-colors ${focusMode && focusNodeId === selectedNode.entityId ? "text-(--color-accent) bg-(--color-accent)/10" : "text-(--color-faint-foreground) hover:text-(--color-foreground)"}`}
                title="Focus on this node"
              >
                <Focus className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
            </div>
            <div className="mt-1.5 flex items-center gap-2 text-xs text-(--color-muted-foreground)">
              <span
                className="rounded px-1.5 py-0.5 font-medium capitalize"
                style={{ backgroundColor: `${nodeColor(selectedNode.nodeType)}20`, color: nodeColor(selectedNode.nodeType) }}
              >
                {selectedNode.nodeType.replace("_", " ")}
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
            {Object.keys(relationshipCounts).length > 0 && (
              <div className="mt-2 border-t border-(--color-border-subtle) pt-2">
                <p className="mb-1 text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Relationships</p>
                <div className="flex flex-wrap gap-1">
                  {Object.entries(relationshipCounts).map(([type, count]) => (
                    <span
                      key={type}
                      className="inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] font-medium"
                      style={{ backgroundColor: `${edgeColor(type as GraphRelationshipType)}20`, color: edgeColor(type as GraphRelationshipType) }}
                    >
                      {type.replace("_", " ")} {count}
                    </span>
                  ))}
                </div>
              </div>
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
              placeholder="Search visible nodes..."
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
              {searchResults.map((node, i) => (
                <button
                  key={`${node.nodeType}-${node.entityId}`}
                  onClick={() => { jumpTo(node); setShowSearch(false); setSearchQuery(""); }}
                  className={`flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm transition-colors ${
                    i === focusedSearchIndex
                      ? "bg-(--color-accent)/10 text-(--color-accent)"
                      : "text-(--color-foreground) hover:bg-(--color-surface-hover)"
                  }`}
                >
                  <span
                    className="h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: nodeColor(node.nodeType) }}
                  />
                  <span className="truncate">{node.title}</span>
                  <span className="ml-auto shrink-0 text-[10px] text-(--color-faint-foreground)">
                    {node.nodeType.replace("_", " ")}
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
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) active:scale-95"
          title="Zoom in"
        >
          <ZoomIn className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => setZoom((prev) => prev * 0.77)}
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) active:scale-95"
          title="Zoom out"
        >
          <ZoomOut className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => { setZoom(1); setOffset({ x: 0, y: 0 }); momentumRef.current = { vx: 0, vy: 0 }; }}
          className="rounded-lg border border-(--color-border) bg-(--color-surface) p-2 transition-all duration-200 ease-[cubic-bezier(0.32,0.08,0.24,1)] hover:bg-(--color-surface-hover) active:scale-95"
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
        <span>{nodes.length} nodes</span>
        <span className="text-(--color-border-subtle)">|</span>
        <span>{edges.length} edges</span>
        <span className="text-(--color-border-subtle)">|</span>
        <span>{Math.round(zoom * 100)}%</span>
      </div>

      {onLoadMore && totalHint != null && nodes.length < totalHint && (
        <div className="absolute bottom-6 left-1/2 z-20 -translate-x-1/2">
          <button
            onClick={onLoadMore}
            className="flex items-center gap-2 rounded-lg border border-(--color-border) bg-(--color-surface-raised) px-3 py-1.5 text-xs font-medium text-(--color-muted-foreground) shadow-[0_4px_12px_rgba(0,0,0,0.3)] transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          >
            <span className="h-1.5 w-1.5 rounded-full bg-(--color-accent)" />
            {nodes.length} of {totalHint} nodes loaded — load more
          </button>
        </div>
      )}

      <div className="absolute left-1/2 top-16 z-20 hidden -translate-x-1/2 items-center gap-1.5 rounded-lg border border-(--color-border-subtle) bg-(--color-surface)/80 px-3 py-1 text-[10px] text-(--color-faint-foreground) backdrop-blur-sm xl:flex">
        <Keyboard className="h-3 w-3" strokeWidth={1.75} />
        Scroll to zoom &middot; Drag to pan &middot; Click node to explore
      </div>
    </div>
  );
}
