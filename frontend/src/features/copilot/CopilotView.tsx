// CopilotView - Main AI assistant interface with chat, sidebar, and briefing

import { useState, useEffect, useRef } from "react";
import { Sparkles } from "lucide-react";
import { copilotRepository } from "@/services/copilotRepository";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { Message, Conversation, SuggestedAction, DailyBriefing } from "@/types/copilot";
import { ChatMessage } from "./components/ChatMessage";
import { ChatInput } from "./components/ChatInput";
import { ConversationSidebar } from "./components/ConversationSidebar";
import { WorkspaceContext } from "./components/WorkspaceContext";
import { SuggestedActions } from "./components/SuggestedActions";
import { DailyBriefingWidget } from "./components/DailyBriefingWidget";

export function CopilotView() {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [suggestedActions, setSuggestedActions] = useState<SuggestedAction[]>([]);
  const [isGenerating, setIsGenerating] = useState(false);
  const [currentWorkspace, setCurrentWorkspace] = useState<string | null>(null);
  const [dailyBriefing, setDailyBriefing] = useState<DailyBriefing | null>(null);
  const [showBriefing, setShowBriefing] = useState(false);
  const [loadingBriefing, setLoadingBriefing] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const abortControllerRef = useRef<AbortController | null>(null);

  // Auto-scroll to bottom
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // Load recent conversations on mount
  useEffect(() => {
    loadConversations();
    loadCurrentWorkspace();
  }, []);

  // Load conversation messages when selected
  useEffect(() => {
    if (currentConversationId) {
      loadConversation(currentConversationId);
    }
  }, [currentConversationId]);

  const loadConversations = async () => {
    try {
      const convs = await copilotRepository.getRecentConversations(50);
      setConversations(convs);
    } catch (error) {
      console.error("Failed to load conversations:", error);
    }
  };

  const loadConversation = async (conversationId: string) => {
    try {
      const msgs = await copilotRepository.getConversation(conversationId);
      setMessages(msgs);
      setSuggestedActions([]);
    } catch (error) {
      console.error("Failed to load conversation:", error);
    }
  };

  const loadCurrentWorkspace = async () => {
    try {
      const repo = getWorkspaceRepository();
      const workspaces = await repo.listActiveWorkspaces();
      if (workspaces.length > 0) {
        setCurrentWorkspace(workspaces[0].name);
      }
    } catch (error) {
      console.error("Failed to load workspace:", error);
    }
  };

  const loadDailyBriefing = async () => {
    setLoadingBriefing(true);
    try {
      const briefing = await copilotRepository.getDailyBriefing({
        workspace_id: null, // Load for all workspaces
      });
      setDailyBriefing(briefing);
      setShowBriefing(true);
    } catch (error) {
      console.error("Failed to load daily briefing:", error);
    } finally {
      setLoadingBriefing(false);
    }
  };

  const handleSendMessage = async (content: string) => {
    if (!content.trim() || isGenerating) return;

    setIsGenerating(true);
    abortControllerRef.current = new AbortController();

    // Optimistically add user message
    const userMessage: Message = {
      id: crypto.randomUUID(),
      conversation_id: currentConversationId || "",
      role: "user",
      content,
      tool_calls: null,
      reasoning: null,
      sources: null,
      created_at: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setSuggestedActions([]);

    try {
      const response = await copilotRepository.sendMessage({
        conversation_id: currentConversationId,
        workspace_id: null,
        message: content,
        include_context: true,
      });

      // Update conversation ID if new
      if (!currentConversationId) {
        setCurrentConversationId(response.conversation_id);
      }

      // Add assistant message
      setMessages((prev) => [...prev, response.message]);
      setSuggestedActions(response.suggested_actions);

      // Refresh conversations list
      await loadConversations();
    } catch (error) {
      console.error("Failed to send message:", error);
      // Remove optimistic user message on error
      setMessages((prev) => prev.filter((m) => m.id !== userMessage.id));
    } finally {
      setIsGenerating(false);
      abortControllerRef.current = null;
    }
  };

  const handleStopGeneration = () => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
      abortControllerRef.current = null;
    }
    setIsGenerating(false);
  };

  const handleNewConversation = () => {
    setCurrentConversationId(null);
    setMessages([]);
    setSuggestedActions([]);
    setShowBriefing(false);
  };

  const handleSelectConversation = (conversationId: string) => {
    setCurrentConversationId(conversationId);
    setShowBriefing(false);
  };

  const handleDeleteConversation = async (conversationId: string) => {
    // Note: Backend doesn't have delete endpoint yet, so we just remove from local state
    setConversations((prev) => prev.filter((c) => c.id !== conversationId));
    if (conversationId === currentConversationId) {
      handleNewConversation();
    }
  };

  const handleExecuteAction = async (action: SuggestedAction) => {
    await handleSendMessage(action.title);
  };

  const isEmpty = messages.length === 0 && !showBriefing;

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <ConversationSidebar
        conversations={conversations}
        currentConversationId={currentConversationId}
        onSelect={handleSelectConversation}
        onNew={handleNewConversation}
        onDelete={handleDeleteConversation}
      />

      {/* Main Chat Area */}
      <div className="flex flex-1 flex-col">
        {/* Workspace Context */}
        <WorkspaceContext workspaceName={currentWorkspace} />

        {/* Messages */}
        <div className="flex-1 overflow-y-auto">
          {isEmpty ? (
            <div className="flex h-full flex-col items-center justify-center gap-6 px-8">
              <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-[hsl(199,89%,58%)] to-[hsl(199,89%,48%)] shadow-lg">
                <Sparkles className="h-8 w-8 text-white" />
              </div>
              <div className="text-center">
                <h2 className="font-(family-name:--font-display) text-2xl font-bold text-(--color-foreground)">
                  Welcome to ChronoDesk AI
                </h2>
                <p className="mt-2 text-sm text-(--color-muted-foreground)">
                  Ask me anything about your workspace, get daily briefings, or let me help you with
                  your tasks.
                </p>
              </div>

              {/* Quick Actions */}
              <div className="grid w-full max-w-2xl grid-cols-1 gap-3 md:grid-cols-2">
                <button
                  onClick={loadDailyBriefing}
                  className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-left transition-colors hover:border-(--color-accent) hover:bg-(--color-surface-hover)"
                >
                  <div className="text-sm font-medium text-(--color-foreground)">
                    Get Daily Briefing
                  </div>
                  <div className="mt-1 text-xs text-(--color-muted-foreground)">
                    Summary of today's activity and priorities
                  </div>
                </button>
                <button
                  onClick={() => handleSendMessage("What did I work on yesterday?")}
                  className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-left transition-colors hover:border-(--color-accent) hover:bg-(--color-surface-hover)"
                >
                  <div className="text-sm font-medium text-(--color-foreground)">
                    Yesterday's Work
                  </div>
                  <div className="mt-1 text-xs text-(--color-muted-foreground)">
                    Review what you accomplished
                  </div>
                </button>
                <button
                  onClick={() => handleSendMessage("What files have I been working on most?")}
                  className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-left transition-colors hover:border-(--color-accent) hover:bg-(--color-surface-hover)"
                >
                  <div className="text-sm font-medium text-(--color-foreground)">Top Files</div>
                  <div className="mt-1 text-xs text-(--color-muted-foreground)">
                    Most frequently edited files
                  </div>
                </button>
                <button
                  onClick={() => handleSendMessage("Show me my workflow patterns")}
                  className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4 text-left transition-colors hover:border-(--color-accent) hover:bg-(--color-surface-hover)"
                >
                  <div className="text-sm font-medium text-(--color-foreground)">
                    Workflow Patterns
                  </div>
                  <div className="mt-1 text-xs text-(--color-muted-foreground)">
                    Discover how you work
                  </div>
                </button>
              </div>
            </div>
          ) : showBriefing ? (
            <div className="p-6">
              <div className="mx-auto max-w-3xl">
                <DailyBriefingWidget
                  briefing={dailyBriefing}
                  loading={loadingBriefing}
                  onRefresh={loadDailyBriefing}
                />
              </div>
            </div>
          ) : (
            <>
              {messages.map((message) => (
                <ChatMessage
                  key={message.id}
                  message={message}
                  isStreaming={isGenerating && message === messages[messages.length - 1]}
                />
              ))}
              <div ref={messagesEndRef} />
            </>
          )}
        </div>

        {/* Suggested Actions */}
        {suggestedActions.length > 0 && (
          <SuggestedActions
            actions={suggestedActions}
            onExecute={handleExecuteAction}
            disabled={isGenerating}
          />
        )}

        {/* Input */}
        <ChatInput
          onSend={handleSendMessage}
          onStop={handleStopGeneration}
          disabled={false}
          isGenerating={isGenerating}
        />
      </div>
    </div>
  );
}
