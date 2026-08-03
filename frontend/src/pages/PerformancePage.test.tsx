// PerformancePage tests — the page wires every `performance_*` IPC
// command to the dashboard, benchmark, and diagnostics surfaces.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PerformancePage } from "./PerformancePage";

const snapshot = {
  capturedAt: "2026-08-03T12:00:00Z",
  aggregates: [
    {
      category: "command",
      name: "performance_profile",
      count: 3,
      avgMs: 1.5,
      minMs: 1,
      maxMs: 2,
      p95Ms: 2,
    },
    {
      category: "worker",
      name: "learning_worker",
      count: 5,
      avgMs: 250,
      minMs: 10,
      maxMs: 900,
      p95Ms: 800,
    },
  ],
  recent: [
    {
      id: 1,
      category: "worker",
      name: "learning_worker",
      durationMs: 900,
      metadata: {},
      occurredAt: "2026-08-03T12:00:00Z",
    },
  ],
  slowest: [
    {
      id: 1,
      category: "worker",
      name: "learning_worker",
      durationMs: 900,
      metadata: {},
      occurredAt: "2026-08-03T12:00:00Z",
    },
  ],
};

const startupProfile = {
  runId: "run-1",
  totalMs: 1400,
  stages: [
    { name: "database", label: "Database initialization", durationMs: 200, startedAt: "2026-08-03T11:00:00Z" },
    { name: "graph_sync", label: "Initial knowledge graph sync", durationMs: 900, startedAt: "2026-08-03T11:00:00Z" },
    { name: "copilot", label: "Copilot engine", durationMs: 300, startedAt: "2026-08-03T11:00:01Z" },
  ],
  recordedAt: "2026-08-03T11:00:02Z",
};

const history = {
  profiles: [
    { id: 1, category: "command", name: "performance_profile", durationMs: 2, metadata: {}, occurredAt: "2026-08-03T12:00:00Z" },
  ],
  benchmarks: [
    {
      id: 1,
      name: "memory_search",
      operation: "search",
      category: "memory",
      iterations: 5,
      durationMs: 12,
      throughputPerSec: 83.3,
      ok: true,
      payload: {},
      createdAt: "2026-08-03T12:00:00Z",
    },
  ],
  startups: [startupProfile],
};

const diagnostics = {
  capturedAt: "2026-08-03T12:00:00Z",
  cpu: { usagePercent: 12.5, cores: 8, cpuParallelism: 8 },
  memory: { totalBytes: 16 * 1024 * 1024 * 1024, usedBytes: 8 * 1024 * 1024 * 1024, percent: 50 },
  db: { sizeBytes: 5_000_000, path: "chronodesk.db" },
  cache: { runtimeEntries: 4, runtimeHitRate: 0.9, graphCacheEntries: 0, graphCacheSizeBytes: 0 },
  workers: [
    { name: "learning_worker", status: "healthy", executionCount: 10, errorCount: 0, avgExecutionTimeMs: 12, lastExecution: null },
  ],
  threads: { totalThreads: 0, processCount: 64 },
};

const optimizeResult = {
  recommendations: [
    {
      id: "worker:slow:learning_worker",
      category: "worker",
      severity: "warning",
      title: 'Worker "learning_worker" runs slow passes',
      detail: "Average pass takes 250 ms.",
      action: null,
    },
  ],
  applied: [],
  analyzedAt: "2026-08-03T12:00:00Z",
};

describe("PerformancePage", () => {
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
        case "performance_profile":
          return Promise.resolve(snapshot);
        case "performance_startup":
          return Promise.resolve(startupProfile);
        case "performance_history":
          return Promise.resolve(history);
        case "performance_benchmark":
          return Promise.resolve({
            suiteName: (args?.category as string) ?? "all",
            benchmarks: history.benchmarks.map((b) => ({ ...b })),
            totalDurationMs: 12,
            ranAt: "2026-08-03T12:00:00Z",
          });
        case "performance_diagnostics":
          return Promise.resolve(diagnostics);
        case "performance_optimize":
          return Promise.resolve({
            ...optimizeResult,
            applied: (args?.apply as boolean) ? ["worker:slow:learning_worker"] : [],
          });
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
  };

  const renderDashboard = async () => {
    render(<PerformancePage />);
    await waitFor(() => expect(screen.getByText("Live Profiling")).toBeInTheDocument());
    await waitFor(() =>
      expect(screen.getAllByText("learning_worker").length).toBeGreaterThan(0),
    );
  };

  it("renders the dashboard with aggregates and benchmark history", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderDashboard();

    expect(screen.getByText("Live Profiling")).toBeInTheDocument();
    expect(screen.getAllByText("learning_worker").length).toBeGreaterThan(0);
    expect(screen.getByText("Benchmark runs (1)")).toBeInTheDocument();
    expect(screen.getByText("Startup Timeline")).toBeInTheDocument();
    expect(screen.getByText(/1,400 ms across 3 stages/)).toBeInTheDocument();
  });

  it("runs a benchmark suite from the benchmarks tab", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderDashboard();

    fireEvent.click(screen.getByRole("button", { name: "Benchmarks" }));
    fireEvent.click(screen.getByRole("button", { name: /All suites/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("performance_benchmark", { category: undefined });
    });
    expect((await screen.findAllByText("memory_search")).length).toBeGreaterThan(0);
  });

  it("shows diagnostics and runs the optimizer analysis", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderDashboard();

    fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));

    expect(await screen.findByText("System Diagnostics")).toBeInTheDocument();
    expect(await screen.findByText("13%")).toBeInTheDocument();
    expect(await screen.findByText("Optimization Recommendations")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Analyze" }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("performance_optimize", { apply: false });
    });
    expect(await screen.findByText(/runs slow passes/)).toBeInTheDocument();
  });

  it("applies a recommendation with the analyze-and-apply action", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderDashboard();

    fireEvent.click(screen.getByRole("button", { name: "Diagnostics" }));
    await screen.findByText("System Diagnostics");
    fireEvent.click(screen.getByRole("button", { name: /Analyze & apply/ }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("performance_optimize", { apply: true });
    });
  });
});