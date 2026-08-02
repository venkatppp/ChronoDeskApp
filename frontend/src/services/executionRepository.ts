// Execution Repository - IPC bindings for plan execution control &
// progress monitoring (RC-5 M5).

import { invoke } from "@tauri-apps/api/core";
import type { ExecutionPlan } from "@/types/proactive";
import type { ExecutionProgress } from "@/types/execution";

export const executionRepository = {
  /**
   * Start execution of an approved plan. Returns the new execution id.
   */
  async start(plan: ExecutionPlan, conversationId?: string): Promise<string> {
    return invoke<string>("execution_start", {
      plan,
      conversationId: conversationId ?? null,
    });
  },

  /**
   * Pause a running execution.
   */
  async pause(executionId: string): Promise<void> {
    return invoke<void>("execution_pause", { executionId });
  },

  /**
   * Resume a paused execution.
   */
  async resume(executionId: string): Promise<void> {
    return invoke<void>("execution_resume", { executionId });
  },

  /**
   * Cancel a running execution.
   */
  async cancel(executionId: string): Promise<void> {
    return invoke<void>("execution_cancel", { executionId });
  },

  /**
   * Fetch the current progress of an execution (used on reconnect/restore).
   */
  async getProgress(executionId: string): Promise<ExecutionProgress> {
    return invoke<ExecutionProgress>("execution_get_progress", { executionId });
  },

  /**
   * Fetch the most recent executions with full progress, so the dashboard
   * can re-attach to an in-flight or last-completed run after a reload.
   */
  async listRecent(limit?: number): Promise<ExecutionProgress[]> {
    return invoke<ExecutionProgress[]>("execution_list_recent", {
      limit: limit ?? 10,
    });
  },
};