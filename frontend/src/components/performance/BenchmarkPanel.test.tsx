// BenchmarkPanel tests — suite selection triggers the run callback and
// a completed suite renders per-benchmark latency, throughput, and state.

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { BenchmarkPanel } from "./BenchmarkPanel";
import type { BenchmarkSuiteResult } from "@/types/performance";

describe("BenchmarkPanel", () => {
  it("invokes the run callback with the chosen category", () => {
    const run = vi.fn();
    render(<BenchmarkPanel run={run} running={false} error={null} result={null} />);
    fireEvent.click(screen.getByRole("button", { name: "Planner" }));
    expect(run).toHaveBeenCalledWith("planner");
    fireEvent.click(screen.getByRole("button", { name: /All suites/ }));
    expect(run).toHaveBeenCalledWith(undefined);
  });

  it("renders empty state before a run", () => {
    render(<BenchmarkPanel run={vi.fn()} running={false} error={null} result={null} />);
    expect(screen.getByText(/Nothing measured yet/)).toBeInTheDocument();
  });

  it("renders measured benchmarks with state and throughput", () => {
    const result: BenchmarkSuiteResult = {
      suiteName: "memory",
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
        {
          id: 2,
          name: "semantic_search",
          operation: "search",
          category: "vector",
          iterations: 0,
          durationMs: 0,
          throughputPerSec: null,
          ok: false,
          payload: "semantic search not wired",
          createdAt: "2026-08-03T12:00:00Z",
        },
      ],
      totalDurationMs: 12,
      ranAt: "2026-08-03T12:00:00Z",
    };
    render(<BenchmarkPanel run={vi.fn()} running={false} error={null} result={result} />);
    expect(screen.getAllByText("memory_search").length).toBeGreaterThan(0);
    expect(screen.getAllByText("12 ms").length).toBeGreaterThan(0);
    expect(screen.getByText("83.3/s")).toBeInTheDocument();
    expect(screen.getByText("semantic search not wired")).toBeInTheDocument();
  });

  it("renders the running state", () => {
    render(<BenchmarkPanel run={vi.fn()} running error={null} result={null} />);
    expect(screen.getByText(/Running benchmark suite/)).toBeInTheDocument();
  });
});