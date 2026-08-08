// GraphPage tests - the RC-8 knowledge graph page wires the stats header,
// entity-type filters, global search, subgraph exploration, and context
// discovery panel to the right IPC commands.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { GraphPage } from "./GraphPage";
import type { KgNode, KgStats } from "@/types/graph";

const makeNode = (overrides: Partial<KgNode> = {}): KgNode => ({
  nodeType: "workspace",
  entityId: "ws-1",
  title: "Alpha WS",
  workspaceId: "ws-1",
  summary: null,
  metadata: {},
  createdAt: "2026-08-02T09:58:00Z",
  updatedAt: "2026-08-02T09:58:00Z",
  ...overrides,
});

const stats: KgStats = {
  nodeCount: 3,
  edgeCount: 2,
  nodesByType: [
    { name: "workspace", count: 1 },
    { name: "file", count: 2 },
  ],
  edgesByType: [{ name: "contains", count: 2 }],
};

const analytics = {
  scope: "all",
  nodeCount: 3,
  edgeCount: 2,
  averageDegree: 1.33,
  density: 0.667,
  degreeDistribution: [
    { degree: 1, count: 2 },
    { degree: 2, count: 1 },
  ],
  topCentralNodes: [
    { nodeType: "workspace", entityId: "ws-1", title: "Alpha WS", inDegree: 2, outDegree: 0, degreeCentrality: 1.0, eigenvector: 0.707 },
  ],
  components: [{ index: 0, size: 3, nodeTypes: [{ name: "workspace", count: 1 }], memberTitles: ["Alpha WS"] }],
  workspaceImportance: [{ workspaceId: "ws-1", name: "Alpha WS", importance: 1.0, nodeCount: 3, edgeCount: 2, weightSum: 2.0 }],
  cached: true,
  computedAt: "2026-08-02T09:58:00Z",
};

const workspaceNode = makeNode();
const fileNode = makeNode({
  nodeType: "file",
  entityId: "file-1",
  title: "main.rs",
  workspaceId: "ws-1",
});
const memoryNode = makeNode({
  nodeType: "memory_record",
  entityId: "mem-1",
  title: "alpha crash fix",
  workspaceId: "ws-1",
});

const knowledgeSummary = (node: KgNode) => ({
  node,
  points: [
    { label: "Entity", value: node.title, detail: node.nodeType },
    { label: "Graph connections", value: "3", detail: "contains: 2, runs_in: 1" },
    { label: "Workspace", value: node.workspaceId ?? "global", detail: null },
    { label: "Last updated", value: "just now", detail: null },
  ],
  confidence: 0.74,
  generatedAt: "2026-08-02T09:58:00Z",
});

const contextInference = (node: KgNode) => ({
  source: node,
  related: [
    { node: fileNode, reason: "Direct file connection", score: 0.9, signal: "structural" },
    { node: memoryNode, reason: "Similar memory record", score: 0.55, signal: "memory" },
  ],
  confidence: { structural: 0.9, semantic: 0, temporal: 0, memory: 0.55, total: 0.5325 },
  inferredAt: "2026-08-02T09:58:00Z",
});

const workspaceSimilarity = {
  sourceWorkspaceId: "ws-1",
  sourceName: "Alpha WS",
  related: [
    {
      sourceWorkspaceId: "ws-1",
      targetWorkspaceId: "ws-2",
      targetName: "Beta WS",
      similarity: 0.72,
      confidence: 0.83,
      signals: [
        { signal: "goalOverlap", score: 0.8, detail: "goal overlap 0.80" },
        { signal: "structural", score: 0.3, detail: "cross-workspace graph bridges 0.30" },
      ],
      persisted: true,
    },
  ],
  cached: true,
  computedAt: "2026-08-02T09:58:00Z",
};

const goalClusters = [
  {
    id: 1,
    workspaceId: "ws-1",
    name: "fix login",
    memberCount: 2,
    members: [
      { nodeType: "memory_record", entityId: "mem-1", title: "fix login bug", workspaceId: "ws-1", score: 1.0 },
    ],
    centroidTerms: ["fix", "login"],
    confidence: 0.8,
  },
];

const snapshots = [
  {
    id: 1,
    workspaceId: "ws-1",
    snapshotType: "manual",
    nodeCount: 2,
    edgeCount: 1,
    confidence: 0.5,
    summary: [],
    createdAt: "2026-08-02T09:58:00Z",
  },
];

