// Copilot Repository - IPC bindings for AI assistant

import { invoke } from "@tauri-apps/api/core";
import type {
  Conversation,
  Message,
  CopilotResponse,
  SendMessageRequest,
  DailyBriefing,
  DailyBriefingRequest,
  WorkspaceAnswer,
  WorkspaceQuestionRequest,
  ToolDefinition,
} from "@/types/copilot";

export const copilotRepository = {
  /**
   * Send a message to the copilot assistant.
   */
  async sendMessage(request: SendMessageRequest): Promise<CopilotResponse> {
    return invoke<CopilotResponse>("copilot_send_message", { request });
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
};
