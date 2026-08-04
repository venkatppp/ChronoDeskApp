// CrashPanel tests — crash reports render with type, severity, and
// recovery state.

import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { CrashPanel } from "./CrashPanel";
import type { CrashReport } from "@/types/recovery";

const crashes: CrashReport[] = [
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
  {
    id: 2,
    component: "worker:indexer",
    crashType: "worker_failure",
    severity: "critical",
    message: "indexer crashed",
    stackTrace: "",
    metadata: null,
    wasRecovered: false,
    recoveredAt: null,
    reportedAt: "2026-08-04T11:00:00Z",
  },
];

describe("CrashPanel", () => {
  it("renders every crash with recovery and severity state", () => {
    render(<CrashPanel crashes={crashes} loading={false} error={null} />);
    expect(screen.getByText("Crash Reports")).toBeInTheDocument();
    expect(screen.getByText(/runtime/)).toBeInTheDocument();
    expect(screen.getByText("recovered")).toBeInTheDocument();
    expect(screen.getByText("critical")).toBeInTheDocument();
    expect(screen.getByText(/indexer crashed/)).toBeInTheDocument();
  });

  it("handles the empty, loading and error states", () => {
    render(<CrashPanel crashes={[]} loading={false} error={null} />);
    expect(screen.getByText(/No crashes recorded/)).toBeInTheDocument();
    render(<CrashPanel crashes={[]} loading error={null} />);
    expect(screen.getByText(/Loading crash reports/)).toBeInTheDocument();
    render(<CrashPanel crashes={[]} loading={false} error="boom" />);
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});