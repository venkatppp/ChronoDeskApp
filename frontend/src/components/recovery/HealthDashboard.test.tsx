// HealthDashboard tests — the score ring, status badge, worker rows and
// open issues render from a health snapshot.

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { HealthDashboard } from "./HealthDashboard";
import type { HealthSnapshot } from "@/types/recovery";

const snapshot: HealthSnapshot = {
  capturedAt: "2026-08-04T12:00:00Z",
  status: "degraded",
  overallScore: 50,
  workers: [
    { id: 1, worker: "runtime", status: "healthy", lastHeartbeat: "2026-08-04T11:59:00Z", consecutiveMisses: 0, executionCount: 0, errorCount: 0, lastError: "", details: null, updatedAt: "2026-08-04T11:59:00Z" },
    { id: 2, worker: "indexer", status: "stalled", lastHeartbeat: "2026-08-04T10:00:00Z", consecutiveMisses: 2, executionCount: 0, errorCount: 0, lastError: "", details: null, updatedAt: "2026-08-04T10:00:00Z" },
  ],
  issues: ["worker 'indexer' is stalled"],
  details: null,
};

describe("HealthDashboard", () => {
  it("renders the status badge and worker rows", () => {
    render(<HealthDashboard snapshot={snapshot} loading={false} error={null} />);
    expect(screen.getByText("Runtime Health")).toBeInTheDocument();
    expect(screen.getByText("Degraded")).toBeInTheDocument();
    expect(screen.getByText("50")).toBeInTheDocument();
    expect(screen.getByText("runtime")).toBeInTheDocument();
    expect(screen.getAllByText("stalled").length).toBeGreaterThan(0);
  });

  it("surfaces open issues", () => {
    render(<HealthDashboard snapshot={snapshot} loading={false} error={null} />);
    expect(screen.getByText("Open Issues")).toBeInTheDocument();
    expect(screen.getByText("worker 'indexer' is stalled")).toBeInTheDocument();
  });

  it("handles loading, error and empty snapshots", () => {
    render(<HealthDashboard snapshot={null} loading error={null} />);
    expect(screen.getByText(/Loading health snapshot/)).toBeInTheDocument();
    render(<HealthDashboard snapshot={null} loading={false} error="boom" />);
    expect(screen.getByText("boom")).toBeInTheDocument();
    render(<HealthDashboard snapshot={null} loading={false} error={null} />);
    expect(screen.getByText("No health snapshot yet.")).toBeInTheDocument();
  });
});