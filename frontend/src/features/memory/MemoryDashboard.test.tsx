// MemoryDashboard tests - verify stats load, search calls the right IPC,
// recommendations/avoid render, and the vector index status card with the
// manual re-index action (RC-6 M2).

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryDashboard } from "./MemoryDashboard";
import type { MemoryStats, VectorIndexStatus } from "@/types/memory";

const baseStats = (overrides: Partial<MemoryStats> = {}): MemoryStats => ({
  total_records: 3,
  successful: 2,
  failed: 1,
  cancelled: 0,
  executions: 1,
  planner_reports: 1,
  autonomous_sessions: 1,
  total_replays: 4,
  learned_workflows: 2,
  ...overrides,
});

const baseIndexStatus = (overrides: Partial<VectorIndexStatus> = {}): VectorIndexStatus => ({
  total_records: 3,
  indexed: 2,
  pending: 1,
  provider: "local-ngram",
  dimensions: 384,
  last_indexed_at: "2026-08-02T09:58:00Z",
  cache_size: 4,
  cache_capacity: 512,
  cache_hits: 8,
  cache_misses: 2,
  cache_hit_rate: 0.8,
  ...overrides,
});

const record = (overrides: Record<string, unknown> = {}) => ({
  id: "mem-1",
  kind: "execution",
  source_id: "exec-1",
  workspace_id: null,
  goal: "resume my focus session",
  status: "success",
  plan: null,
  steps: ["List workspaces", "Resume focused work"],
  reasoning: [],
  tools_used: ["list_workspaces", "resume_workspace"],
  failed_steps: [],
  error: null,
  outcome: { steps: 2, completed: 2, replaced: 0, replan_count: 0, retries_used: 0, plans_attempted: 1 },
  goal_embedding: null,
  replay_count: 2,
  created_at: "2026-08-02T09:58:00Z",
  updated_at: "2026-08-02T09:58:00Z",
  ...overrides,
});

const learnedWorkflow = {
  goal_fingerprint: "resume my focus session",
  goal: "Resume My Focus Session",
  success_count: 2,
  failure_count: 1,
  best_plan: null,
  last_success_at: "2026-08-02T09:58:00Z",
};

describe("MemoryDashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupTest = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementation(
      (_cmd: string, ..._args: unknown[]) => {
        const args = (_args[0] ?? {}) as Record<string, unknown>;
        const command = String(_cmd);
        if (command === "memory_stats") {
          return Promise.resolve(baseStats());
        }
        if (command === "memory_index_status") {
          return Promise.resolve(baseIndexStatus());
        }
        if (command === "memory_reindex") {
          return Promise.resolve({ requested: 3, indexed: 3, failed: 0, skipped: 0 });
        }
        if (command === "memory_learned_workflows") {
          return Promise.resolve([learnedWorkflow]);
        }
        if (command === "memory_search") {
          if (args?.query === "") {
            return Promise.resolve([{ record: record(), similarity: 0.4 }]);
          }
          return Promise.resolve([
            { record: record({ goal: "resume my focus session" }), similarity: 0.95 },
          ]);
        }
        if (command === "memory_recommend") {
          return Promise.resolve([
            {
              record: record(),
              score: 0.88,
              replay_count: 2,
            },
          ]);
        }
        if (command === "memory_avoid") {
          return Promise.resolve([
            {
              record: record({
                id: "mem-2",
                goal: "resume my focus session",
                status: "failed",
                error: "permission denied",
                failed_steps: ["get_recent_events"],
              }),
              similarity: 0.72,
              failure: "permission denied",
            },
          ]);
        }
        return Promise.resolve(null);
      }
    );
    return { invoke };
  };

  it("loads stats and recent memories on mount", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Total runs")).toBeInTheDocument();
    });
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getAllByText("2").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Recent memories")).toBeInTheDocument();
    expect(screen.getByText("resume my focus session")).toBeInTheDocument();
  });

  it("searches memory and renders ranked hits", async () => {
    const { invoke } = await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Recent memories")).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText(
      "Search remembered goals, e.g. resume my focus session"
    );
    fireEvent.change(input, { target: { value: "resume focus" } });
    fireEvent.click(screen.getByText("Search"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        expect.stringMatching("memory_search"),
        expect.objectContaining({ query: "resume focus" })
      );
    });
    expect(screen.getByText(/similarity 0.95/)).toBeInTheDocument();
  });

  it("recommends workflows and surfaces strategies to avoid", async () => {
    const { invoke } = await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Recent memories")).toBeInTheDocument();
    });

    const goalInput = screen.getByPlaceholderText(
      "Goal to plan for — e.g. resume my focus session"
    );
    fireEvent.change(goalInput, { target: { value: "resume focus" } });
    fireEvent.click(screen.getByText("Recommend"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("memory_recommend", expect.any(Object));
      expect(invoke).toHaveBeenCalledWith("memory_avoid", expect.any(Object));
    });
    expect(screen.getByText(/score 0.88/)).toBeInTheDocument();
    expect(screen.getByText("Strategies to avoid:")).toBeInTheDocument();
    expect(screen.getByText("permission denied")).toBeInTheDocument();
  });

  it("shows learned workflows with success/failure counts", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getAllByText("Learned workflows").length).toBeGreaterThanOrEqual(1);
    });
    expect(screen.getByText("Resume My Focus Session")).toBeInTheDocument();
    expect(screen.getByText(/2 ok · 1 failed/)).toBeInTheDocument();
  });

  it("shows vector index status with provider and coverage", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Vector index")).toBeInTheDocument();
    });
    const { within } = await import("@testing-library/react");
    const section = screen.getByText("Vector index").closest("section");
    expect(section).not.toBeNull();
    expect(within(section!).getByText("Indexed")).toBeInTheDocument();
    expect(within(section!).getByText("2/3")).toBeInTheDocument();
    expect(within(section!).getByText("Pending")).toBeInTheDocument();
    expect(within(section!).getByText("1")).toBeInTheDocument();
    expect(within(section!).getByText("local-ngram · 384d")).toBeInTheDocument();
    expect(within(section!).getByText("80%")).toBeInTheDocument();
  });

  it("re-indexes on demand and refreshes the overview", async () => {
    const { invoke } = await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Vector index")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Index now"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("memory_reindex");
    });
    await waitFor(() => {
      const statsCalls = vi
        .mocked(invoke)
        .mock.calls.filter(([cmd]) => String(cmd) === "memory_stats");
      expect(statsCalls.length).toBeGreaterThanOrEqual(2);
    });
  });
});
