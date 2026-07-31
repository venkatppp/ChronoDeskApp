/**
 * Action Types
 * 
 * TypeScript types for intelligent actions and automation.
 * Mirrors the Rust backend's action module types.
 */

// ============================================================================
// Action Types
// ============================================================================

export type ActionType =
  | "archive_workspace"
  | "restore_workspace"
  | "pin_workspace"
  | "unpin_workspace"
  | "clean_duplicate_files"
  | "open_suggested_workspace"
  | "resume_previous_session"
  | "open_most_relevant_files"
  | "mark_recommendation_complete";

export interface ExecuteActionRequest {
  actionType: ActionType;
  workspaceId?: number;
  recommendationId?: string;
  metadata?: unknown;
}

export interface ActionResult {
  success: boolean;
  message: string;
  actionId: number;
  data?: unknown;
}

export interface UndoState {
  wasArchived?: boolean;
  wasPinned?: boolean;
  deletedFileIds?: number[];
}

export interface ActionHistory {
  id: number;
  actionType: ActionType;
  workspaceId?: number;
  recommendationId?: string;
  executedAt: string; // ISO-8601
  success: boolean;
  metadata: unknown;
  undoState?: UndoState;
}
