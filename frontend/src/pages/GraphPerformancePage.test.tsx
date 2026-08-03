// GraphPerformancePage tests - the RC-8 M4 page wires the performance
// dashboard (memory/cache statistics), integrity panel, orphan +
// consistency controls, benchmark viewer, query metrics, maintenance
// history, and the virtualized + progressively loaded node browser to
// the right IPC commands.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { GraphPerformancePage } from "./GraphPerformancePage";

const diagnostics = {
  integrity: {
    issues: [
      {
        id: 1,
        issueType: "orphan_edge",
        severity: "critical",
        nodeType: null,
        entityId: "edge-1",
        detail: "Edge references a node that no longer exists",
        status: "open",
        createdAt: "2026-08-03T10:00:00Z",
        resolvedAt: null,
      },
      {
        id: 2,
        issueType: "dangling_workspace",
        severity: "warning",
        nodeType: "file",
        entityId: "file-9",
        detail: "Workspace-linked node 'ghost.rs' has no workspace",
        status: "open",
        createdAt: "2026-08-03T10:00:00Z",
        resolvedAt: null,
      },
    ],
    issueTypeCounts: [
      { name: "orphan_edge", count: 1 },
      { name: "dangling_workspace", count: 1 },
    ],
    checkedAt: "2026-08-03T10:00:00Z",
  },
  consistency: {
    checks: [
      { name: "Node uniqueness", passed: true, detail: "0 duplicate (type, id) pairs" },
      { name: "Forward references", passed: false, detail: "1 edges reference a missing node" },
      { name: "Workspace references", passed: true, detail: "0 nodes reference a missing workspace" },
      { name: "Node well-formedness", passed: true, detail: "0 nodes with empty/unknown fields" },
      { name: "Confidence bounds", passed: true, detail: "0 edges with out-of-range confidence" },
    ],
    passed: false,
    checkedAt: "2026-08-03T10:00:00Z",
  },
  memory: {
    nodeCount: 5,
    edgeCount: 4,
    cacheEntries: 12,
    cacheSizeBytes: 4096,
    estimatedBytes: 4096 + 5 * 512 + 4 * 256,
  },
  recentMaintenance: [
    {
      id: 1,
      runType: "integrity_check",
      status: "completed",
      issuesFound: 2,
      issuesResolved: 0,
      durationMs: 12,
      summary: {},
      startedAt: "2026-08-03T10:00:00Z",
      finishedAt: "2026-08-03T10:00:01Z",
    },
  ],
  recentBenchmarks: [
    {
      name: "nodes_page_50",
      operation: "paginate_nodes",
      nodeCount: 5,
      edgeCount: 4,
      durationMs: 8,
      throughputPerSec: 6250,
      suiteName: "suite_1",
      createdAt: "2026-08-03T10:00:00Z",
    },
  ],
  recentMetrics: [
    {
      id: 1,
      operation: "ranked_search",
      scope: "all",
      query: "alpha",
      durationMs: 4,
      rowsReturned: 3,
      hitCache: false,
      occurredAt: "2026-08-03T10:00:00Z",
    },
    {
      id: 2,
      operation: "paginate_nodes",
      scope: null,
      query: null,
      durationMs: 2,
      rowsReturned: 50,
      hitCache: true,
      occurredAt: "2026-08-03T10:00:01Z",
    },
  ],
};

const node = (entityId: string, title: string, nodeType: "workspace" | "file" | "memory_record") => ({
  nodeType,
  entityId,
  title,
  workspaceId: entityId,
  summary: null,
  metadata: {},
  createdAt: "2026-08-03T09:00:00Z",
  updatedAt: "2026-08-03T09:00:00Z",
});

const nodePage = (offset: number, limit: number) => {
  const all = [node("ws-1", "Alpha WS", "workspace"), node("file-1", "main.rs", "file"), node("mem-1", "alpha crash fix", "memory_record")];
  // Two rows per page so the progressive-loading path is exercised.
  const page = all.slice(offset, offset + Math.min(limit, 2));
  return { nodes: page, total: all.length, offset, limit, hasMore: offset + page.length < all.length };
};

