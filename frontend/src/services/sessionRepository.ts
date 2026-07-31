// Session Repository - IPC wrapper for session intelligence commands

import { invoke } from "@tauri-apps/api/core";
import type { Session, SessionSummary } from "@/types/session";

export class SessionRepository {
  /**
   * Gets the most recent session for Smart Resume.
   * Returns the latest active session across all workspaces.
   */
  async getSmartResumeSession(): Promise<SessionSummary | null> {
    return invoke<SessionSummary | null>("get_smart_resume_session");
  }

  /**
   * Gets recent sessions for a specific workspace.
   */
  async getWorkspaceSessions(
    workspaceId: string,
    limit?: number
  ): Promise<Session[]> {
    return invoke<Session[]>("get_workspace_sessions", {
      workspaceId,
      limit,
    });
  }

  /**
   * Gets the latest session for a specific workspace with full details.
   */
  async getLatestWorkspaceSession(
    workspaceId: string
  ): Promise<SessionSummary | null> {
    return invoke<SessionSummary | null>("get_latest_workspace_session", {
      workspaceId,
    });
  }

  /**
   * Updates the session inactivity threshold setting.
   * @param thresholdSeconds - Inactivity threshold in seconds (60-14400)
   */
  async setInactivityThreshold(thresholdSeconds: number): Promise<void> {
    return invoke<void>("set_session_inactivity_threshold", {
      thresholdSeconds,
    });
  }

  /**
   * Gets the current session inactivity threshold setting.
   * @returns Threshold in seconds
   */
  async getInactivityThreshold(): Promise<number> {
    return invoke<number>("get_session_inactivity_threshold");
  }
}

// Singleton instance
let sessionRepositoryInstance: SessionRepository | null = null;

export function getSessionRepository(): SessionRepository {
  if (!sessionRepositoryInstance) {
    sessionRepositoryInstance = new SessionRepository();
  }
  return sessionRepositoryInstance;
}
