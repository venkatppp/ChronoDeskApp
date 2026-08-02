import { useState, useEffect, useCallback, useMemo } from "react";
import { KnowledgeGraphView } from "@/features/graph/KnowledgeGraphView";
import { getGraphRepository } from "@/services/graphRepository";
import type {
  KgNode,
  KgEdge,
  GraphNodeType,
  GraphSyncSummary,
  KgStats,
  ContextDiscovery,
} from "@/types/graph";
import { useAppEvents } from "@/hooks/useAppEvents";
import { Network, RefreshCw, Search, X, Map, ChevronLeft } from "lucide-react";

const NODE_TYPE_FILTERS: { value: GraphNodeType | "all"; label: string }[] = [
  { value: "all", label: "All" },
  { value: "workspace", label: "Workspaces" },
  { value: "file", label: "Files" },
  { value: "planner_report", label: "Planner Reports" },
  { value: "execution", label: "Executions" },
  { value: "memory_record", label: "Memory" },
  { value: "autonomous_session", label: "Sessions" },
];

const TYPE_COLORS: Record<GraphNodeType, string> = {
  workspace: "var(--color-accent)",
  file: "var(--color-success)",
  planner_report: "var(--color-warning)",
  execution: "var(--color-danger)",
  memory_record: "var(--color-accent-muted)",
  autonomous_session: "var(--color-warning-foreground)",
};

