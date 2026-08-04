// MaintenancePage tests — the page wires every `maintenance_*` IPC
// command to the backups, integrity, and maintenance surfaces, and drives
// the backup/restore/cancel/integrity/maintenance actions.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MaintenancePage } from "./MaintenancePage";

const backupRun = {
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
};

const integrityReport = {
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

const maintenanceReport = {
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

const stagedRestore = {
  ok: true,
  message: "restore staged — restart to apply",
  backupPath: "/backups/chronodesk-20260804T120000000000Z.db",
  stagedPath: "/data/restore-pending.db",
  appliesOnNextLaunch: true,
  validated: integrityReport.main,
};

describe("MaintenancePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  const mockCommands = (invoke: ReturnType<typeof vi.fn>) => {
    let staged = false;
    invoke.mockImplementation((command: string) => {
      switch (command) {
        case "maintenance_backups":
          return Promise.resolve([backupRun]);
        case "maintenance_pending_restore":
          return Promise.resolve(staged ? stagedRestore : null);
        case "maintenance_backup":
          return Promise.resolve(backupRun);
        case "maintenance_restore":
          staged = true;
          return Promise.resolve(stagedRestore);
        case "maintenance_cancel_restore":
          staged = false;
          return Promise.resolve(null);
        case "maintenance_integrity":
          return Promise.resolve(integrityReport);
        case "maintenance_optimize":
          return Promise.resolve(maintenanceReport);
        default:
          return Promise.reject(new Error(`unexpected command ${command}`));
      }
    });
    return invoke;
  };

  const renderPage = async () => {
    render(<MaintenancePage />);
    await waitFor(() => expect(screen.getByText(/snapshot created/)).toBeInTheDocument());
  };

  it("renders the backup ledger on load", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    expect(screen.getByText(/chronodesk-20260804T120000000000Z\.db/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Back up now/ })).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("maintenance_backups", { limit: 100 });
    expect(invoke).toHaveBeenCalledWith("maintenance_pending_restore");
  });

  it("creates a backup and refreshes the ledger", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: /Back up now/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("maintenance_backup");
    });
    expect(await screen.findByText(/Backup created: chronodesk-20260804T120000000000Z\.db/)).toBeInTheDocument();
  });

  it("stages a restore and offers to cancel it", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: /Restore/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("maintenance_restore", { backupId: 1 });
    });
    fireEvent.click(screen.getByRole("button", { name: /Cancel staged restore/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("maintenance_cancel_restore");
    });
  });

  it("runs the integrity check on its tab", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: "Integrity" }));
    fireEvent.click(await screen.findByRole("button", { name: /Run integrity check/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("maintenance_integrity");
    });
    expect(await screen.findByText("healthy")).toBeInTheDocument();
  });

  it("runs the maintenance pass on its tab", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    await renderPage();

    fireEvent.click(screen.getByRole("button", { name: "Maintenance" }));
    fireEvent.click(await screen.findByRole("button", { name: /Run maintenance/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("maintenance_optimize");
    });
    expect(await screen.findByText("vacuumed")).toBeInTheDocument();
  });

  it("shows a staged restore banner with cancel", async () => {
    const invoke = await setupInvoke();
    mockCommands(invoke);
    invoke.mockClear();
    invoke.mockImplementation((command: string) => {
      if (command === "maintenance_pending_restore") return Promise.resolve(stagedRestore);
      return Promise.resolve([]);
    });
    render(<MaintenancePage />);
    expect(await screen.findByText(/Restore staged/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Cancel staged restore/ })).toBeInTheDocument();
  });
});