// Proactive AI Repository - IPC bindings

import { invoke } from "@tauri-apps/api/core";
import type {
  ProactiveNotification,
  ResumeContext,
  ExecutionPlan,
  PermissionLevel,
  EnhancedBriefing,
  TimelineIntelligence,
} from "@/types/proactive";

export const proactiveRepository = {
  /**
   * Get active proactive notifications.
   */
  async getNotifications(workspaceId?: string): Promise<ProactiveNotification[]> {
    return invoke<ProactiveNotification[]>("copilot_get_notifications", {
      workspaceId: workspaceId || null,
    });
  },

  /**
   * Dismiss a notification.
   */
  async dismissNotification(notificationId: string): Promise<void> {
    return invoke<void>("copilot_dismiss_notification", { notificationId });
  },

  /**
   * Get resume context for a workspace.
   */
  async getResumeContext(workspaceId: string): Promise<ResumeContext> {
    return invoke<ResumeContext>("copilot_get_resume_context", { workspaceId });
  },

  /**
   * Generate an execution plan for a goal.
   */
  async generatePlan(goal: string, workspaceId?: string): Promise<ExecutionPlan> {
    return invoke<ExecutionPlan>("copilot_generate_plan", {
      workspaceId: workspaceId || null,
      goal,
    });
  },

  /**
   * Set automation permission for an action.
   */
  async setPermission(
    actionType: string,
    permission: PermissionLevel,
    workspaceId?: string
  ): Promise<void> {
    return invoke<void>("copilot_set_permission", {
      workspaceId: workspaceId || null,
      actionType,
      permission,
    });
  },

  /**
   * Check automation permission for an action.
   */
  async checkPermission(actionType: string, workspaceId?: string): Promise<PermissionLevel> {
    return invoke<PermissionLevel>("copilot_check_permission", {
      workspaceId: workspaceId || null,
      actionType,
    });
  },

  /**
   * Get enhanced daily briefing with intelligence.
   */
  async getEnhancedBriefing(workspaceId?: string): Promise<EnhancedBriefing> {
    return invoke<EnhancedBriefing>("copilot_get_enhanced_briefing", {
      workspaceId: workspaceId || null,
    });
  },

  /**
   * Query timeline intelligence.
   */
  async queryTimeline(query: string, workspaceId?: string): Promise<TimelineIntelligence> {
    return invoke<TimelineIntelligence>("copilot_query_timeline", {
      workspaceId: workspaceId || null,
      query,
    });
  },

  /**
   * Trigger proactive opportunity check.
   */
  async checkOpportunities(workspaceId: string): Promise<void> {
    return invoke<void>("copilot_check_opportunities", { workspaceId });
  },
};
