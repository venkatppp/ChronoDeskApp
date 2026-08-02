// Copilot Repository - IPC bindings for AI assistant

import { invoke } from "@tauri-apps/api/core";
import type {
  Conversation,
  Message,
  CopilotResponse,
  CopilotStreamResponse,
  SendMessageRequest,
  StreamingDiagnostics,
  ConversationSearchRequest,
  ConversationSearchResult,
  DailyBriefing,
  DailyBriefingRequest,
  WorkspaceAnswer,
  WorkspaceQuestionRequest,
  ToolDefinition,
  ToolDiagnostics,
} from "@/types/copilot";

export const copilotRepository = {
  /**
   * Send a message to the copilot assistant.
   */
  async sendMessage(request: SendMessageRequest): Promise<CopilotResponse> {
    return invoke<CopilotResponse>("copilot_send_message", { request });
  },

  /**
   * Start a streaming message response.
   */
  async sendMessageStream(request: SendMessageRequest): Promise<CopilotStreamResponse> {
    return invoke<CopilotStreamResponse>("copilot_send_message_stream", { request });
  },

  /**
   * Cancel an active streaming response.
   */
  async cancelStream(streamId: string): Promise<void> {
    return invoke<void>("copilot_cancel_stream", { streamId });
  },

  /**
   * Get current streaming diagnostics.
   */
  async getStreamingDiagnostics(): Promise<StreamingDiagnostics> {
    return invoke<StreamingDiagnostics>("copilot_get_streaming_diagnostics");
  },

  /**
   * Get conversation history by ID.
   */
  async getConversation(conversationId: string): Promise<Message[]> {
    return invoke<Message[]>("copilot_get_conversation", { conversationId });
  },

  /**
   * Get recent conversations.
   */
  async getRecentConversations(limit: number): Promise<Conversation[]> {
    return invoke<Conversation[]>("copilot_get_recent_conversations", { limit });
  },

  /**
   * Search conversations with backend filters.
   */
  async searchConversations(request: ConversationSearchRequest): Promise<ConversationSearchResult[]> {
    return invoke<ConversationSearchResult[]>("copilot_search_conversations", { request });
  },

  /**
   * Export a conversation as Markdown.
   */
  async exportConversationMarkdown(conversationId: string): Promise<string> {
    return invoke<string>("copilot_export_conversation_markdown", { conversationId });
  },

  /**
   * Export a conversation as JSON.
   */
  async exportConversationJson(conversationId: string): Promise<string> {
    return invoke<string>("copilot_export_conversation_json", { conversationId });
  },

  /**
   * Get daily briefing with workspace insights.
   */
  async getDailyBriefing(request: DailyBriefingRequest): Promise<DailyBriefing> {
    return invoke<DailyBriefing>("copilot_get_daily_briefing", { request });
  },

  /**
   * Ask a question about workspace history.
   */
  async askQuestion(request: WorkspaceQuestionRequest): Promise<WorkspaceAnswer> {
    return invoke<WorkspaceAnswer>("copilot_ask_question", { request });
  },

  /**
   * Get available tools the copilot can use.
   */
  async getTools(): Promise<ToolDefinition[]> {
    return invoke<ToolDefinition[]>("copilot_get_tools");
  },

  /**
   * Discover runtime tool metadata from the backend registry.
   */
  async discoverTools(): Promise<ToolDefinition[]> {
    return invoke<ToolDefinition[]>("copilot_discover_tools");
  },

  /**
   * Get current tool framework diagnostics.
   */
  async getToolDiagnostics(): Promise<ToolDiagnostics> {
    return invoke<ToolDiagnostics>("copilot_get_tool_diagnostics");
  },
};
