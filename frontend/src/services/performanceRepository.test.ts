// PerformanceRepository tests — every method forwards the right IPC
// command name and argument shape to `invoke`.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { TauriPerformanceRepository } from "./performanceRepository";

describe("TauriPerformanceRepository", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const setupInvoke = async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    return vi.mocked(invoke);
  };

  it("performanceProfile invokes performance_profile", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceProfile();
    expect(invoke).toHaveBeenCalledWith("performance_profile");
  });

  it("performanceStartup invokes performance_startup", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceStartup();
    expect(invoke).toHaveBeenCalledWith("performance_startup");
  });

  it("performanceBenchmark forwards the category", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceBenchmark("graph");
    expect(invoke).toHaveBeenCalledWith("performance_benchmark", { category: "graph" });
  });

  it("performanceBenchmark omits category for a full run", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceBenchmark();
    expect(invoke).toHaveBeenCalledWith("performance_benchmark", { category: undefined });
  });

  it("performanceDiagnostics invokes performance_diagnostics", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceDiagnostics();
    expect(invoke).toHaveBeenCalledWith("performance_diagnostics");
  });

  it("performanceOptimize forwards the apply flag", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceOptimize(true);
    expect(invoke).toHaveBeenCalledWith("performance_optimize", { apply: true });
    await repo.performanceOptimize();
    expect(invoke).toHaveBeenCalledWith("performance_optimize", { apply: undefined });
  });

  it("performanceHistory forwards the limit", async () => {
    const invoke = await setupInvoke();
    const repo = new TauriPerformanceRepository();
    await repo.performanceHistory(50);
    expect(invoke).toHaveBeenCalledWith("performance_history", { limit: 50 });
  });

  it("getPerformanceRepository returns a shared singleton", async () => {
    const { getPerformanceRepository } = await import("./performanceRepository");
    expect(getPerformanceRepository()).toBe(getPerformanceRepository());
  });
});