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

export interface CopilotStreamResponse {
  conversation_id: string;
  stream_id: string;
}

export interface StreamEventPayload {
  stream_id: string;
  conversation_id: string;
  content: string | null;
  message_id: string | null;
  status: "started" | "streaming" | "completed" | "cancelled" | "failed";
  error: string | null;
}

export interface StreamingDiagnostics {
  active_streams: number;
  started_streams: number;
  finished_streams: number;
  cancelled_streams: number;
  stream_errors: number;
  streamed_tokens: number;
  average_tokens_per_second: number;
  average_first_token_latency_ms: number;
  average_stream_duration_ms: number;
  provider_streaming_health: number;
}

export interface ToolDiagnostics {
  registered_tools: number;
  total_invocations: number;
  successful_invocations: number;
  failed_invocations: number;
  cancelled_invocations: number;
  retried_invocations: number;
  average_duration_ms: number;
  success_rate: number;
}

export interface ConversationSearchRequest {
  query: string | null;
  workspace_id: string | null;
  provider: string | null;
  start_date: string | null;
  end_date: string | null;
  limit: number | null;
}

export interface ConversationSearchResult {
  conversation: Conversation;
  matched_message_id: string | null;
  matched_at: string | null;
  snippet: string | null;
  provider: string | null;
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
  category?: string;
  permission?: ToolPermission;
  timeout_ms?: number;
  retry_policy?: ToolRetryPolicy;
  supports_parallel?: boolean;
}

export interface ToolParameter {
  name: string;
  param_type: string;
  parameter_type?: string;
  description: string;
  required: boolean;
}

export interface ToolPermission {
  required_level: string;
  requires_confirmation: boolean;
  risk_level: string;
}

export interface ToolRetryPolicy {
  max_attempts: number;
  backoff_ms: number;
  retryable: boolean;
}

export type ToolPermissionDecision = "allow_once" | "always_allow" | "deny";

export interface ToolPermissionPolicy {
  id: string;
  tool_name: string;
  workspace_id: string | null;
  decision: ToolPermissionDecision;
  created_at: string;
  updated_at: string;
}
