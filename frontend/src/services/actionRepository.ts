/**
 * Action Repository
 * 
 * Service layer for action execution and history management.
 */

import { invoke } from '@tauri-apps/api/core';
import type { ActionHistory, ActionResult, ExecuteActionRequest } from '../types/actions';

export const actionRepository = {
  /**
   * Execute an action
   */
  async executeAction(request: ExecuteActionRequest): Promise<ActionResult> {
    return await invoke<ActionResult>('execute_action', { request });
  },

  /**
   * Undo an action
   */
  async undoAction(actionId: number): Promise<ActionResult> {
    return await invoke<ActionResult>('undo_action', { actionId });
  },

  /**
   * Get action history for a workspace
   */
  async getActionHistory(workspaceId: number, limit?: number): Promise<ActionHistory[]> {
    return await invoke<ActionHistory[]>('get_action_history', { workspaceId, limit });
  },

  /**
   * Get all action history
   */
  async getAllActionHistory(limit?: number): Promise<ActionHistory[]> {
    return await invoke<ActionHistory[]>('get_all_action_history', { limit });
  },

  /**
   * Clear all action history
   */
  async clearActionHistory(): Promise<void> {
    return await invoke<void>('clear_action_history');
  },

  /**
   * Clear action history for a workspace
   */
  async clearWorkspaceActionHistory(workspaceId: number): Promise<void> {
    return await invoke<void>('clear_workspace_action_history', { workspaceId });
  },
};
