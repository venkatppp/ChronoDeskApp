import { Search, Info, Share2, Filter, Folder, FileText } from "lucide-react";
import type { GraphNode, GraphEdgeType, NodeDetails } from "@/types/graph";

interface GraphSidebarProps {
  selectedNode: GraphNode | null;
  nodeDetails: NodeDetails | null;
  edgeTypes: GraphEdgeType[];
  onEdgeTypesChange: (types: GraphEdgeType[]) => void;
  onSearch: (query: string) => void;
}

export function GraphSidebar({
  selectedNode,
  nodeDetails,
  edgeTypes,
  onEdgeTypesChange,
  onSearch,
}: GraphSidebarProps) {
  const EDGE_TYPES: { type: GraphEdgeType; label: string; color: string }[] = [
    { type: "co_occurrence", label: "Co-occurrence", color: "bg-accent" },
    { type: "semantic_similarity", label: "Semantic", color: "bg-accent-muted" },
    { type: "explicit_reference", label: "Reference", color: "bg-warning" },
    { type: "derivation", label: "Derivation", color: "bg-success" },
  ];

  const toggleEdgeType = (type: GraphEdgeType) => {
    if (edgeTypes.includes(type)) {
      onEdgeTypesChange(edgeTypes.filter((t) => t !== type));
    } else {
      onEdgeTypesChange([...edgeTypes, type]);
    }
  };

  return (
    <div className="w-80 border-l border-border bg-background flex flex-col h-full overflow-hidden">
      <div className="p-6 border-b border-border">
        <div className="relative mb-6">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search nodes..."
            onChange={(e) => onSearch(e.target.value)}
            className="w-full h-10 pl-10 pr-4 bg-background-secondary border border-border rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent transition-all"
          />
        </div>

        <div className="space-y-4">
          <div className="flex items-center gap-2 text-xs font-bold text-muted-foreground uppercase tracking-widest">
            <Filter className="h-3 w-3" />
            Connections
          </div>
          <div className="space-y-2">
            {EDGE_TYPES.map((et) => (
              <label key={et.type} className="flex items-center gap-3 cursor-pointer group">
                <input
                  type="checkbox"
                  checked={edgeTypes.includes(et.type)}
                  onChange={() => toggleEdgeType(et.type)}
                  className="w-4 h-4 rounded border-border bg-background-secondary text-accent focus:ring-accent"
                />
                <div className={`w-2 h-2 rounded-full ${et.color}`} />
                <span className="text-sm font-medium text-muted-foreground group-hover:text-foreground transition-colors">
                  {et.label}
                </span>
              </label>
            ))}
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        {selectedNode ? (
          <div className="animate-in fade-in slide-in-from-right-4">
            <div className="flex items-start justify-between mb-6">
              <div className={`p-3 rounded-xl ${
                selectedNode.entityType === "workspace" ? "bg-accent/10 text-accent" : "bg-accent-muted/10 text-accent-muted"
              }`}>
                {selectedNode.entityType === "workspace" ? (
                  <Folder className="h-6 w-6" />
                ) : (
                  <FileText className="h-6 w-6" />
                )}
              </div>
              <button className="p-2 text-muted-foreground hover:text-foreground transition-colors">
                <Share2 className="h-4 w-4" />
              </button>
            </div>
            
            <h2 className="text-xl font-bold text-foreground mb-1">{selectedNode.title}</h2>
            <div className="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-background-tertiary border border-border text-muted-foreground mb-6">
              {selectedNode.entityType}
            </div>

            <div className="space-y-6">
              <div>
                <div className="flex items-center gap-2 text-xs font-bold text-muted-foreground uppercase tracking-widest mb-3">
                  <Info className="h-3 w-3" />
                  Stats
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div className="p-3 bg-background-secondary border border-border rounded-xl">
                    <div className="text-[10px] text-muted-foreground font-medium uppercase mb-1">Connections</div>
                    <div className="text-xl font-bold text-foreground">{nodeDetails?.edges.length ?? 0}</div>
                  </div>
                  <div className="p-3 bg-background-secondary border border-border rounded-xl">
                    <div className="text-[10px] text-muted-foreground font-medium uppercase mb-1">Workspace ID</div>
                    <div className="text-xs font-mono text-foreground truncate">{selectedNode.workspaceId.slice(0, 8)}...</div>
                  </div>
                </div>
              </div>

              {nodeDetails && nodeDetails.edges.length > 0 && (
                <div>
                  <div className="flex items-center gap-2 text-xs font-bold text-muted-foreground uppercase tracking-widest mb-3">
                    Connected To
                  </div>
                  <div className="space-y-2">
                    {nodeDetails.edges.slice(0, 5).map((edge) => (
                      <div key={edge.id} className="p-3 bg-background-secondary border border-border rounded-xl flex items-center justify-between text-xs">
                        <span className="text-muted-foreground truncate max-w-[120px]">
                          {edge.targetEntityId === selectedNode.entityId ? edge.sourceEntityId.slice(0, 10) : edge.targetEntityId.slice(0, 10)}...
                        </span>
                        <span className="px-2 py-0.5 bg-background-tertiary rounded border border-border text-[9px] font-bold uppercase opacity-70">
                          {edge.edgeType.split("_")[0]}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center text-center opacity-40 italic">
            <Info className="h-10 w-10 mb-4" />
            <p className="text-sm">Select a node to view its connections and details.</p>
          </div>
        )}
      </div>
    </div>
  );
}
