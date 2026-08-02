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

const workspaceNode = makeNode();
const fileNode = makeNode({
  nodeType: "file",
  entityId: "file-1",
  title: "main.rs",
  workspaceId: "ws-1",
});

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
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
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
    expect(screen.getByText("Alpha WS")).toBeInTheDocument();
  });

  it("filters the node list by entity type", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await screen.findByText("Alpha WS");

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
    await screen.findByText("Alpha WS");

    fireEvent.click(screen.getByText("Alpha WS"));
    await waitFor(() => {
      expect(screen.getByText("Related context (1)")).toBeInTheDocument();
    });
    expect(screen.getByText(/File of this workspace/)).toBeInTheDocument();
    expect(screen.getByText(/contains/)).toBeInTheDocument();
    expect(screen.getAllByText("100%").length).toBeGreaterThan(0);
  });

  it("searches the graph and jumps to a hit", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPage />);
    await screen.findByText("Alpha WS");

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
    await screen.findByText("Alpha WS");

    fireEvent.click(screen.getByRole("button", { name: /rebuild/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_sync");
    });
    await screen.findByText("+0 nodes · +0 edges");
  });
});
