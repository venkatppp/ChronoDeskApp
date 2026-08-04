// IntegrityPanel tests — the battery results render a verdict, file
// statistics, and any violation lines; the run button fires onRun.

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { IntegrityPanel } from "./IntegrityPanel";
import type { IntegrityReport } from "@/types/backup";

const healthy: IntegrityReport = {
  checkedAt: "2026-08-04T12:00:00Z",
  dbPath: "/data/chronodesk.db",
  main: {
    databaseSizeBytes: 409_600,
    pageCount: 100,
    pageSize: 4096,
    freelistCount: 3,
    journalMode: "wal",
    integrity: { ok: true, lines: ["ok"] },
    quickCheck: { ok: true, lines: ["ok"] },
    foreignKeyCheck: [],
  },
  ok: true,
};

const damaged: IntegrityReport = {
  ...healthy,
  ok: false,
  main: {
    ...healthy.main,
    quickCheck: { ok: false, lines: ["page 4 is never used", "ok"] },
    foreignKeyCheck: ["files: row 9 references workspaces (fk 1)"],
  },
};

describe("IntegrityPanel", () => {
  it("renders a healthy verdict with file statistics", () => {
    render(<IntegrityPanel report={healthy} loading={false} error={null} acting={false} onRun={vi.fn()} />);
    expect(screen.getByText("healthy")).toBeInTheDocument();
    expect(screen.getByText(/400\.0 KB · 100 pages · 4096 B\/page · 3 free · journal wal/)).toBeInTheDocument();
    expect(screen.getByText(/checked 2026-08-04T12:00:00Z/)).toBeInTheDocument();
  });

  it("renders violation lines for a damaged database", () => {
    render(<IntegrityPanel report={damaged} loading={false} error={null} acting={false} onRun={vi.fn()} />);
    expect(screen.getByText("issues found")).toBeInTheDocument();
    expect(screen.getByText("page 4 is never used")).toBeInTheDocument();
    expect(screen.getByText(/files: row 9 references workspaces \(fk 1\)/)).toBeInTheDocument();
  });

  it("fires onRun and handles idle/loading/error states", () => {
    const onRun = vi.fn();
    render(<IntegrityPanel report={null} loading={false} error={null} acting={false} onRun={onRun} />);
    expect(screen.getByText(/No check run yet/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Run integrity check/ }));
    expect(onRun).toHaveBeenCalledOnce();

    render(<IntegrityPanel report={null} loading error={null} acting={false} onRun={vi.fn()} />);
    expect(screen.getByText(/Running checks/)).toBeInTheDocument();

    render(<IntegrityPanel report={null} loading={false} error="boom" acting={false} onRun={vi.fn()} />);
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});
