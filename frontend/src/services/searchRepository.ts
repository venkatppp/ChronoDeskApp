import { invoke } from "@tauri-apps/api/core";
import type { SearchFilters, SearchResult, SavedSearch, SearchStats } from "@/types/search";

export interface SearchRepository {
  search(filters: SearchFilters, limit?: number): Promise<SearchResult[]>;
  getSearchHistory(limit?: number): Promise<string[]>;
  saveSearchQuery(query: string): Promise<void>;
  clearSearchHistory(): Promise<void>;
  saveSearch(query: string): Promise<SavedSearch>;
  listSavedSearches(): Promise<SavedSearch[]>;
  deleteSavedSearch(id: string): Promise<void>;
  getRecentFiles(workspaceId: string, limit?: number): Promise<SearchResult[]>;
  getWorkspaceStats(workspaceId: string): Promise<SearchStats>;
}

export class TauriSearchRepository implements SearchRepository {
  async search(filters: SearchFilters, limit?: number): Promise<SearchResult[]> {
    return invoke<SearchResult[]>("search", {
      query: filters.query,
      entityTypes: filters.entityTypes,
      workspaceId: filters.workspaceId,
      limit,
    });
  }

  async getSearchHistory(limit?: number): Promise<string[]> {
    return invoke<string[]>("get_search_history", { limit });
  }

  async saveSearchQuery(query: string): Promise<void> {
    await invoke<void>("save_search_query", { query });
  }

  async clearSearchHistory(): Promise<void> {
    await invoke<void>("clear_search_history");
  }

  async saveSearch(query: string): Promise<SavedSearch> {
    return invoke<SavedSearch>("save_search", { query });
  }

  async listSavedSearches(): Promise<SavedSearch[]> {
    return invoke<SavedSearch[]>("list_saved_searches");
  }

  async deleteSavedSearch(id: string): Promise<void> {
    await invoke<void>("delete_saved_search", { id });
  }

  async getRecentFiles(workspaceId: string, limit?: number): Promise<SearchResult[]> {
    return invoke<SearchResult[]>("get_recent_files", { workspaceId, limit });
  }

  async getWorkspaceStats(workspaceId: string): Promise<SearchStats> {
    return invoke<SearchStats>("get_workspace_stats", { workspaceId });
  }
}

let repositoryInstance: SearchRepository | null = null;

export function getSearchRepository(): SearchRepository {
  if (!repositoryInstance) {
    repositoryInstance = new TauriSearchRepository();
  }
  return repositoryInstance;
}
