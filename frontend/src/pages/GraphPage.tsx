import { useState, useEffect, useCallback } from "react";
import { GraphView } from "@/features/graph/GraphView";
import { GraphSidebar } from "@/features/graph/components/GraphSidebar";
import { GraphStatistics } from "@/features/graph/components/GraphStatistics";
import { RelatedFilesPanel } from "@/features/graph/components/RelatedFilesPanel";
import { getGraphRepository } from "@/services/graphRepository";
import type { GraphView as GraphViewType, GraphNode, GraphEdgeType, GraphStats, NodeDetails } from "@/types/graph";
import { useAppEvents } from "@/hooks/useAppEvents";

export function GraphPage() {
  const [data, setData] = useState<GraphViewType>({ nodes: [], edges: [] });
  const [stats, setStats] = useState<GraphStats | null>(null);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [nodeDetails, setNodeDetails] = useState<NodeDetails | null>(null);
  const [edgeTypes, setEdgeTypes] = useState<GraphEdgeType[]>([
    "co_occurrence",
    "semantic_similarity",
    "explicit_reference",
    "derivation",
  ]);
  const [isLoading, setIsLoading] = useState(true);
  const [isNodeLoading, setIsNodeLoading] = useState(false);
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

  const handleNodeSelect = async (node: GraphNode) => {
    setSelectedNode(node);
    setIsNodeLoading(true);
    try {
      const details = await graphRepo.getNodeDetails(node.entityId, node.entityType);
      setNodeDetails(details);
    } catch (err) {
      console.error("Failed to fetch node details:", err);
    } finally {
      setIsNodeLoading(false);
    }
  };

  const handleSearchNodes = (query: string) => {
    if (!query) {
      fetchData();
      return;
    }
    const filteredNodes = data.nodes.filter(n => n.title.toLowerCase().includes(query.toLowerCase()));
    setData({ ...data, nodes: filteredNodes });
  };

  return (
    <div className="flex h-[calc(100vh-64px)] overflow-hidden">
      <div className="flex-1 flex flex-col min-w-0 bg-background-tertiary">
        <div className="p-8 pb-0">
          <div className="flex items-center justify-between mb-2">
            <div>
              <h1 className="text-3xl font-bold text-foreground">Knowledge Graph</h1>
              <p className="text-muted-foreground">Visualize connections and semantic relationships.</p>
            </div>
          </div>
          <GraphStatistics stats={stats} isLoading={isLoading} />
        </div>

        <div className="flex-1 relative min-h-0 px-8">
          <div className="h-full bg-background-secondary border border-border rounded-2xl overflow-hidden shadow-inner">
            {isLoading ? (
              <div className="h-full flex flex-col items-center justify-center space-y-4">
                <div className="w-16 h-16 border-4 border-primary/20 border-t-primary rounded-full animate-spin" />
                <p className="text-muted-foreground font-medium animate-pulse">Mapping connections...</p>
              </div>
            ) : error ? (
              <div className="h-full flex flex-col items-center justify-center p-10 text-center">
                <div className="bg-destructive/10 p-6 rounded-full mb-4">
                  <span className="text-4xl text-destructive font-bold">!</span>
                </div>
                <h3 className="text-xl font-semibold text-foreground mb-2">{error}</h3>
                <button 
                  onClick={fetchData}
                  className="px-6 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 transition-colors"
                >
                  Retry
                </button>
              </div>
            ) : (
              <GraphView 
                data={data} 
                onNodeSelect={handleNodeSelect} 
                selectedNodeId={selectedNode?.entityId}
              />
            )}
          </div>
          <div className="absolute bottom-12 left-12 right-12">
            <RelatedFilesPanel 
              selectedNode={selectedNode} 
              edges={nodeDetails?.edges ?? []} 
              isLoading={isNodeLoading} 
            />
          </div>
        </div>
      </div>

      <GraphSidebar
        selectedNode={selectedNode}
        nodeDetails={nodeDetails}
        edgeTypes={edgeTypes}
        onEdgeTypesChange={setEdgeTypes}
        onSearch={handleSearchNodes}
      />
    </div>
  );
}
