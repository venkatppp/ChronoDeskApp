// HistoryPanel tests — recovery runs render with trigger, outcome,
// actions, recovered jobs and rollback targets.

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { HistoryPanel } from "./HistoryPanel";
import type { RecoveryRun } from "@/types/recovery";

const runs: RecoveryRun[] = [
  {
    id: 1,
    runId: "run-1",
    trigger: "startup",
    outcome: "recovered",
    status: "success",
    actions: ["revalidate", "resume"],
    recoveredJobs: ["job-1", "job-2"],
    rolledBackTo: null,
    errors: [],
    durationMs: 12,
    startedAt: "2026-08-04T11:58:00Z",
    completedAt: "2026-08-04T11:58:00Z",
  },
  {
    id: 2,
    runId: "run-2",
    trigger: "watchdog",
    outcome: "rolled_back",
    status: "success",
    actions: ["revalidate", "rollback"],
    recoveredJobs: ["job-3"],
    rolledBackTo: 4,
    errors: [],
    durationMs: 30,
    startedAt: "2026-08-04T12:00:00Z",
    completedAt: "2026-08-04T12:00:01Z",
  },
];

describe("HistoryPanel", () => {
  it("renders runs with outcome, actions and jobs", () => {
    render(<HistoryPanel runs={runs} loading={false} error={null} />);
    expect(screen.getByText("Recovery History")).toBeInTheDocument();
    expect(screen.getByText("recovered")).toBeInTheDocument();
    expect(screen.getByText(/revalidate → resume/)).toBeInTheDocument();
    expect(screen.getByText(/job-1/)).toBeInTheDocument();
    expect(screen.getByText(/rolled back to checkpoint #4/)).toBeInTheDocument();
  });

  it("handles the empty, loading and error states", () => {
    render(<HistoryPanel runs={[]} loading={false} error={null} />);
    expect(screen.getByText(/No recovery runs recorded/)).toBeInTheDocument();
    render(<HistoryPanel runs={[]} loading error={null} />);
    expect(screen.getByText(/Loading recovery history/)).toBeInTheDocument();
    render(<HistoryPanel runs={[]} loading={false} error="boom" />);
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});