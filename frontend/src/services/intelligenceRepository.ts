import { invoke } from "@tauri-apps/api/core";
import type { Recommendation, WorkspaceHealth } from "@/types/intelligence";

/**
 * Repository for intelligence layer operations.
 * 
 * Provides access to workspace health monitoring and recommendations
 * via the RecommendationEngine and WorkspaceHealthEngine.
 */
export interface IntelligenceRepository {
  getWorkspaceHealth(workspaceId: number): Promise<WorkspaceHealth>;
  getLatestWorkspaceHealth(workspaceId: number): Promise<WorkspaceHealth | null>;
  getWorkspaceHealthHistory(workspaceId: number, days: number): Promise<WorkspaceHealth[]>;
  getWorkspaceRecommendations(workspaceId: number): Promise<Recommendation[]>;
  getCategoryRecommendations(workspaceId: number, category: string): Promise<Recommendation[]>;
  getPriorityRecommendations(workspaceId: number, minPriority: string): Promise<Recommendation[]>;
}

export class TauriIntelligenceRepository implements IntelligenceRepository {
  async getWorkspaceHealth(workspaceId: number): Promise<WorkspaceHealth> {
    return invoke<WorkspaceHealth>("get_workspace_health", { workspaceId });
  }

  async getLatestWorkspaceHealth(workspaceId: number): Promise<WorkspaceHealth | null> {
    return invoke<WorkspaceHealth | null>("get_latest_workspace_health", { workspaceId });
  }

  async getWorkspaceHealthHistory(workspaceId: number, days: number): Promise<WorkspaceHealth[]> {
    return invoke<WorkspaceHealth[]>("get_workspace_health_history", { workspaceId, days });
  }

  async getWorkspaceRecommendations(workspaceId: number): Promise<Recommendation[]> {
    return invoke<Recommendation[]>("get_workspace_recommendations", { workspaceId });
  }

  async getCategoryRecommendations(workspaceId: number, category: string): Promise<Recommendation[]> {
    return invoke<Recommendation[]>("get_category_recommendations", { workspaceId, category });
  }

  async getPriorityRecommendations(workspaceId: number, minPriority: string): Promise<Recommendation[]> {
    return invoke<Recommendation[]>("get_priority_recommendations", { workspaceId, minPriority });
  }
}

let instance: IntelligenceRepository | null = null;

export function getIntelligenceRepository(): IntelligenceRepository {
  if (!instance) {
    instance = new TauriIntelligenceRepository();
  }
  return instance;
}
