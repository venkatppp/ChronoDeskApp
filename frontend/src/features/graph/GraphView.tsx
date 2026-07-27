import { useState, useMemo, useRef } from "react";
import type { GraphView as GraphViewType, GraphNode } from "@/types/graph";
import { ZoomIn, ZoomOut, Maximize, Folder, FileText } from "lucide-react";

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

  // Simple radial layout for nodes
  const layoutNodes = useMemo(() => {
    const width = 1200;
    const height = 800;
    const centerX = width / 2;
    const centerY = height / 2;
    const radius = Math.min(width, height) * 0.35;

    return data.nodes.map((node, i) => {
      const angle = (i / data.nodes.length) * 2 * Math.PI;
      return {
        ...node,
        x: centerX + radius * Math.cos(angle),
        y: centerY + radius * Math.sin(angle),
      };
    });
  }, [data.nodes]);

  const nodesMap = useMemo(() => {
    return new Map(layoutNodes.map((n) => [`${n.entityType}-${n.entityId}`, n]));
  }, [layoutNodes]);

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setZoom((prev) => Math.min(Math.max(prev * delta, 0.2), 5));
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 0) {
      setIsDragging(true);
      setDragStart({ x: e.clientX - offset.x, y: e.clientY - offset.y });
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging) {
      setOffset({
        x: e.clientX - dragStart.x,
        y: e.clientY - dragStart.y,
      });
    }
  };

  const handleMouseUp = () => setIsDragging(false);

  if (data.nodes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center p-10 opacity-50">
        <h3 className="text-xl font-semibold mb-2">No graph data yet</h3>
        <p className="text-sm max-w-xs">
          Create workspaces and files to build connections. The Knowledge Graph will automatically link related entities.
        </p>
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      className="relative w-full h-full overflow-hidden bg-background-tertiary cursor-grab active:cursor-grabbing"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    >
      <div 
        className="absolute inset-0 transition-transform duration-75"
        style={{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${zoom})` }}
      >
        <svg width="1200" height="800" className="overflow-visible">
          {/* Render Edges */}
          {data.edges.map((edge) => {
            const source = nodesMap.get(`${edge.sourceEntityType}-${edge.sourceEntityId}`);
            const target = nodesMap.get(`${edge.targetEntityType}-${edge.targetEntityId}`);
            if (!source || !target) return null;

            return (
              <line
                key={edge.id}
                x1={source.x}
                y1={source.y}
                x2={target.x}
                y2={target.y}
                stroke="var(--color-border)"
                strokeWidth={Math.max(1, edge.weight * 5)}
                strokeOpacity={0.4}
                className="transition-all"
              />
            );
          })}

          {/* Render Nodes */}
          {layoutNodes.map((node) => (
            <g 
              key={`${node.entityType}-${node.entityId}`}
              transform={`translate(${node.x}, ${node.y})`}
              className="cursor-pointer group"
              onClick={(e) => {
                e.stopPropagation();
                onNodeSelect(node);
              }}
            >
              <circle
                r="30"
                fill="var(--color-background-secondary)"
                stroke={selectedNodeId === node.entityId ? "var(--color-primary)" : "var(--color-border)"}
                strokeWidth={selectedNodeId === node.entityId ? "3" : "1"}
                className="transition-all group-hover:stroke-primary group-hover:r-[32]"
              />
              <text
                dy="45"
                textAnchor="middle"
                className="text-[10px] font-bold fill-muted-foreground group-hover:fill-foreground pointer-events-none transition-colors uppercase tracking-tight"
              >
                {node.title.length > 15 ? node.title.slice(0, 12) + "..." : node.title}
              </text>
              <foreignObject x="-10" y="-10" width="20" height="20" className="pointer-events-none">
                <div className="w-full h-full flex items-center justify-center text-primary">
                  {node.entityType === "workspace" ? (
                    <Folder className="h-4 w-4" />
                  ) : (
                    <FileText className="h-4 w-4" />
                  )}
                </div>
              </foreignObject>
            </g>
          ))}
        </svg>
      </div>

      {/* Controls */}
      <div className="absolute bottom-6 right-6 flex flex-col gap-2">
        <button onClick={() => setZoom(prev => prev * 1.2)} className="p-2 bg-background-secondary border border-border rounded-lg hover:bg-background-tertiary transition-colors">
          <ZoomIn className="h-5 w-5" />
        </button>
        <button onClick={() => setZoom(prev => prev * 0.8)} className="p-2 bg-background-secondary border border-border rounded-lg hover:bg-background-tertiary transition-colors">
          <ZoomOut className="h-5 w-5" />
        </button>
        <button onClick={() => { setZoom(1); setOffset({ x: 0, y: 0 }); }} className="p-2 bg-background-secondary border border-border rounded-lg hover:bg-background-tertiary transition-colors">
          <Maximize className="h-5 w-5" />
        </button>
      </div>
    </div>
  );
}
