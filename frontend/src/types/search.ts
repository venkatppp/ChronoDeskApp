export type SearchEntityType = "workspace" | "file";

export interface SearchResult {
  entityType: SearchEntityType;
  entityId: string;
  workspaceId: string;
  title: string;
  snippet: string;
  rank: number;
}

export interface SavedSearch {
  id: string;
  query: string;
  createdAt: string;
}

export interface SearchStats {
  totalFiles: number;
  totalWorkspaces: number;
  lastIndexed: string | null;
}

export interface SearchFilters {
  query: string;
  entityTypes: SearchEntityType[];
  workspaceId: string | null;
}