describe("GraphPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  const mockCommands = (invoke: ReturnType<typeof vi.fn>) => {
    invoke.mockImplementation((command: string, args?: Record<string, unknown>) => {
      switch (command) {
        case "graph_kg_stats":
          return Promise.resolve(stats);
        case "graph_nodes":
          if ((args?.nodeTypes as string[] | undefined)?.includes("file") && !(args?.nodeTypes as string[]).includes("workspace")) {
            return Promise.resolve([fileNode]);
          }
          return Promise.resolve([workspaceNode, fileNode]);
        case "graph_search":
          return Promise.resolve([fileNode]);
        case "graph_subgraph":
          return Promise.resolve({
            root: args?.entityId === "file-1" ? fileNode : workspaceNode,
            nodes: [workspaceNode, fileNode],
            edges: [],
          });
        case "graph_context":
          return Promise.resolve({
            source: args?.entityId === "file-1" ? fileNode : workspaceNode,
            related: [
              {
                node: fileNode,
                relationshipType: "contains",
                reason: "File of this workspace",
                weight: 1.0,
              },
            ],
          });
        case "graph_sync":
          return Promise.resolve({ createdNodes: 0, updatedNodes: 3, createdEdges: 0, updatedEdges: 2, totalNodes: 3, totalEdges: 2 });
        case "graph_analytics":
          return Promise.resolve(analytics);
        case "graph_incremental_sync":
          return Promise.resolve({ createdNodes: 1, updatedNodes: 0, createdEdges: 1, updatedEdges: 0, totalNodes: 3, totalEdges: 2 });
        case "graph_rebuild_semantic_edges":
          return Promise.resolve({ candidatePairs: 2, created: 1, updated: 0, pruned: 0, threshold: 0.45 });
        case "graph_apply_edge_decay":
          return Promise.resolve({ decayed: 2, pruned: 1, minConfidence: 0.1 });
        case "graph_relationship_details":
          return Promise.resolve({
            node: args?.entityId === "file-1" ? fileNode : workspaceNode,
            relationships: [
              {
                edge: {
                  id: "edge-1",
                  sourceNodeType: "workspace",
                  sourceEntityId: "ws-1",
                  targetNodeType: "file",
                  targetEntityId: "file-1",
                  relationshipType: "related_to",
                  weight: 0.8,
                  confidence: 0.62,
                  metadata: {},
                  createdAt: "2026-08-02T09:58:00Z",
                  updatedAt: "2026-08-02T09:58:00Z",
                },
                neighbor: fileNode,
              },
            ],
          });
        case "graph_knowledge_summary":
          return Promise.resolve(knowledgeSummary(args?.entityId === "file-1" ? fileNode : workspaceNode));
        case "graph_infer_context":
          return Promise.resolve(contextInference(args?.entityId === "file-1" ? fileNode : workspaceNode));
        case "graph_workspace_similarity":
          return Promise.resolve(workspaceSimilarity);
        case "graph_discover_cross_workspace_relationships":
          return Promise.resolve({ ...workspaceSimilarity, cached: false });
        case "graph_goal_clusters":
          return Promise.resolve(goalClusters);
        case "graph_snapshot_list":
          return Promise.resolve(snapshots);
        case "graph_snapshot_create":
          return Promise.resolve({ ...snapshots[0], id: 2 });
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
  };

  const findGraphNode = async (title: string) => {
    const nodes = await screen.findAllByText(title);
    return nodes[0];
  };

  it("loads stats and nodes on mount", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);

    await waitFor(() => {
      expect(screen.getByText("3 nodes")).toBeInTheDocument();
    });
    expect(screen.getByText("2 edges")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith(
      "graph_nodes",
      expect.objectContaining({ nodeTypes: expect.arrayContaining(["workspace", "file", "planner_report", "execution", "memory_record", "autonomous_session"]) }),
    );
    await findGraphNode("Alpha WS");
  });

  it("filters the node list by entity type", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /files/i }));
    await waitFor(() => {
      const fileCalls = invoke.mock.calls.filter(
        ([command, args]) => command === "graph_nodes" && JSON.stringify(args?.nodeTypes) === JSON.stringify(["file"]),
      );
      expect(fileCalls.length).toBeGreaterThan(0);
    });
    await screen.findByText("main.rs");
  });

  it("explores a node's subgraph and shows context discovery", async () => {
    mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /activity/i })).toHaveClass("bg-(--color-surface-hover)");
    });
    fireEvent.click(screen.getAllByText("Alpha WS")[0]);
    await waitFor(() => {
      expect(screen.getByText("Related context (1)")).toBeInTheDocument();
    });
    expect(screen.getByText(/File of this workspace/)).toBeInTheDocument();
    expect(screen.getAllByText(/contains/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("100%").length).toBeGreaterThan(0);
  });

  it("searches the graph and jumps to a hit", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    const input = screen.getByPlaceholderText("Search graph...");
    fireEvent.change(input, { target: { value: "main" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_search", expect.objectContaining({ query: "main" }));
    });
    await waitFor(() => {
      expect(screen.getAllByText("main.rs").length).toBeGreaterThan(0);
    });
  });

  it("rebuilds the graph via graph_sync and refreshes", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /rebuild/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_sync");
    });
    await screen.findByText("+0 nodes · +0 edges");
  });

  it("loads analytics on mount and shows graph density", async () => {
    mockCommands(await setupInvoke());
    render(<GraphPage />);

    await waitFor(() => {
      expect(screen.getByText(/density 66\.7%/)).toBeInTheDocument();
    });
    expect(screen.getByText("Graph analytics")).toBeInTheDocument();
    expect(screen.getByText("Global scope · cached")).toBeInTheDocument();
  });

  it("runs incremental sync, semantic rebuild, and edge decay", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /sync/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_incremental_sync");
    });
    await screen.findByText("+1 nodes · +1 edges");
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /rescore/i })).not.toBeDisabled();
    });

    fireEvent.click(screen.getByRole("button", { name: /rescore/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_rebuild_semantic_edges", expect.anything());
    });
    await screen.findByText(/semantic \+1/);
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /decay/i })).not.toBeDisabled();
    });

    fireEvent.click(screen.getByRole("button", { name: /decay/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_apply_edge_decay");
    });
    await screen.findByText(/decayed 2 · pruned 1/);
  });

  it("shows relationship details with confidence for a selected node", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /activity/i })).toHaveClass("bg-(--color-surface-hover)");
    });
    fireEvent.click(screen.getAllByText("Alpha WS")[0]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "graph_relationship_details",
        expect.objectContaining({ nodeType: "workspace", entityId: "ws-1" }),
      );
    });
    await waitFor(() => {
      expect(screen.getByText("Relationships (1)")).toBeInTheDocument();
    });
    expect(screen.getByText(/conf 62%/)).toBeInTheDocument();
    expect(screen.getAllByText("80%").length).toBeGreaterThan(0);
  });

  it("shows knowledge summary and confidence breakdown for a selected node", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /activity/i })).toHaveClass("bg-(--color-surface-hover)");
    });
    fireEvent.click(screen.getAllByText("Alpha WS")[0]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "graph_knowledge_summary",
        expect.objectContaining({ nodeType: "workspace", entityId: "ws-1" }),
      );
    });
    await waitFor(() => {
      expect(screen.getByText("Context intelligence")).toBeInTheDocument();
    });
    expect(screen.getByText("Graph connections")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("Confidence breakdown")).toBeInTheDocument();
    expect(screen.getAllByText("90%").length).toBeGreaterThan(0);
    expect(screen.getByText("Top inferred hits")).toBeInTheDocument();
    expect(screen.getAllByText("Structural").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Direct file connection/).length).toBeGreaterThan(0);
  });

  it("recomputes workspace relationships, shows clusters, and captures snapshots", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /activity/i })).toHaveClass("bg-(--color-surface-hover)");
    });
    fireEvent.click(screen.getAllByText("Alpha WS")[0]);
    await waitFor(() => {
      expect(screen.getByText("Beta WS")).toBeInTheDocument();
    });
    expect(screen.getByText(/Goal · Structural · persisted/)).toBeInTheDocument();
    expect(screen.getByText("fix login")).toBeInTheDocument();
    expect(screen.getByText("2 members")).toBeInTheDocument();
    expect(screen.getByText("Context snapshots")).toBeInTheDocument();
    expect(screen.getByText("2 nodes · 1 edges")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /recompute/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "graph_discover_cross_workspace_relationships",
        expect.objectContaining({ workspaceId: "ws-1" }),
      );
    });

    fireEvent.click(screen.getByRole("button", { name: /capture/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "graph_snapshot_create",
        expect.objectContaining({ workspaceId: "ws-1", snapshotType: "manual" }),
      );
    });
  });

  it("loads context intelligence for non-workspace nodes too", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await findGraphNode("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    fireEvent.click(screen.getAllByText("Alpha WS")[0]);
    await waitFor(() => {
      expect(screen.getByText("Context intelligence")).toBeInTheDocument();
    });

    fireEvent.click(screen.getAllByText("main.rs")[0]);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "graph_knowledge_summary",
        expect.objectContaining({ nodeType: "file", entityId: "file-1" }),
      );
    });
    await waitFor(() => {
      expect(screen.getAllByText("90%").length).toBeGreaterThan(0);
    });
    expect(screen.queryByText("Related workspaces")).not.toBeInTheDocument();
    expect(screen.queryByText("Context snapshots")).not.toBeInTheDocument();
  });
});
