// RecoveryPage tests — the page wires every `recovery_*` IPC command to
// the health, history, and journal surfaces, and drives the manual
// self-healing and rollback actions.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { RecoveryPage } from "./RecoveryPage";

const snapshot = {
  capturedAt: "2026-08-04T12:00:00Z",
  status: "healthy",
  overallScore: 100,
  workers: [
    {
      id: 1,
      worker: "runtime",
      status: "healthy",
      lastHeartbeat: "2026-08-04T11:59:00Z",
      consecutiveMisses: 0,
      executionCount: 0,
      errorCount: 0,
      lastError: "",
      details: null,
      updatedAt: "2026-08-04T11:59:00Z",
    },
    {
      id: 2,
      worker: "indexer",
      status: "stalled",
      lastHeartbeat: "2026-08-04T10:00:00Z",
      consecutiveMisses: 2,
      executionCount: 0,
      errorCount: 0,
      lastError: "",
      details: null,
      updatedAt: "2026-08-04T10:00:00Z",
    },
  ],
  issues: ["worker 'indexer' is stalled"],
  details: null,
};

const checkpoint = {
  id: 7,
  entryType: "checkpoint",
  scope: "startup",
  entity: "app",
  state: "running",
  payload: { active_jobs: [] },
  checksum: "abc123",
  createdAt: "2026-08-04T11:58:00Z",
};

const history = {
  runs: [
    {
      id: 1,
      runId: "run-1",
      trigger: "startup",
      outcome: "no_action",
      status: "success",
      actions: ["checkpoint"],
      recoveredJobs: [],
      rolledBackTo: null,
      errors: [],
      durationMs: 4,
      startedAt: "2026-08-04T11:58:00Z",
      completedAt: "2026-08-04T11:58:00Z",
    },
  ],
  crashes: [
    {
      id: 1,
      component: "runtime",
      crashType: "timeout",
      severity: "error",
      message: "previous session ended without a clean shutdown",
      stackTrace: "",
      metadata: { checkpoint_id: 6 },
      wasRecovered: true,
      recoveredAt: "2026-08-04T11:58:00Z",
      reportedAt: "2026-08-04T11:58:00Z",
    },
  ],
  journal: [
    {
      id: 7,
      entryType: "checkpoint",
      scope: "startup",
      entity: "app",
      state: "running",
      payload: {},
      checksum: "abc123",
      createdAt: "2026-08-04T11:58:00Z",
    },
  ],
};

describe("RecoveryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  const mockCommands = (invoke: ReturnType<typeof vi.fn>) => {
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case "recovery_status":
          return Promise.resolve(snapshot);
        case "recovery_history":
          return Promise.resolve(history);
        case "recovery_latest_checkpoint":
          return Promise.resolve(checkpoint);
        case "recovery_tick":
          return Promise.resolve(42);
        case "recovery_self_heal":
          return Promise.resolve({ executed: ["restart_worker:indexer"], failed: [], healedWorkers: ["indexer"], ranAt: "2026-08-04T12:01:00Z" });
        case "recovery_rollback":
          return Promise.resolve({ rolledBackTo: 6, restored: [], ok: true, message: "rolled back" });
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
  };

  const renderPage = async () => {
    render(<RecoveryPage />);
    await waitFor(() => expect(screen.getByText("Runtime Health")).toBeInTheDocument());
  };

  it("renders the health dashboard with workers and issues", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    expect(screen.getByText("Runtime Health")).toBeInTheDocument();
    expect(screen.getByText("Session Checkpoint")).toBeInTheDocument();
    expect(screen.getByText(/#7/)).toBeInTheDocument();
    expect(screen.getAllByText("runtime").length).toBeGreaterThan(0);
    expect(screen.getByText("stalled")).toBeInTheDocument();
    expect(screen.getByText("Open Issues")).toBeInTheDocument();
  });

  it("shows recovery runs and crash reports on the history tab", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: "History" }));
    await waitFor(() => expect(screen.getByText("Recovery History")).toBeInTheDocument());
    expect(screen.getByText("Crash Reports")).toBeInTheDocument();
    expect(screen.getByText(/previous session ended without a clean shutdown/)).toBeInTheDocument();
  });

  it("renders the reliability journal tab", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: "Journal" }));
    await waitFor(() => expect(screen.getByText("Reliability Journal")).toBeInTheDocument());
    expect(screen.getByText("app")).toBeInTheDocument();
  });

  it("runs manual self-healing and refreshes the surfaces", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: /Run self-healing/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("recovery_self_heal");
    });
    expect(await screen.findByText(/Self-healing pass done/)).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("recovery_history", { limit: 100 });
  });

  it("rolls back to the newest valid checkpoint", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: /Roll back/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("recovery_rollback");
    });
    expect(await screen.findByText(/Rolled back to checkpoint #6/)).toBeInTheDocument();
  });
});
