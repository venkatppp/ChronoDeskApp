// RecoveryRepository tests — every method forwards the right IPC
// command name and argument shape to `invoke`.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { TauriRecoveryRepository, getRecoveryRepository } from "./recoveryRepository";

describe("TauriRecoveryRepository", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  it("recoveryStatus invokes recovery_status", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryStatus();
    expect(invoke).toHaveBeenCalledWith("recovery_status");
  });

  it("recoveryHistory forwards the limit", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryHistory(40);
    expect(invoke).toHaveBeenCalledWith("recovery_history", { limit: 40 });
    await new TauriRecoveryRepository().recoveryHistory();
    expect(invoke).toHaveBeenCalledWith("recovery_history", { limit: undefined });
  });

  it("recoveryCrashReports forwards the limit", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryCrashReports(10);
    expect(invoke).toHaveBeenCalledWith("recovery_crash_reports", { limit: 10 });
  });

  it("recoveryLatestCheckpoint invokes recovery_latest_checkpoint", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryLatestCheckpoint();
    expect(invoke).toHaveBeenCalledWith("recovery_latest_checkpoint");
  });

  it("recoverySelfHeal invokes recovery_self_heal", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoverySelfHeal();
    expect(invoke).toHaveBeenCalledWith("recovery_self_heal");
  });

  it("recoveryRollback invokes recovery_rollback", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryRollback();
    expect(invoke).toHaveBeenCalledWith("recovery_rollback");
  });

  it("recoveryTick invokes recovery_tick", async () => {
    const invoke = await setupInvoke();
    await new TauriRecoveryRepository().recoveryTick();
    expect(invoke).toHaveBeenCalledWith("recovery_tick");
  });

  it("getRecoveryRepository returns a shared singleton", async () => {
    await import("@tauri-apps/api/core");
    expect(getRecoveryRepository()).toBe(getRecoveryRepository());
  });
});
