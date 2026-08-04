// MaintenanceRepository tests — every method forwards the right IPC
// command name and argument shape to `invoke`.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { TauriMaintenanceRepository, getMaintenanceRepository } from "./maintenanceRepository";

describe("TauriMaintenanceRepository", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  it("maintenanceIntegrity invokes maintenance_integrity", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceIntegrity();
    expect(invoke).toHaveBeenCalledWith("maintenance_integrity");
  });

  it("maintenanceBackup invokes maintenance_backup", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceBackup();
    expect(invoke).toHaveBeenCalledWith("maintenance_backup");
  });

  it("maintenanceBackups forwards the limit", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceBackups(25);
    expect(invoke).toHaveBeenCalledWith("maintenance_backups", { limit: 25 });
    await new TauriMaintenanceRepository().maintenanceBackups();
    expect(invoke).toHaveBeenCalledWith("maintenance_backups", { limit: undefined });
  });

  it("maintenanceRestore forwards the backup id", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceRestore(7);
    expect(invoke).toHaveBeenCalledWith("maintenance_restore", { backupId: 7 });
  });

  it("maintenancePendingRestore invokes maintenance_pending_restore", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenancePendingRestore();
    expect(invoke).toHaveBeenCalledWith("maintenance_pending_restore");
  });

  it("maintenanceCancelRestore invokes maintenance_cancel_restore", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceCancelRestore();
    expect(invoke).toHaveBeenCalledWith("maintenance_cancel_restore");
  });

  it("maintenanceOptimize invokes maintenance_optimize", async () => {
    const invoke = await setupInvoke();
    await new TauriMaintenanceRepository().maintenanceOptimize();
    expect(invoke).toHaveBeenCalledWith("maintenance_optimize");
  });

  it("getMaintenanceRepository returns a singleton", () => {
    expect(getMaintenanceRepository()).toBe(getMaintenanceRepository());
  });
});
