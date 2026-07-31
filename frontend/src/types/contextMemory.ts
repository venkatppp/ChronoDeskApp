/**
 * Context Memory Types
 * 
 * TypeScript types for context memory and workspace intelligence.
 */

// ============================================================================
// Context Memory Types
// ============================================================================

export type SnapshotType = "manual" | "milestone" | "auto";

export interface ContextSnapshot {
  id: number;
  workspaceId: string;
  snapshotType: SnapshotType;
  capturedAt: string; // ISO-8601
  activeFiles: string[];
  sessionSummary?: unknown;
  timelineReferences?: number[];
  analyticsSummary?: unknown;
  healthScore?: number;
  recommendationsSummary?: string[];
  metadata: unknown;
}

export interface CreateSnapshotRequest {
  workspaceId: string;
  snapshotType: SnapshotType;
  activeFiles: string[];
  sessionSummary?: unknown;
  timelineReferences?: number[];
  analyticsSummary?: unknown;
  healthScore?: number;
  recommendationsSummary?: string[];
  metadata?: unknown;
}

export type WorkspaceRelationshipType =
  | "shared_files"
  | "shared_folders"
  | "shared_tech"
  | "similar_patterns";

export interface WorkspaceRelationship {
  id: number;
  sourceWorkspaceId: string;
  targetWorkspaceId: string;
  relationshipType: WorkspaceRelationshipType;
  strength: number;
  evidence: unknown;
  detectedAt: string; // ISO-8601
  lastUpdated: string; // ISO-8601
}

export interface RelatedWorkspace {
  workspaceId: string;
  workspaceName: string;
  relationshipType: WorkspaceRelationshipType;
  strength: number;
  evidence: unknown;
  lastActiveAt: string; // ISO-8601
}

export type KnowledgeQuery =
  | { type: "related_workspaces"; workspaceId: string }
  | { type: "related_files"; filePath: string }
  | { type: "recent_context"; workspaceId: string; limit: number }
  | { type: "previous_sessions"; workspaceId: string; limit: number }
  | { type: "similar_projects"; workspaceId: string };

export interface KnowledgeSearchResult {
  queryType: string;
  results: unknown;
  totalCount: number;
}
