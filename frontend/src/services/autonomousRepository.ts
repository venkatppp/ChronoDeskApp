// Autonomous Repository - IPC bindings for the autonomous agent runtime
// (RC-5 M6): start / observe / control autonomous sessions and resolve
// approval checkpoints.

import { invoke } from "@tauri-apps/api/core";
import type {
  AutonomousSessionProgress,
  ExecutionPolicy,
} from "@/types/autonomous";

export const autonomousRepository = {
  /**
   * Start an autonomous session for a goal. Returns the initial progress
   * snapshot; the loop runs detached and streams `autonomous:session` /
   * `autonomous:reasoning` events.
   */
  async start(
    goal: string,
    workspaceId?: string,
    policy?: ExecutionPolicy
  ): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_start", {
      goal,
      workspaceId: workspaceId ?? null,
      policy: policy ?? null,
    });
  },

  /**
   * Current progress snapshot for one session (used on reconnect/restore).
   */
  async getProgress(sessionId: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_get_progress", {
      sessionId,
    });
  },

  /**
   * Recent sessions (newest first) so the page can list + re-attach.
   */
  async listRecent(limit?: number): Promise<AutonomousSessionProgress[]> {
    return invoke<AutonomousSessionProgress[]>("autonomous_list_recent", {
      limit: limit ?? 10,
    });
  },

  /**
   * Pause a running session.
   */
  async pause(sessionId: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_pause", { sessionId });
  },

  /**
   * Resume a paused session.
   */
  async resume(sessionId: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_resume", { sessionId });
  },

  /**
   * Cancel a session, propagating to the active engine run.
   */
  async cancel(sessionId: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_cancel", { sessionId });
  },

  /**
   * Approve a pending approval checkpoint.
   */
  async approve(sessionId: string, note?: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_approve", {
      sessionId,
      note: note ?? null,
    });
  },

  /**
   * Reject a pending approval checkpoint (terminates the session).
   */
  async reject(sessionId: string, note?: string): Promise<AutonomousSessionProgress> {
    return invoke<AutonomousSessionProgress>("autonomous_reject", {
      sessionId,
      note: note ?? null,
    });
  },
};