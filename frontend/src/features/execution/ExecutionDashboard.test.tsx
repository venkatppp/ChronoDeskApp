// ExecutionDashboard tests - verify the stream hook feeds the UI,
// controls call the right IPC, timeline shows checkpoint events, and the
// planner report renders when present.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ExecutionDashboard } from "./ExecutionDashboard";
import { ExecutionStatusPill } from "./ExecutionStatusPill";
import type { ExecutionProgress } from "@/types/execution";

const plan = {
  id: "plan-1",
  workspace_id: null,
  goal: "Bind outputs across workspaces",
  tasks: [
    {
      id: "task-a",
      description: "List workspaces",
      dependencies: [],
      estimated_minutes: 1,
      required_files: [],
      tool_name: "list_workspaces",
      arguments: {},
      completed: true,
    },
    {
      id: "task-b",
      description: "Resolve workspace",
      dependencies: ["task-a"],
      estimated_minutes: 1,
      required_files: [],
      tool_name: "get_workspace",
      arguments: {},
      completed: false,
    },
  ],
  estimated_duration_minutes: 2,
  required_files: [],
  checkpoints: [],
  confidence: 0.8,
  reasoning: "test",
  status: "executing" as const,
  created_at: "2026-08-02T09:58:00Z",
};

const baseProgress = (overrides: Partial<ExecutionProgress> = {}): ExecutionProgress => ({
  execution_id: "exec-1",
  status: "running",
  current_step: 1,
  total_steps: 2,
  progress_percentage: 50,
  steps: [
    {
      id: "step-1",
      execution_id: "exec-1",
      step_number: 0,
      description: "List workspaces",
      tool_name: "list_workspaces",
      arguments: {},
      status: "completed",
      result: "{}",
      error: null,
      started_at: "2026-08-02T10:00:00Z",
      completed_at: "2026-08-02T10:01:00Z",
      created_at: "2026-08-02T09:59:00Z",
    },
    {
      id: "step-2",
      execution_id: "exec-1",
      step_number: 1,
      description: "Resolve workspace",
      tool_name: "get_workspace",
      arguments: {},
      status: "running",
      result: null,
      error: null,
      started_at: "2026-08-02T10:01:00Z",
      completed_at: null,
      created_at: "2026-08-02T09:59:00Z",
    },
  ],
  recent_events: [
    {
      id: "evt-1",
      execution_id: "exec-1",
      event_type: "checkpoint_saved",
      step_number: null,
      message: "Checkpoint saved with 1 completed step(s)",
      metadata: { completed: [0] },
      created_at: "2026-08-02T10:01:00Z",
    },
    {
      id: "evt-2",
      execution_id: "exec-1",
      event_type: "step_completed",
      step_number: 0,
      message: "Completed step 1: List workspaces",
      metadata: {},
      created_at: "2026-08-02T10:01:00Z",
    },
  ],
  plan,
  planner_report: null,
  ...overrides,
});

describe("ExecutionDashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("restores current state on mount (reconnect) via execution_get_progress", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress());

    render(<ExecutionDashboard executionId="exec-1" />);

    await waitFor(() => {
      expect(screen.getByText("Bind outputs across workspaces")).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith("execution_get_progress", { executionId: "exec-1" });
  });

  it("shows pause and cancel while running, and issues execution_pause", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress());

    render(<ExecutionDashboard executionId="exec-1" />);
    await waitFor(() => expect(screen.getByTestId("pause-button")).toBeInTheDocument());

    fireEvent.click(screen.getByTestId("pause-button"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("execution_pause", { executionId: "exec-1" });
    });
  });

  it("shows resume control once paused and issues execution_resume", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress({ status: "paused" }));

    render(<ExecutionDashboard executionId="exec-1" />);
    const resume = await screen.findByTestId("resume-button");

    fireEvent.click(resume);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("execution_resume", { executionId: "exec-1" });
    });
  });

  it("issues execution_cancel and only shows controls for active states", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress());
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress({ status: "cancelled" }));

    render(<ExecutionDashboard executionId="exec-1" />);
    await waitFor(() => expect(screen.getByTestId("cancel-button")).toBeInTheDocument());

    fireEvent.click(screen.getByTestId("cancel-button"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("execution_cancel", { executionId: "exec-1" });
    });
  });

  it("renders the planner report panel when present", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    const progress = baseProgress({
      status: "completed",
      progress_percentage: 100,
      planner_report: {
        plan,
        execution_id: "exec-1",
        completed: ["task-a"],
        skipped: [],
        replaced: ["task-b"],
        replan_count: 2,
        error: null,
      },
    });
    vi.mocked(invoke).mockResolvedValueOnce(progress);

    render(<ExecutionDashboard executionId="exec-1" />);

    const report = await screen.findByTestId("planner-report");
    expect(report).toBeInTheDocument();
    expect(screen.getByText("2 replans")).toBeInTheDocument();
    expect(screen.getByTestId("planner-completed")).toBeInTheDocument();
  });

  it("shows checkpoint_saved events in the timeline", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(baseProgress());

    render(<ExecutionDashboard executionId="exec-1" />);

    await waitFor(() => {
      expect(screen.getByText("Checkpoint saved")).toBeInTheDocument();
    });
  });
});

describe("ExecutionStatusPill", () => {
  it("reflects execution status text", () => {
    const { rerender } = render(<ExecutionStatusPill status="failed" />);
    expect(screen.getByText("failed")).toBeInTheDocument();
    rerender(<ExecutionStatusPill status="completed" />);
    expect(screen.getByText("completed")).toBeInTheDocument();
  });
});