// useAutonomousStream tests - verify mount restore (reconnect) via
// `autonomous_get_progress`, subscription to `autonomous:session` with
// payload marshalling, per-session filtering, and reasoning event log.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useAutonomousStream } from "@/hooks/useAutonomousStream";
import type { AutonomousSessionProgress, ReasoningEvent } from "@/types/autonomous";

const progress = (overrides: Partial<AutonomousSessionProgress> = {}): AutonomousSessionProgress => ({
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
  reasoning: [],
  current_plan: null,
  execution_id: null,
  last_execution_id: null,
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

const reasoningEvent = (overrides: Partial<ReasoningEvent> = {}): ReasoningEvent => ({
  session_id: "sess-1",
  phase: "planning",
  message: "Starting autonomous session",
  detail: null,
  created_at: "2026-08-02T09:58:00Z",
  ...overrides,
});

describe("useAutonomousStream", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const mockListen = async (handlers: {
    session?: (callback: (event: { payload: AutonomousSessionProgress }) => void) => void;
    reasoning?: (callback: (event: { payload: ReasoningEvent }) => void) => void;
  } = {}) => {
    const { listen } = await import("@tauri-apps/api/event");
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (name: string, cb: (event: { payload: unknown }) => void) => {
        if (name === "autonomous:session" && handlers.session) {
          handlers.session(cb as (event: { payload: AutonomousSessionProgress }) => void);
        } else if (name === "autonomous:reasoning" && handlers.reasoning) {
          handlers.reasoning(cb as (event: { payload: ReasoningEvent }) => void);
        }
        return Promise.resolve(() => {});
      }
    );
  };

  it("applies a streamed snapshot for the subscribed session", async () => {
    let sessionHandler: ((event: { payload: AutonomousSessionProgress }) => void) | null = null;
    await mockListen({
      session: (h) => { sessionHandler = h; },
    });

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(progress());

    const { result } = renderHook(() => useAutonomousStream("sess-1"));
    await waitFor(() => expect(result.current.progress).not.toBeNull());

    act(() => {
      sessionHandler?.({
        payload: progress({ session_id: "sess-1", status: "completed" }),
      });
    });

    await waitFor(() => expect(result.current.progress?.status).toBe("completed"));
    expect(invoke).toHaveBeenCalledWith("autonomous_get_progress", { sessionId: "sess-1" });
  });

  it("ignores events for other sessions", async () => {
    let sessionHandler: ((event: { payload: AutonomousSessionProgress }) => void) | null = null;
    await mockListen({
      session: (h) => { sessionHandler = h; },
    });

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(progress());

    const { result } = renderHook(() => useAutonomousStream("sess-1"));
    await waitFor(() => expect(result.current.progress).not.toBeNull());

    act(() =>
      sessionHandler?.({
        payload: progress({ session_id: "other", status: "cancelled" }),
      })
    );

    expect(result.current.progress?.status).toBe("running");
  });

  it("appends reasoning events for the subscribed session", async () => {
    let reasoningHandler: ((event: { payload: ReasoningEvent }) => void) | null = null;

    await mockListen({
      reasoning: (h) => { reasoningHandler = h; },
    });

    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockResolvedValueOnce(progress({ reasoning: [] }));

    const { result } = renderHook(() => useAutonomousStream("sess-1"));
    await waitFor(() => expect(result.current.progress).not.toBeNull());

    act(() => {
      reasoningHandler?.({
        payload: reasoningEvent({ phase: "executing", message: "Running plan step 1" }),
      });
    });

    expect(result.current.reasoning).toHaveLength(1);
    expect(result.current.reasoning[0].phase).toBe("executing");
  });
});