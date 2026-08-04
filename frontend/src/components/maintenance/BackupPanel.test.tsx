// BackupPanel tests — the ledger renders runs with status badges, sizes
// and checksums; successful backups expose a Restore action; staged
// restores surface with a cancel action.

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { BackupPanel } from "./BackupPanel";
import type { BackupRun, RestoreResult } from "@/types/backup";

const runs: BackupRun[] = [
  {
    id: 1,
    kind: "backup",
    status: "success",
    path: "chronodesk-20260804T120000000000Z.db",
    sizeBytes: 409_600,
    checksum: "ab12cd34ef56".padEnd(64, "0"),
    detail: "snapshot created",
    durationMs: 12,
    startedAt: "2026-08-04T12:00:00Z",
    completedAt: "2026-08-04T12:00:00Z",
  },
  {
    id: 2,
    kind: "integrity",
    status: "success",
    path: "",
    sizeBytes: 0,
    checksum: "",
    detail: "all checks passed",
    durationMs: 5,
    startedAt: "2026-08-04T12:01:00Z",
    completedAt: "2026-08-04T12:01:00Z",
  },
];

const pending: RestoreResult = {
  ok: true,
  message: "restore staged — restart to apply",
  backupPath: "/backups/chronodesk-20260804T120000000000Z.db",
  stagedPath: "/data/restore-pending.db",
  appliesOnNextLaunch: true,
  validated: {
    databaseSizeBytes: 409_600,
    pageCount: 100,
    pageSize: 4096,
    freelistCount: 0,
    journalMode: "",
    integrity: { ok: true, lines: [] },
    quickCheck: { ok: true, lines: ["ok"] },
    foreignKeyCheck: [],
  },
};

describe("BackupPanel", () => {
  it("renders runs with badges, sizes, checksums and durations", () => {
    render(
      <BackupPanel
        runs={runs}
        pending={null}
        loading={false}
        error={null}
        acting={false}
        onBackup={vi.fn()}
        onRestore={vi.fn()}
        onCancelRestore={vi.fn()}
      />,
    );
    expect(screen.getByText(/chronodesk-20260804T120000000000Z\.db/)).toBeInTheDocument();
    expect(screen.getByText(/400\.0 KB/)).toBeInTheDocument();
    expect(screen.getByText(/sha256 ab12cd34ef56/)).toBeInTheDocument();
    expect(screen.getByText("all checks passed")).toBeInTheDocument();
    expect(screen.getAllByText("success").length).toBeGreaterThanOrEqual(2);
  });

  it("shows a Restore action only for successful backups", () => {
    render(
      <BackupPanel
        runs={runs}
        pending={null}
        loading={false}
        error={null}
        acting={false}
        onBackup={vi.fn()}
        onRestore={vi.fn()}
        onCancelRestore={vi.fn()}
      />,
    );
    expect(screen.getAllByRole("button", { name: /Restore/ }).length).toBe(1);
  });

  it("fires onRestore with the backup id", () => {
    const onRestore = vi.fn();
    render(
      <BackupPanel
        runs={runs}
        pending={null}
        loading={false}
        error={null}
        acting={false}
        onBackup={vi.fn()}
        onRestore={onRestore}
        onCancelRestore={vi.fn()}
      />,
    );
    fireEvent.click(screen.getAllByRole("button", { name: /Restore/ })[0]);
    expect(onRestore).toHaveBeenCalledWith(1);
  });

  it("fires onBackup and onCancelRestore", () => {
    const onBackup = vi.fn();
    const onCancelRestore = vi.fn();
    render(
      <BackupPanel
        runs={runs}
        pending={pending}
        loading={false}
        error={null}
        acting={false}
        onBackup={onBackup}
        onRestore={vi.fn()}
        onCancelRestore={onCancelRestore}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Back up now/ }));
    expect(onBackup).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: /Cancel staged restore/ }));
    expect(onCancelRestore).toHaveBeenCalledOnce();
  });

  it("handles the empty, loading and error states", () => {
    render(
      <BackupPanel
        runs={[]}
        pending={null}
        loading={false}
        error={null}
        acting={false}
        onBackup={vi.fn()}
        onRestore={vi.fn()}
        onCancelRestore={vi.fn()}
      />,
    );
    expect(screen.getByText(/No runs recorded yet/)).toBeInTheDocument();
    render(
      <BackupPanel
        runs={[]}
        pending={null}
        loading
        error={null}
        acting={false}
        onBackup={vi.fn()}
        onRestore={vi.fn()}
        onCancelRestore={vi.fn()}
      />,
    );
    expect(screen.getByText(/Loading ledger/)).toBeInTheDocument();
    render(
      <BackupPanel
        runs={[]}
        pending={null}
        loading={false}
        error="boom"
        acting={false}
        onBackup={vi.fn()}
        onRestore={vi.fn()}
        onCancelRestore={vi.fn()}
      />,
    );
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});
