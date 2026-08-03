import { invoke } from "@tauri-apps/api/core";
import type {
  BenchmarkCategory,
  BenchmarkSuiteResult,
  DiagnosticsSnapshot,
  OptimizeResult,
  PerformanceHistory,
  ProfileSnapshot,
  StartupProfile,
} from "@/types/performance";

export interface PerformanceRepository {
  /** Live profile snapshot (aggregates, recent, slowest). */
  performanceProfile(): Promise<ProfileSnapshot>;
  /** The most recent startup profile. */
  performanceStartup(): Promise<StartupProfile>;
  /** Runs one benchmark suite, or every suite when omitted. */
  performanceBenchmark(category?: BenchmarkCategory): Promise<BenchmarkSuiteResult>;
  /** System + application diagnostics snapshot. */
  performanceDiagnostics(): Promise<DiagnosticsSnapshot>;
  /** Runs the optimizer analysis; applies safe actions when `apply`. */
  performanceOptimize(apply?: boolean): Promise<OptimizeResult>;
  /** Combined recent history. */
  performanceHistory(limit?: number): Promise<PerformanceHistory>;
}

export class TauriPerformanceRepository implements PerformanceRepository {
  async performanceProfile(): Promise<ProfileSnapshot> {
    return invoke<ProfileSnapshot>("performance_profile");
  }

  async performanceStartup(): Promise<StartupProfile> {
    return invoke<StartupProfile>("performance_startup");
  }

  async performanceBenchmark(category?: BenchmarkCategory): Promise<BenchmarkSuiteResult> {
    return invoke<BenchmarkSuiteResult>("performance_benchmark", { category });
  }

  async performanceDiagnostics(): Promise<DiagnosticsSnapshot> {
    return invoke<DiagnosticsSnapshot>("performance_diagnostics");
  }

  async performanceOptimize(apply?: boolean): Promise<OptimizeResult> {
    return invoke<OptimizeResult>("performance_optimize", { apply });
  }

  async performanceHistory(limit?: number): Promise<PerformanceHistory> {
    return invoke<PerformanceHistory>("performance_history", { limit });
  }
}

let instance: PerformanceRepository | null = null;

/** Returns the shared performance repository (Tauri-backed). */
export function getPerformanceRepository(): PerformanceRepository {
  if (!instance) {
    instance = new TauriPerformanceRepository();
  }
  return instance;
}