describe("GraphPerformancePage", () => {
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
        case "graph_diagnostics":
          return Promise.resolve(diagnostics);
        case "graph_nodes_page":
          return Promise.resolve(nodePage((args?.offset as number) ?? 0, (args?.limit as number) ?? 100));
        case "graph_integrity_check":
          return Promise.resolve(diagnostics.integrity);
        case "graph_repair":
          return Promise.resolve({ orphanEdgesRemoved: 1, danglingWorkspacesRemoved: 1, malformedNodesFixed: 0, invalidConfidenceFixed: 0, issuesResolved: 2 });
        case "graph_orphan_summary":
          return Promise.resolve({ orphanEdges: 1, danglingWorkspaces: 1 });
        case "graph_orphan_cleanup":
          return Promise.resolve({ orphanEdgesRemoved: 1, danglingWorkspacesRemoved: 1, issuesResolved: 2 });
        case "graph_consistency_report":
          return Promise.resolve(diagnostics.consistency);
        case "graph_benchmark_suite":
          return Promise.resolve({
            suiteName: "suite_live",
            benchmarks: [
              { name: "nodes_page_50", operation: "paginate_nodes", nodeCount: 5, edgeCount: 4, durationMs: 7, throughputPerSec: 7142, suiteName: "suite_live", createdAt: "2026-08-03T10:01:00Z" },
              { name: "memory_stats", operation: "memory_stats", nodeCount: 5, edgeCount: 4, durationMs: 1, throughputPerSec: null, suiteName: "suite_live", createdAt: "2026-08-03T10:01:00Z" },
            ],
            totalDurationMs: 8,
            ranAt: "2026-08-03T10:01:00Z",
          });
        case "graph_cache_trim":
          return Promise.resolve(50);
        case "graph_clear_expired_cache":
          return Promise.resolve(3);
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
  };

  it("renders memory statistics, consistency checks, and query metrics from diagnostics", async () => {
    mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);

    await waitFor(() => {
      expect(screen.getByText("Memory & cache statistics")).toBeInTheDocument();
    });
    expect(screen.getByText("5")).toBeInTheDocument();
    expect(screen.getByText("4.0 KB")).toBeInTheDocument();
    expect(screen.getByText("Forward references")).toBeInTheDocument();
    expect(screen.getByText(/1 edges reference a missing node/)).toBeInTheDocument();
    expect(screen.getByText(/ranked_search/)).toBeInTheDocument();
    expect(screen.getAllByText("cached").length).toBeGreaterThan(0);
        expect(screen.getByText("integrity check")).toBeInTheDocument();
    expect(screen.getByText(/2 found · 0 resolved · 12 ms/)).toBeInTheDocument();
  });

  it("runs the integrity check and repairs detected issues", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);
    await waitFor(() => {
      expect(screen.getByText("Integrity panel")).toBeInTheDocument();
    });

    expect(screen.getByText(/orphan edge · 1/)).toBeInTheDocument();
    expect(screen.getByText(/Edge references a node that no longer exists/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /run integrity check/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_integrity_check");
    });

    fireEvent.click(screen.getByRole("button", { name: /repair issues/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_repair");
    });
    await screen.findByText(/Repair removed 1 orphan edges, 1 dangling nodes/);
  });

  it("detects and cleans up orphans", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);
    await waitFor(() => {
      expect(screen.getByText("Orphans")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /detect/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_orphan_summary");
    });
    await screen.findAllByText("1");

    fireEvent.click(screen.getByRole("button", { name: /clean up/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_orphan_cleanup");
    });
    await screen.findByText(/Cleanup removed 1 edges and 1 nodes/);
  });

  it("runs the consistency verification", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);
    await waitFor(() => {
      expect(screen.getByText("Consistency")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /verify/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_consistency_report");
    });
  });

  it("runs the benchmark suite and lists results", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);
    await waitFor(() => {
      expect(screen.getByText("Benchmark viewer")).toBeInTheDocument();
    });
    expect(screen.getByText("nodes_page_50")).toBeInTheDocument();
    expect(screen.getByText("8 ms")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /run benchmark suite/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_benchmark_suite", expect.anything());
    });
    await screen.findByText(/Suite suite_live · 8 ms total/);
  });

  it("trims and sweeps the query cache", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);
    await waitFor(() => {
      expect(screen.getByText("Memory & cache statistics")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /trim 50 oldest/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_cache_trim", expect.objectContaining({ n: 50 }));
    });

    fireEvent.click(screen.getByRole("button", { name: /clear expired/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("graph_clear_expired_cache");
    });
  });

  it("loads nodes progressively through the virtualized browser", async () => {
    const invoke = mockCommands(await setupInvoke());
    render(<GraphPerformancePage />);

    await waitFor(() => {
      expect(screen.getByText("Virtualized node browser")).toBeInTheDocument();
    });
    await screen.findByText("Alpha WS");
    await screen.findByText(/Showing 2 of 3 nodes/);

    const list = screen.getByTestId("virtualized-node-list");
    Object.defineProperty(list, "clientHeight", { configurable: true, value: 480 });
    Object.defineProperty(list, "scrollHeight", { configurable: true, value: 5000 });
    list.scrollTop = 4900;
    fireEvent.scroll(list);

    await waitFor(() => {
      const pageCalls = invoke.mock.calls.filter(([command]) => command === "graph_nodes_page");
      expect(pageCalls.length).toBeGreaterThan(1);
    });
  });
});
