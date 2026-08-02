// useExecutionStream tests - verify mount restore (reconnect) via
// `execution_get_progress`, subscription to `execution:progress` with
// payload marshalling, and per-execution filtering.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useExecutionStream } from "@/hooks/useExecutionStream";
import type { ExecutionProgress } from "@/types/execution";

const progress = (overrides: Partial<ExecutionProgress> = {}): ExecutionProgress => ({
  execution_id: "exec-1",
  status: "running",
  current_step: 1,
  total_steps: 2,
  progress_percentage: 50,
  steps: [],
  recent_events: [],
  plan: null,
  planner_report: null,
  ...overrides,
});

describe("useExecutionStream", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("applies a streamed snapshot for the subscribed execution", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const { invoke } = await import("@tauri-apps/api/core");

    let handler: ((event: { payload: ExecutionProgress }) => void) | null = null;
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (_name: string, cb: (e: { payload: ExecutionProgress }) => void) => {
        handler = cb;
        return Promise.resolve(() => {});
      }
    );

    vi.mocked(invoke).mockResolvedValueOnce(progress());

    const { result } = renderHook(() => useExecutionStream("exec-1"));
    await waitFor(() => expect(result.current.progress).not.toBeNull());

    act(() => {
      handler?.({
        payload: progress({ status: "completed", progress_percentage: 100 }),
      });
    });

    expect(result.current.progress?.status).toBe("completed");
    expect(invoke).toHaveBeenCalledWith("execution_get_progress", { executionId: "exec-1" });
  });

  it("ignores events for other executions", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const { invoke } = await import("@tauri-apps/api/core");

    let handler: ((event: { payload: ExecutionProgress }) => void) | null = null;
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (_name: string, cb: (e: { payload: ExecutionProgress }) => void) => {
        handler = cb;
        return Promise.resolve(() => {});
      }
    );

    vi.mocked(invoke).mockResolvedValueOnce(progress());

    const { result } = renderHook(() => useExecutionStream("exec-1"));
    await waitFor(() => expect(result.current.progress).not.toBeNull());

    act(() =>
      handler?.({
        payload: progress({ execution_id: "other", status: "cancelled" }),
      })
    );

    expect(result.current.progress?.status).toBe("running");
  });
});