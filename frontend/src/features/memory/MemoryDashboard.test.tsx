// MemoryDashboard tests - verify stats load, search calls the right IPC,
// recommendations/avoid render with confidence explanations, the vector
// index status card with the manual re-index action (RC-6 M2), and the
// adaptive learning cards: learning health, aging, failure patterns,
// workflow families, duplicate merge, and feedback (RC-6 M3).

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryDashboard } from "./MemoryDashboard";
import type {
  FailurePattern,
  LearningHealth,
  MemoryAgingSummary,
  MemoryStats,
  VectorIndexStatus,
  WorkflowFamily,
} from "@/types/memory";

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

const baseHealth = (overrides: Partial<LearningHealth> = {}): LearningHealth => ({
  confidence_average: 0.72,
  confidence_successful: 0.81,
  acceptance_rate: 0.83,
  score_average: 0.69,
  workflow_quality: {
    workflow_count: 2,
    avg_success_rate: 0.75,
    avg_plan_confidence: 0.7,
    avg_duration_seconds: 240,
    replay_adoption_rate: 0.5,
    replay_per_run: 1.2,
  },
  success_trends: [
    { date: "2026-08-01", successes: 3, failures: 1, success_rate: 0.75 },
    { date: "2026-08-02", successes: 4, failures: 1, success_rate: 0.8 },
  ],
  memory_utilization: {
    total_records: 3,
    active_records: 2,
    aging_records: 1,
    archived_records: 0,
    avg_freshness: 0.8,
    utilization_ratio: 0.67,
    workflows_per_record: 0.67,
  },
  ...overrides,
});

const baseAging = (overrides: Partial<MemoryAgingSummary> = {}): MemoryAgingSummary => ({
  total_records: 3,
  fresh_records: 2,
  aging_records: 1,
  archived_records: 0,
  avg_freshness: 0.8,
  oldest_days: 45,
  newest_days: 0,
  ...overrides,
});

const baseFailurePatterns = (overrides: Partial<FailurePattern> = {}): FailurePattern => ({
  pattern_type: "repeated_failure",
  goal: "deploy the app",
  goal_fingerprint: "deploy the app",
  description: "Goal failed 3 time(s) with only 1 success(es) — repeating it likely fails again",
  severity: 0.7,
  occurrences: 3,
  last_seen: "2026-08-02T09:58:00Z",
  avg_plan_confidence: null,
  ...overrides,
});

const baseFamilies = (overrides: Partial<WorkflowFamily> = {}): WorkflowFamily => ({
  family_id: 0,
  name: "Resume Focus Session",
  member_count: 2,
  goals: ["Resume My Focus Session", "Resume the most recent workspace"],
  shared_tools: ["resume_workspace"],
  total_successes: 3,
  total_failures: 1,
  avg_duration_seconds: 180,
  avg_confidence: 0.78,
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
  outcome: {
    steps: 2,
    completed: 2,
    replaced: 0,
    replan_count: 0,
    retries_used: 0,
    plans_attempted: 1,
    duration_seconds: 90,
  },
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
              confidence_score: 0.79,
              explanation: [
                { factor: "similarity", impact: 0.15, description: "Goal similarity is 95%" },
                { factor: "success_history", impact: 0.1, description: "100% of runs of this goal succeeded" },
                { factor: "replay_history", impact: 0.15, description: "Replayed 2 time(s); proven by reuse" },
                { factor: "freshness", impact: 0.1, description: "Memory is recent" },
                { factor: "usage_count", impact: 0.0, description: "Workflow has been used enough to be trusted (40%)" },
              ],
            },
          ]);
        }
        if (command === "memory_recommendation_feedback") {
          return Promise.resolve(null);
        }
        if (command === "memory_learning_health") {
          return Promise.resolve(baseHealth());
        }
        if (command === "memory_aging_summary") {
          return Promise.resolve(baseAging());
        }
        if (command === "memory_failure_patterns") {
          return Promise.resolve([baseFailurePatterns()]);
        }
        if (command === "memory_workflow_families") {
          return Promise.resolve([baseFamilies()]);
        }
        if (command === "memory_duplicate_groups") {
          return Promise.resolve([
            {
              goal_fingerprint: "resume my focus session",
              records: [record({ id: "mem-1" }), record({ id: "mem-3" })],
              keep_id: "mem-1",
              reason: "2 identical run(s) of 'resume my focus session' with the same outcome",
            },
          ]);
        }
        if (command === "memory_merge_duplicates") {
          return Promise.resolve({ groups_merged: 1, records_merged: 1 });
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
    expect(screen.getAllByText("resume my focus session").length).toBeGreaterThanOrEqual(1);
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

  it("shows learning health with confidence, quality, and trends", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Learning health")).toBeInTheDocument();
    });
    expect(screen.getByText("Avg confidence")).toBeInTheDocument();
    expect(screen.getByText("72%")).toBeInTheDocument();
    expect(screen.getByText("Acceptance rate")).toBeInTheDocument();
    expect(screen.getByText("83%")).toBeInTheDocument();
    expect(screen.getByText("Workflow quality")).toBeInTheDocument();
    expect(screen.getByText("Success trend (14 days)")).toBeInTheDocument();
  });

  it("shows memory aging buckets and average freshness", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Memory aging")).toBeInTheDocument();
    });
    expect(screen.getByText("Fresh")).toBeInTheDocument();
    expect(screen.getByText("Aging")).toBeInTheDocument();
    expect(screen.getByText("Archived")).toBeInTheDocument();
  });

  it("shows failure patterns with severity", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Failure patterns")).toBeInTheDocument();
    });
    expect(screen.getByText("deploy the app")).toBeInTheDocument();
    expect(screen.getByText("Repeated failure")).toBeInTheDocument();
    expect(screen.getByText(/severity 0.70/)).toBeInTheDocument();
  });

  it("shows workflow families with confidence bars", async () => {
    await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Workflow families")).toBeInTheDocument();
    });
    expect(screen.getByText("Resume Focus Session")).toBeInTheDocument();
    expect(screen.getByText(/2 workflow\(s\) · 3 ok · 1 failed/)).toBeInTheDocument();
    expect(screen.getByText(/confidence 78%/)).toBeInTheDocument();
  });

  it("explains recommendation confidence and sends feedback", async () => {
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
      expect(screen.getByText(/confidence 0.79/)).toBeInTheDocument();
    });
    expect(screen.getByText(/success_history/)).toBeInTheDocument();
    expect(screen.getByText("Goal similarity is 95%")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Accept"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith(
        "memory_recommendation_feedback",
        expect.objectContaining({ memoryId: "mem-1", accepted: true })
      );
    });
  });

  it("merges duplicate memories and refreshes the overview", async () => {
    const { invoke } = await setupTest();
    render(<MemoryDashboard />);

    await waitFor(() => {
      expect(screen.getByText("Duplicate memories")).toBeInTheDocument();
    });
    expect(screen.getByText("2 identical run(s)")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Merge duplicates"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("memory_merge_duplicates");
    });
    await waitFor(() => {
      const calls = vi
        .mocked(invoke)
        .mock.calls.filter(([cmd]) => String(cmd) === "memory_duplicate_groups");
      expect(calls.length).toBeGreaterThanOrEqual(2);
    });
  });
});
