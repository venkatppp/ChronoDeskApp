import { useState, useEffect, useCallback, useMemo } from "react";
import { GraphView } from "@/features/graph/GraphView";
import { getGraphRepository } from "@/services/graphRepository";
import type { GraphView as GraphViewType, GraphNode, GraphEdgeType, GraphStats, NodeDetails } from "@/types/graph";
import type { SearchEntityType } from "@/types/search";
import { useAppEvents } from "@/hooks/useAppEvents";
import { Focus, Network } from "lucide-react";

export function GraphPage() {
  const [data, setData] = useState<GraphViewType>({ nodes: [], edges: [] });
  const [stats, setStats] = useState<GraphStats | null>(null);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [nodeDetails, setNodeDetails] = useState<NodeDetails | null>(null);
  const [edgeTypes] = useState<GraphEdgeType[]>([
    "co_occurrence",
    "semantic_similarity",
    "explicit_reference",
    "derivation",
  ]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const graphRepo = getGraphRepository();

  const fetchData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [view, graphStats] = await Promise.all([
        graphRepo.getGraph(undefined, edgeTypes),
        graphRepo.getGraphStats(),
      ]);
      setData(view);
      setStats(graphStats);
    } catch (err) {
      console.error("Failed to fetch graph data:", err);
      setError("Failed to load Knowledge Graph. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [graphRepo, edgeTypes]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useAppEvents(["graph:edge_added", "workspace:indexed"], () => {
    fetchData();
  });

  const handleNodeSelect = useCallback(async (node: GraphNode) => {
    setSelectedNode(node);
    try {
      const details = await graphRepo.getNodeDetails(node.entityId, node.entityType as SearchEntityType);
      setNodeDetails(details);
    } catch (err) {
      console.error("Failed to fetch node details:", err);
    }
  }, [graphRepo]);

  const filteredData = useMemo(() => {
    if (!selectedNode) return data;
    return data;
  }, [data, selectedNode]);

  const connectionCount = useMemo(() => {
    if (!nodeDetails) return 0;
    return nodeDetails.edges.length;
  }, [nodeDetails]);

  return (
    <div className="mx-auto flex h-[calc(100vh-64px)] flex-col overflow-hidden">
      <div className="flex shrink-0 items-center justify-between border-b border-(--color-border-subtle) px-6 py-4">
        <div>
          <h1 className="font-(family-name:--font-display) text-xl font-bold">Knowledge Graph</h1>
          <p className="text-sm text-(--color-muted-foreground)">
            Visualize connections and semantic relationships across all workspaces.
          </p>
        </div>
        {stats && !isLoading && (
          <div className="flex items-center gap-4 text-xs text-(--color-muted-foreground)">
            <span className="inline-flex items-center gap-1">
              <Network className="h-3.5 w-3.5" strokeWidth={1.75} />
              {stats.nodeCount} nodes
            </span>
            <span className="text-(--color-border-subtle)">|</span>
            <span>{stats.edgeCount} edges</span>
            <span className="text-(--color-border-subtle)">|</span>
            <span>{(stats.density * 100).toFixed(1)}% density</span>
          </div>
        )}
      </div>

      <div className="flex flex-1 gap-0 overflow-hidden">
        <div className="flex flex-1 flex-col overflow-hidden">
          {isLoading ? (
            <div className="flex flex-1 items-center justify-center">
              <div className="flex flex-col items-center gap-3">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-(--color-border) border-t-(--color-accent)" />
                <p className="text-sm text-(--color-muted-foreground)">Loading graph...</p>
              </div>
            </div>
          ) : error ? (
            <div className="flex flex-1 items-center justify-center">
              <div className="flex flex-col items-center gap-3">
                <p className="text-sm text-(--color-danger)">{error}</p>
                <button
                  onClick={fetchData}
                  className="rounded-[var(--radius-control)] bg-(--color-accent) px-4 py-2 text-sm font-medium text-(--color-accent-foreground)"
                >
                  Retry
                </button>
              </div>
            </div>
          ) : (
            <GraphView
              data={filteredData}
              onNodeSelect={handleNodeSelect}
              selectedNodeId={selectedNode?.entityId}
            />
          )}
        </div>

        <div className="w-72 shrink-0 border-l border-(--color-border-subtle) bg-(--color-surface) p-4 overflow-y-auto hidden lg:block">
          {selectedNode && nodeDetails ? (
            <div className="flex flex-col gap-4 animate-fade-in">
              <div className="flex items-center gap-2">
                <div className="flex h-8 w-8 items-center justify-center rounded-lg" style={{ backgroundColor: `${nodeColor(selectedNode.entityType)}20` }}>
                  <Focus className="h-4 w-4" style={{ color: nodeColor(selectedNode.entityType) }} strokeWidth={1.75} />
                </div>
                <div className="min-w-0">
                  <p className="truncate text-sm font-semibold text-(--color-foreground)">{selectedNode.title}</p>
                  <p className="text-[10px] font-medium uppercase tracking-wide" style={{ color: nodeColor(selectedNode.entityType) }}>
                    {selectedNode.entityType}
                  </p>
                </div>
              </div>

              <div className="space-y-2">
                <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Connections</p>
                <div className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2">
                  <p className="text-2xl font-bold text-(--color-foreground)">{connectionCount}</p>
                  <p className="text-[10px] text-(--color-muted-foreground)">{connectionCount === 1 ? "edge" : "edges"}</p>
                </div>
              </div>

              {nodeDetails.edges.length > 0 && (
                <div className="space-y-2">
                  <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Related nodes</p>
                  <div className="flex flex-col gap-1">
                    {nodeDetails.edges.slice(0, 8).map((edge) => {
                      const relId = edge.targetEntityId === selectedNode.entityId ? edge.sourceEntityId : edge.targetEntityId;
                      const relType = edge.targetEntityId === selectedNode.entityId ? edge.sourceEntityType : edge.targetEntityType;
                      return (
                        <div key={edge.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5 transition-colors hover:bg-(--color-surface-hover)">
                          <span className="h-1.5 w-1.5 shrink-0 rounded-full" style={{ backgroundColor: nodeColor(relType) }} />
                          <span className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{relId.split("/").pop()}</span>
                          <span className="shrink-0 text-[9px] text-(--color-faint-foreground)">{edge.edgeType.split("_")[0]}</span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="flex h-full flex-col items-center justify-center text-center opacity-40">
              <Network className="mb-3 h-8 w-8" strokeWidth={1.5} />
              <p className="text-xs">Select a node to see details.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function nodeColor(entityType: string): string {
  const colors: Record<string, string> = {
    workspace: "var(--color-accent)",
    folder: "var(--color-warning)",
    file: "var(--color-success)",
    language: "var(--color-danger)",
    project: "var(--color-accent)",
  };
  return colors[entityType] ?? "var(--color-accent)";
}
