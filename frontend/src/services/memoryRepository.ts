// Memory Repository - IPC bindings for the execution memory & learning
// system (RC-6 M1): search remembered runs, get workflow recommendations,
// and inspect what ChronoDesk has learned from previous executions.

import { invoke } from "@tauri-apps/api/core";
import type {
  AvoidedStrategy,
  LearnedWorkflow,
  MemoryHit,
  MemoryKind,
  MemoryRecommendation,
  MemoryStats,
  MemoryStatus,
} from "@/types/memory";

export const memoryRepository = {
  /**
   * Search remembered runs by goal similarity, with optional filters.
   */
  async search(
    query: string,
    options?: {
      kind?: MemoryKind;
      workspaceId?: string;
      status?: MemoryStatus;
      limit?: number;
    }
  ): Promise<MemoryHit[]> {
    return invoke<MemoryHit[]>("memory_search", {
      query,
      kind: options?.kind ?? null,
      workspaceId: options?.workspaceId ?? null,
      status: options?.status ?? null,
      limit: options?.limit ?? 10,
    });
  },

  /**
   * Recommend previously successful workflows for a goal, ranked by the
   * learning blend (similarity + success history + recency).
   */
  async recommend(
    goal: string,
    workspaceId?: string,
    limit?: number
  ): Promise<MemoryRecommendation[]> {
    return invoke<MemoryRecommendation[]>("memory_recommend", {
      goal,
      workspaceId: workspaceId ?? null,
      limit: limit ?? 5,
    });
  },

  /**
   * Failed/cancelled strategies relevant to a goal — what to avoid.
   */
  async avoid(
    goal: string,
    workspaceId?: string,
    limit?: number
  ): Promise<AvoidedStrategy[]> {
    return invoke<AvoidedStrategy[]>("memory_avoid", {
      goal,
      workspaceId: workspaceId ?? null,
      limit: limit ?? 5,
    });
  },

  /**
   * Aggregated workflows learned from repeated executions.
   */
  async learnedWorkflows(): Promise<LearnedWorkflow[]> {
    return invoke<LearnedWorkflow[]>("memory_learned_workflows");
  },

  /**
   * Dashboard statistics over the memory store.
   */
  async stats(): Promise<MemoryStats> {
    return invoke<MemoryStats>("memory_stats");
  },
};
