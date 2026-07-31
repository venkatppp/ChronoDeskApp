/**
 * Context Memory Repository
 * 
 * Service layer for context memory and workspace intelligence.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  ContextSnapshot,
  CreateSnapshotRequest,
  KnowledgeQuery,
  KnowledgeSearchResult,
  RelatedWorkspace,
} from '../types/contextMemory';

export const contextMemoryRepository = {
  /**
   * Create a context snapshot
   */
  async createSnapshot(request: CreateSnapshotRequest): Promise<ContextSnapshot> {
    return await invoke<ContextSnapshot>('create_context_snapshot', { request });
  },

  /**
   * Get context snapshots for a workspace
   */
  async getWorkspaceSnapshots(workspaceId: string, limit?: number): Promise<ContextSnapshot[]> {
    return await invoke<ContextSnapshot[]>('get_workspace_snapshots', { workspaceId, limit });
  },

  /**
   * Get the latest snapshot for a workspace
   */
  async getLatestSnapshot(workspaceId: string): Promise<ContextSnapshot | null> {
    return await invoke<ContextSnapshot | null>('get_latest_snapshot', { workspaceId });
  },

  /**
   * Detect workspace relationships
   */
  async detectWorkspaceRelationships(workspaceId: string): Promise<void> {
    return await invoke<void>('detect_workspace_relationships', { workspaceId });
  },

  /**
   * Get related workspaces
   */
  async getRelatedWorkspaces(
    workspaceId: string,
    minStrength?: number,
    limit?: number
  ): Promise<RelatedWorkspace[]> {
    return await invoke<RelatedWorkspace[]>('get_related_workspaces', {
      workspaceId,
      minStrength,
      limit,
    });
  },

  /**
   * Search knowledge base
   */
  async searchKnowledge(query: KnowledgeQuery): Promise<KnowledgeSearchResult> {
    return await invoke<KnowledgeSearchResult>('search_knowledge', { query });
  },

  /**
   * Create a milestone snapshot
   */
  async snapshotMilestone(
    workspaceId: string,
    activeFiles: string[],
    metadata: unknown
  ): Promise<ContextSnapshot> {
    return await invoke<ContextSnapshot>('snapshot_milestone', {
      workspaceId,
      activeFiles,
      metadata,
    });
  },
};
