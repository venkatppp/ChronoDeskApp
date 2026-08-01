// Copilot types matching backend models

export type MessageRole = "user" | "assistant" | "system";

export type ToolExecutionStatus = "pending" | "success" | "failed" | "cancelled";

export type PlanStatus = "planning" | "executing" | "completed" | "failed" | "cancelled";

export type SourceType = "timeline_event" | "context_memory" | "workspace_file" | "session_history" | "knowledge_graph";

export interface Conversation {
  id: string;
  workspace_id: string | null;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: MessageRole;
  content: string;
  tool_calls: ToolCall[] | null;
  reasoning: string | null;
  sources: Source[] | null;
  created_at: string;
}

export interface ToolCall {
  id: string;
  tool_name: string;
  arguments: Record<string, unknown>;
  result: Record<string, unknown> | null;
  status: ToolExecutionStatus;
}

export interface Source {
  source_type: SourceType;
  title: string;
  reference: string;
  relevance: number;
}

export interface SuggestedAction {
  title: string;
  description: string;
  tool_name: string;
  arguments: Record<string, unknown>;
  requires_confirmation: boolean;
}

export interface CopilotResponse {
  conversation_id: string;
  message: Message;
  suggested_actions: SuggestedAction[];
}

export interface SendMessageRequest {
  conversation_id: string | null;
  workspace_id: string | null;
  message: string;
  include_context: boolean;
}

export interface DailyBriefingRequest {
  workspace_id: string | null;
}

export interface WorkspaceStats {
  active_workspaces: number;
  files_modified: number;
  time_tracked: number;
  sessions_completed: number;
}

export interface DailyBriefing {
  date: string;
  summary: string;
  highlights: string[];
  pending_tasks: string[];
  recommendations: string[];
  workspace_stats: WorkspaceStats;
}

export interface WorkspaceQuestionRequest {
  workspace_id: string | null;
  question: string;
}

export interface WorkspaceAnswer {
  answer: string;
  reasoning: string;
  sources: Source[];
  confidence: number;
}

export interface ToolDefinition {
  name: string;
  description: string;
  parameters: ToolParameter[];
  requires_confirmation: boolean;
}

export interface ToolParameter {
  name: string;
  param_type: string;
  description: string;
  required: boolean;
}
