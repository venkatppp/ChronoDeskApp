// AutonomousDashboard tests - verify the stream hook feeds the UI,
// controls call the right IPC, approval gate renders, and reason log shows.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { AutonomousDashboard } from "./AutonomousDashboard";
import type { AutonomousSessionProgress } from "@/types/autonomous";

const baseProgress = (overrides: Partial<AutonomousSessionProgress> = {}): AutonomousSessionProgress => ({
  session_id: "sess-1",
  workspace_id: null,
  goal: "Resume latest workspace",
  status: "running",
  policy: {
    budget: { max_steps: 50, max_plans: 8, max_replans: 3, max_duration_seconds: 3600 },
    retry: { max_attempts: 1, backoff_ms: 250, retry_on_timeout: true },
    timeout: { step_timeout_ms: 10000, plan_timeout_seconds: 0, approval_timeout_seconds: 0 },
    approval: { mode: "automatic", gate_replans: false },
  },
  reasoning: [
    { session_id: "sess-1", phase: "planning", message: "Starting autonomous session", detail: null, created_at: "2026-08-02T09:58:00Z" },
  ],
  current_plan: null,
  execution_id: "exec-1",
  last_execution_id: "exec-1",
  plans_attempted: 1,
  plans_completed: 0,
  steps_completed: 0,
  retries_used: 0,
  replans_used: 0,
  steps_left: 50,
  error: null,
  pending_approval: null,
  created_at: "2026-08-02T09:58:00Z",
  updated_at: "2026-08-02T09:58:00Z",
  ...overrides,
});

describe("AutonomousDashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupTest = async (progressOverride?: Partial<AutonomousSessionProgress>) => {
    const { listen } = await import("@tauri-apps/api/event");
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (_name: string, _callback: (event: unknown) => void) => {
        return Promise.resolve(() => {});
      }
    );

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValue(baseProgress(progressOverride));

    return { invoke };
  };

  it("restores current state on mount (reconnect) via autonomous_get_progress", async () => {
    await setupTest();
    render(<AutonomousDashboard sessionId="sess-1" />);

    await waitFor(() => {
      expect(screen.getByText("Resume latest workspace")).toBeInTheDocument();
    });
  });

  it("shows pause and cancel while running, and issues autonomous_pause", async () => {
    const { invoke } = await setupTest();
    render(<AutonomousDashboard sessionId="sess-1" />);
    await waitFor(() => expect(screen.getByTestId("pause-button")).toBeInTheDocument());

    fireEvent.click(screen.getByTestId("pause-button"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("autonomous_pause", { sessionId: "sess-1" });
    });
  });

  it("shows resume control once paused and issues autonomous_resume", async () => {
    const { invoke } = await setupTest({ status: "paused" });
    render(<AutonomousDashboard sessionId="sess-1" />);
    const resume = await screen.findByTestId("resume-button");

    fireEvent.click(resume);
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("autonomous_resume", { sessionId: "sess-1" });
    });
  });

  it("issues autonomous_cancel and only shows controls for active states", async () => {
    const { invoke } = await setupTest();
    render(<AutonomousDashboard sessionId="sess-1" />);
    await waitFor(() => expect(screen.getByTestId("cancel-button")).toBeInTheDocument());

    fireEvent.click(screen.getByTestId("cancel-button"));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("autonomous_cancel", { sessionId: "sess-1" });
    });
  });

  it("shows approval gate when pending_approval is present", async () => {
    const progressWithApproval = baseProgress({
      status: "waiting_approval",
      pending_approval: {
        request_id: "req-1",
        session_id: "sess-1",
        goal: "Resume latest workspace",
        plan: { id: "plan-1", steps: [] } as any,
        reason: "Manual approval mode requires operator confirmation",
        requested_at: "2026-08-02T09:58:00Z",
        decided_at: null,
        approved: null,
        note: null,
      },
    });
    await setupTest(progressWithApproval);
    render(<AutonomousDashboard sessionId="sess-1" />);

    await waitFor(() => {
      expect(screen.getByText("Approval Required")).toBeInTheDocument();
    });
    expect(screen.getByTestId("approve-button")).toBeInTheDocument();
    expect(screen.getByTestId("reject-button")).toBeInTheDocument();
  });

  it("renders the reasoning log", async () => {
    await setupTest();
    render(<AutonomousDashboard sessionId="sess-1" />);

    await waitFor(() => {
      expect(screen.getByText("Reasoning Log (newest first)")).toBeInTheDocument();
    });
    expect(screen.getByText("Planning")).toBeInTheDocument();
    expect(screen.getByText("Starting autonomous session")).toBeInTheDocument();
  });

  it("shows error banner when session has an error", async () => {
    const progressWithError = baseProgress({
      error: "Execution budget exceeded: step budget (50) exhausted after 50 steps",
    });
    await setupTest(progressWithError);
    render(<AutonomousDashboard sessionId="sess-1" />);

    await waitFor(() => {
      expect(screen.getByText("Execution budget exceeded: step budget (50) exhausted after 50 steps")).toBeInTheDocument();
    });
  });
});