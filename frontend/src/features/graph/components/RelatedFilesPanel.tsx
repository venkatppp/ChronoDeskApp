import { FileText, ArrowRight } from "lucide-react";
import type { GraphEdge, GraphNode } from "@/types/graph";

interface RelatedFilesPanelProps {
  selectedNode: GraphNode | null;
  edges: GraphEdge[];
  isLoading: boolean;
}

export function RelatedFilesPanel({
  selectedNode,
  edges,
  isLoading,
}: RelatedFilesPanelProps) {
  if (!selectedNode) return null;

  if (isLoading) {
    return (
      <div className="mt-8 space-y-3">
        <div className="h-6 w-32 bg-background-secondary rounded animate-pulse" />
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-20 bg-background-secondary border border-border rounded-xl animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  const relatedNodes = edges.map(edge => ({
    id: edge.targetEntityId === selectedNode.entityId ? edge.sourceEntityId : edge.targetEntityId,
    type: edge.targetEntityId === selectedNode.entityId ? edge.sourceEntityType : edge.targetEntityType,
    edgeType: edge.edgeType,
    weight: edge.weight
  })).filter(n => n.type === "file");

  if (relatedNodes.length === 0) return null;

  return (
    <div className="mt-8 animate-in fade-in slide-in-from-bottom-4">
      <h3 className="text-sm font-bold text-muted-foreground uppercase tracking-widest mb-4 flex items-center gap-2">
        <FileText className="h-4 w-4" />
        Related Files
      </h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
        {relatedNodes.map((node) => (
          <div 
            key={node.id} 
            className="p-4 bg-background-secondary border border-border rounded-xl hover:border-accent/40 transition-all flex flex-col justify-between group cursor-pointer"
          >
            <div className="flex items-start justify-between mb-2">
              <span className="text-sm font-semibold text-foreground truncate max-w-[150px]">{node.id.split("/").pop()}</span>
              <span className={`px-1.5 py-0.5 rounded text-[8px] font-bold uppercase ${
                node.edgeType === "semantic_similarity" ? "bg-accent-muted/10 text-accent-muted" : "bg-accent/10 text-accent"
              }`}>
                {node.edgeType.split("_")[0]}
              </span>
            </div>
            <div className="flex items-center justify-between mt-auto">
              <div className="flex items-center gap-1">
                <div className="w-16 h-1 bg-background-tertiary rounded-full overflow-hidden">
                  <div className="h-full bg-accent" style={{ width: `${node.weight * 100}%` }} />
                </div>
                <span className="text-[9px] text-muted-foreground font-mono">{(node.weight * 100).toFixed(0)}%</span>
              </div>
              <ArrowRight className="h-3 w-3 text-muted-foreground group-hover:text-accent transition-colors translate-x-0 group-hover:translate-x-1" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