export function GraphPage() {
  const [stats, setStats] = useState<KgStats | null>(null);
  const [nodes, setNodes] = useState<KgNode[]>([]);
  const [edges, setEdges] = useState<KgEdge[]>([]);
  const [exploring, setExploring] = useState(false);
  const [selectedNode, setSelectedNode] = useState<KgNode | null>(null);
  const [context, setContext] = useState<ContextDiscovery | null>(null);
  const [activeFilter, setActiveFilter] = useState<GraphNodeType | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<KgNode[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSyncing, setIsSyncing] = useState(false);
  const [lastSync, setLastSync] = useState<GraphSyncSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const graphRepo = getGraphRepository();

  const fetchAllNodes = useCallback(async (filter: GraphNodeType | "all" = activeFilter) => {
    setIsLoading(true);
    setError(null);
    try {
      const [allNodes, graphStats] = await Promise.all([
        graphRepo.graphNodes(
          filter === "all" ? ["workspace", "file", "planner_report", "execution", "memory_record", "autonomous_session"] : [filter],
          undefined,
          400,
        ),
        graphRepo.graphKgStats(),
      ]);
      setNodes(allNodes);
      setEdges([]);
      setExploring(false);
      setStats(graphStats);
    } catch (err) {
      console.error("Failed to fetch knowledge graph:", err);
      setError("Failed to load Knowledge Graph. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [graphRepo, activeFilter]);

  useEffect(() => {
    fetchAllNodes();
  }, [fetchAllNodes]);

  useAppEvents(["graph:edge_added", "workspace:indexed"], () => {
    fetchAllNodes();
  });

  const handleFilterChange = useCallback(
    (filter: GraphNodeType | "all") => {
      setActiveFilter(filter);
      fetchAllNodes(filter);
    },
    [fetchAllNodes],
  );

  const handleNodeSelect = useCallback(
    async (node: KgNode) => {
      setSelectedNode(node);
      setIsLoading(true);
      try {
        const [subgraph, discovery] = await Promise.all([
          graphRepo.graphSubgraph(node.nodeType, node.entityId, 2),
          graphRepo.graphContext(node.nodeType, node.entityId, 30),
        ]);
        setNodes(subgraph.nodes);
        setEdges(subgraph.edges);
        setExploring(true);
        setContext(discovery);
      } catch (err) {
        console.error("Failed to explore node:", err);
        setError("Failed to explore this node.");
      } finally {
        setIsLoading(false);
      }
    },
    [graphRepo],
  );

  const handleSync = useCallback(async () => {
    setIsSyncing(true);
    try {
      const summary = await graphRepo.syncGraph();
      setLastSync(summary);
      await fetchAllNodes(activeFilter);
    } catch (err) {
      console.error("Failed to sync knowledge graph:", err);
      setError("Failed to rebuild the Knowledge Graph.");
    } finally {
      setIsSyncing(false);
    }
  }, [graphRepo, fetchAllNodes, activeFilter]);

  const handleSearch = useCallback(async () => {
    const query = searchQuery.trim();
    if (!query) {
      setSearchResults([]);
      return;
    }
    try {
      const hits = await graphRepo.searchGraphNodes(
        query,
        activeFilter === "all" ? undefined : [activeFilter],
        20,
      );
      setSearchResults(hits);
    } catch (err) {
      console.error("Failed to search knowledge graph:", err);
    }
  }, [searchQuery, graphRepo, activeFilter]);

  const typeCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const entry of stats?.nodesByType ?? []) {
      counts[entry.name] = entry.count;
    }
    return counts;
  }, [stats]);

  return (
    <div className="mx-auto flex h-[calc(100vh-64px)] flex-col overflow-hidden">
      <div className="flex shrink-0 items-center justify-between gap-4 border-b border-(--color-border-subtle) px-6 py-4">
        <div>
          <h1 className="font-(family-name:--font-display) text-xl font-bold">Knowledge Graph</h1>
          <p className="text-sm text-(--color-muted-foreground)">
            Typed graph across workspaces, files, planner reports, executions, memory, and sessions.
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
            {lastSync && (
              <>
                <span className="text-(--color-border-subtle)">|</span>
                <span className="text-(--color-success)">
                  +{lastSync.createdNodes} nodes · +{lastSync.createdEdges} edges
                </span>
              </>
            )}
          </div>
        )}
      </div>

      <div className="flex shrink-0 items-center gap-3 border-b border-(--color-border-subtle) px-6 py-2.5">
        <div className="flex items-center gap-1 overflow-x-auto">
          {NODE_TYPE_FILTERS.map((filter) => (
            <button
              key={filter.value}
              onClick={() => handleFilterChange(filter.value)}
              className={`shrink-0 rounded-[var(--radius-control)] px-2.5 py-1 text-xs font-medium transition-colors ${
                activeFilter === filter.value
                  ? "bg-(--color-accent)/10 text-(--color-accent)"
                  : "text-(--color-muted-foreground) hover:bg-(--color-surface-hover)"
              }`}
            >
              {filter.label}
              {filter.value !== "all" && (
                <span className="ml-1 text-[10px] text-(--color-faint-foreground)">
                  {typeCounts[filter.value] ?? 0}
                </span>
              )}
            </button>
          ))}
        </div>

        <div className="ml-auto flex items-center gap-2">
          {exploring && (
            <button
              onClick={() => fetchAllNodes(activeFilter)}
              className="flex shrink-0 items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1.5 text-xs font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover)"
            >
              <ChevronLeft className="h-3 w-3" strokeWidth={1.75} />
              <Map className="h-3 w-3" strokeWidth={1.75} />
              All nodes
            </button>
          )}
          <div className="flex items-center gap-1.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) px-2.5 py-1.5">
            <Search className="h-3.5 w-3.5 shrink-0 text-(--color-faint-foreground)" strokeWidth={1.75} />
            <input
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                if (!e.target.value) setSearchResults([]);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleSearch();
              }}
              placeholder="Search graph..."
              className="w-40 bg-transparent text-xs text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:outline-none"
            />
            {searchQuery && (
              <button
                onClick={() => {
                  setSearchQuery("");
                  setSearchResults([]);
                }}
                className="rounded p-0.5 text-(--color-faint-foreground) hover:text-(--color-foreground)"
              >
                <X className="h-3 w-3" strokeWidth={1.75} />
              </button>
            )}
          </div>
          <button
            onClick={handleSync}
            disabled={isSyncing}
            className="flex shrink-0 items-center gap-1.5 rounded-[var(--radius-control)] bg-(--color-accent) px-3 py-1.5 text-xs font-medium text-(--color-accent-foreground) transition-opacity disabled:opacity-50"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${isSyncing ? "animate-spin" : ""}`} strokeWidth={1.75} />
            Rebuild
          </button>
        </div>
      </div>

      {searchResults.length > 0 && (
        <div className="absolute left-1/2 top-24 z-30 w-80 -translate-x-1/2 overflow-hidden rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) py-1 shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
          {searchResults.map((node) => (
            <button
              key={`${node.nodeType}-${node.entityId}`}
              onClick={() => {
                handleNodeSelect(node);
                setSearchResults([]);
                setSearchQuery("");
              }}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm transition-colors hover:bg-(--color-surface-hover)"
            >
              <span className="h-2 w-2 shrink-0 rounded-full" style={{ backgroundColor: TYPE_COLORS[node.nodeType] }} />
              <span className="min-w-0 flex-1 truncate">{node.title}</span>
              <span className="ml-auto shrink-0 text-[10px] text-(--color-faint-foreground)">
                {node.nodeType.replace("_", " ")}
              </span>
            </button>
          ))}
        </div>
      )}

      <div className="flex flex-1 gap-0 overflow-hidden">
        <div className="flex flex-1 flex-col overflow-hidden">
          {isLoading ? (
            <div className="flex flex-1 items-center justify-center">
              <div className="flex flex-col items-center gap-3">
                <div className="h-8 w-8 animate-spin rounded-full border-2 border-(--color-border) border-t-(--color-accent)" />
                <p className="text-sm text-(--color-muted-foreground">Loading graph...</p>
              </div>
            </div>
          ) : error ? (
            <div className="flex flex-1 items-center justify-center">
              <div className="flex flex-col items-center gap-3">
                <p className="text-sm text-(--color-danger)">{error}</p>
                <button
                  onClick={() => fetchAllNodes(activeFilter)}
                  className="rounded-[var(--radius-control)] bg-(--color-accent) px-4 py-2 text-sm font-medium text-(--color-accent-foreground)"
                >
                  Retry
                </button>
              </div>
            </div>
          ) : (
            <KnowledgeGraphView
              nodes={nodes}
              edges={edges}
              onNodeSelect={handleNodeSelect}
              selectedNodeId={selectedNode?.entityId}
              emptyMessage={
                exploring
                  ? "This node has no connections yet."
                  : "Workspaces, files, planner reports, executions, memory records, and autonomous sessions become graph nodes automatically."
              }
            />
          )}
        </div>

        <div className="hidden w-80 shrink-0 overflow-y-auto border-l border-(--color-border-subtle) bg-(--color-surface) p-4 lg:block">
          {selectedNode && context ? (
            <div className="flex flex-col gap-4 animate-fade-in">
              <div className="flex items-start gap-2">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg" style={{ backgroundColor: `${TYPE_COLORS[selectedNode.nodeType]}20` }}>
                  <Network className="h-4 w-4" style={{ color: TYPE_COLORS[selectedNode.nodeType] }} strokeWidth={1.75} />
                </div>
                <div className="min-w-0">
                  <p className="truncate text-sm font-semibold text-(--color-foreground)">{selectedNode.title}</p>
                  <p className="text-[10px] font-medium uppercase tracking-wide" style={{ color: TYPE_COLORS[selectedNode.nodeType] }}>
                    {selectedNode.nodeType.replace("_", " ")}
                  </p>
                </div>
              </div>

              {selectedNode.summary && (
                <p className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2 text-xs text-(--color-muted-foreground)">
                  {selectedNode.summary}
                </p>
              )}

              <div className="space-y-2">
                <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
                  Related context ({context.related.length})
                </p>
                <div className="flex flex-col gap-1">
                  {context.related.slice(0, 12).map((hit) => (
                    <button
                      key={`${hit.node.nodeType}-${hit.node.entityId}`}
                      onClick={() => handleNodeSelect(hit.node)}
                      className="flex items-center gap-2 rounded-[var(--radius-control)] px-2 py-1.5 text-left transition-colors hover:bg-(--color-surface-hover)"
                    >
                      <span className="h-1.5 w-1.5 shrink-0 rounded-full" style={{ backgroundColor: TYPE_COLORS[hit.node.nodeType] }} />
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-xs text-(--color-foreground)">{hit.node.title}</span>
                        <span className="block truncate text-[10px] text-(--color-faint-foreground)">
                          {hit.reason}
                          {hit.relationshipType ? ` · ${hit.relationshipType.replace("_", " ")}` : ""}
                        </span>
                      </span>
                      <span className="shrink-0 font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
                        {(hit.weight * 100).toFixed(0)}%
                      </span>
                    </button>
                  ))}
                </div>
              </div>

              <div className="space-y-2">
                <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">Metadata</p>
                <div className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2 font-(family-name:--font-mono) text-[10px] text-(--color-muted-foreground)">
                  <pre className="max-h-40 overflow-y-auto whitespace-pre-wrap">{JSON.stringify(selectedNode.metadata, null, 2)}</pre>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex h-full flex-col items-center justify-center text-center opacity-40">
              <Network className="mb-3 h-8 w-8" strokeWidth={1.5} />
              <p className="text-xs">Select a node to explore its context.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
