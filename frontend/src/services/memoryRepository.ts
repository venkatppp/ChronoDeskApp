// Memory Repository - IPC bindings for the execution memory & learning
// system (RC-6 M1): search remembered runs, get workflow recommendations,
// and inspect what ChronoDesk has learned from previous executions.

import { invoke } from "@tauri-apps/api/core";
import type {
  AvoidedStrategy,
  DuplicateGroup,
  FailurePattern,
  IndexResult,
  LearnedWorkflow,
  LearningHealth,
  MemoryAgingSummary,
  MemoryHit,
  MemoryKind,
  MemoryRecommendation,
  MemoryStats,
  MemoryStatus,
  MergeResult,
  VectorIndexStatus,
  WorkflowFamily,
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

  /**
   * Status of the vector index and embedding cache (RC-6 M2).
   */
  async indexStatus(): Promise<VectorIndexStatus> {
    return invoke<VectorIndexStatus>("memory_index_status");
  },

  /**
   * Runs an index pass now, re-indexing everything if needed (RC-6 M2).
   */
  async reindex(): Promise<IndexResult> {
    return invoke<IndexResult>("memory_reindex");
  },

  /**
   * Records user acceptance/rejection of a recommendation, feeding the
   * acceptance ledger the adaptive weights learn from (RC-6 M3).
   */
  async recommendationFeedback(memoryId: string, accepted: boolean): Promise<void> {
    return invoke<void>("memory_recommendation_feedback", {
      memoryId,
      accepted,
    });
  },

  /**
   * Learning health: confidence averages, workflow quality, success
   * trends, memory utilization (RC-6 M3).
   */
  async learningHealth(): Promise<LearningHealth> {
    return invoke<LearningHealth>("memory_learning_health");
  },

  /**
   * Detected failure patterns (RC-6 M3).
   */
  async failurePatterns(): Promise<FailurePattern[]> {
    return invoke<FailurePattern[]>("memory_failure_patterns");
  },

  /**
   * Workflow families learned by clustering remembered goals (RC-6 M3).
   */
  async workflowFamilies(): Promise<WorkflowFamily[]> {
    return invoke<WorkflowFamily[]>("memory_workflow_families");
  },

  /**
   * Memory aging summary (fresh / aging / archived, RC-6 M3).
   */
  async agingSummary(): Promise<MemoryAgingSummary> {
    return invoke<MemoryAgingSummary>("memory_aging_summary");
  },

  /**
   * Identical memories detected in the store (RC-6 M3).
   */
  async duplicateGroups(): Promise<DuplicateGroup[]> {
    return invoke<DuplicateGroup[]>("memory_duplicate_groups");
  },

  /**
   * Merges identical memories, keeping the best record of each group
   * (RC-6 M3).
   */
  async mergeDuplicates(): Promise<MergeResult> {
    return invoke<MergeResult>("memory_merge_duplicates");
  },
};
