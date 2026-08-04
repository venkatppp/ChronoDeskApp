// MaintenancePanel tests — the report renders before/after measurements,
// freed-page recovery and the vacuum decision badge; the run button fires
// onRun.

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { MaintenancePanel } from "./MaintenancePanel";
import type { MaintenanceReport } from "@/types/backup";

const report: MaintenanceReport = {
  checkedAt: "2026-08-04T12:00:00Z",
  freelistBefore: 200,
  freelistAfter: 0,
  freedPages: 200,
  sizeBeforeBytes: 2_097_152,
  sizeAfterBytes: 1_638_400,
  recoveredBytes: 458_752,
  vacuumRan: true,
  checkpointedFrames: 120,
};

describe("MaintenancePanel", () => {
  it("renders before/after measurements and recovery", () => {
    render(<MaintenancePanel report={report} loading={false} error={null} acting={false} onRun={vi.fn()} />);
    expect(screen.getByText("vacuumed")).toBeInTheDocument();
    expect(screen.getByText("120 frames checkpointed")).toBeInTheDocument();
    expect(screen.getByText(/2\.0 MB → 1\.6 MB/)).toBeInTheDocument();
    expect(screen.getByText(/200 → 0/)).toBeInTheDocument();
    expect(screen.getByText("448.0 KB")).toBeInTheDocument();
    expect(screen.getByText(/200 pages freed/)).toBeInTheDocument();
  });

  it("renders the no-vacuum badge when a rewrite was skipped", () => {
    render(
      <MaintenancePanel
        report={{ ...report, vacuumRan: false, recoveredBytes: 0, freedPages: 0 }}
        loading={false}
        error={null}
        acting={false}
        onRun={vi.fn()}
      />,
    );
    expect(screen.getByText("no vacuum needed")).toBeInTheDocument();
    expect(screen.getByText("0 pages freed · ran at 2026-08-04T12:00:00Z")).toBeInTheDocument();
  });

  it("fires onRun and handles idle/loading/error states", () => {
    const onRun = vi.fn();
    render(<MaintenancePanel report={null} loading={false} error={null} acting={false} onRun={onRun} />);
    expect(screen.getByText(/No maintenance run yet/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Run maintenance/ }));
    expect(onRun).toHaveBeenCalledOnce();

    render(<MaintenancePanel report={null} loading error={null} acting={false} onRun={vi.fn()} />);
    expect(screen.getByText(/Running maintenance/)).toBeInTheDocument();

    render(<MaintenancePanel report={null} loading={false} error="boom" acting={false} onRun={vi.fn()} />);
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});