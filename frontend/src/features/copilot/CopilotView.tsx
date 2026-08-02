// CopilotView - Main AI assistant interface with chat, sidebar, and proactive features

import { useCallback, useEffect, useRef, useState } from "react";
import { Activity, Bell, Sparkles } from "lucide-react";
import { copilotRepository } from "@/services/copilotRepository";
import { llmRepository } from "@/services/llmRepository";
import { proactiveRepository } from "@/services/proactiveRepository";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type {
  Message,
  Conversation,
  ConversationSearchRequest,
  ConversationSearchResult,
  SuggestedAction,
  DailyBriefing,
} from "@/types/copilot";
import type { LLMProviderDiagnostics } from "@/types/llm";
import type { ProactiveNotification, ResumeContext, ExecutionPlan, PermissionLevel } from "@/types/proactive";
import { ChatMessage } from "./components/ChatMessage";
import { ChatInput } from "./components/ChatInput";
import { ConversationSidebar } from "./components/ConversationSidebar";
import { WorkspaceContext } from "./components/WorkspaceContext";
import { SuggestedActions } from "./components/SuggestedActions";
import { DailyBriefingWidget } from "./components/DailyBriefingWidget";
import { ProactiveNotificationCard } from "./components/ProactiveNotificationCard";
import { ExecutionPlanCard } from "./components/ExecutionPlanCard";
import { ResumeWorkspaceBanner } from "./components/ResumeWorkspaceBanner";
import { useCopilotStreaming } from "./useCopilotStreaming";

export function CopilotView() {
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [suggestedActions, setSuggestedActions] = useState<SuggestedAction[]>([]);
  const [isGenerating, setIsGenerating] = useState(false);
  const [currentWorkspace, setCurrentWorkspace] = useState<string | null>(null);
  const [currentWorkspaceId, setCurrentWorkspaceId] = useState<string | null>(null);
  const [dailyBriefing, setDailyBriefing] = useState<DailyBriefing | null>(null);
  const [showBriefing, setShowBriefing] = useState(false);
  const [loadingBriefing, setLoadingBriefing] = useState(false);
  const [searchResults, setSearchResults] = useState<ConversationSearchResult[] | null>(null);
  const [isSearching, setIsSearching] = useState(false);
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const [llmDiagnostics, setLlmDiagnostics] = useState<LLMProviderDiagnostics | null>(null);
  const [isLlmConfigured, setIsLlmConfigured] = useState(false);
  const [showDiagnostics, setShowDiagnostics] = useState(false);

  // Proactive features state
  const [notifications, setNotifications] = useState<ProactiveNotification[]>([]);
  const [resumeContext, setResumeContext] = useState<ResumeContext | null>(null);
  const [executionPlan, setExecutionPlan] = useState<ExecutionPlan | null>(null);
  const [showNotifications, setShowNotifications] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const streaming = useCopilotStreaming({
    onChunk: (messageId, content) => {
      setMessages((prev) =>
        prev.map((message) =>
          message.id === messageId ? { ...message, content: message.content + content } : message
        )
      );
    },
    onFinished: (conversationId) => {
      void loadConversation(conversationId);
    },
    onCancelled: (messageId) => {
      setMessages((prev) => prev.filter((message) => message.id !== messageId));
    },
    onError: (messageId) => {
      setMessages((prev) => prev.filter((message) => message.id !== messageId));
    },
    onTerminal: () => {
      setIsGenerating(false);
      void loadConversations();
    },
  });

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
    loadProactiveNotifications();
    streaming.loadDiagnostics();
    loadLlmDiagnostics();
  }, []);

  // Load conversation messages when selected
  useEffect(() => {
    if (currentConversationId && !streaming.activeStreamRef.current) {
      loadConversation(currentConversationId);
    }
  }, [currentConversationId]);

  // Check for resume context when workspace loads
  useEffect(() => {
    if (currentWorkspaceId && !resumeContext) {
      checkResumeContext();
    }
  }, [currentWorkspaceId]);

  const loadConversations = useCallback(async () => {
    try {
      const convs = await copilotRepository.getRecentConversations(50);
      setConversations(convs);
    } catch (error) {
      console.error("Failed to load conversations:", error);
    }
  }, []);

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
        setCurrentWorkspaceId(workspaces[0].id);
      }
    } catch (error) {
      console.error("Failed to load workspace:", error);
    }
  };

  const loadProactiveNotifications = async () => {
    try {
      const notifs = await proactiveRepository.getNotifications();
      setNotifications(notifs);
    } catch (error) {
      console.error("Failed to load notifications:", error);
    }
  };

  const checkResumeContext = async () => {
    if (!currentWorkspaceId) return;
    try {
      const context = await proactiveRepository.getResumeContext(currentWorkspaceId);
      if (context.unfinished_work.length > 0) {
        setResumeContext(context);
      }
    } catch (error) {
      console.error("Failed to load resume context:", error);
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

  const loadLlmDiagnostics = async () => {
    try {
      const [configured, diagnostics] = await Promise.all([
        llmRepository.isConfigured(),
        llmRepository.getDiagnostics(),
      ]);
      setIsLlmConfigured(configured);
      setLlmDiagnostics(diagnostics);
    } catch (error) {
      console.error("Failed to load LLM diagnostics:", error);
    }
  };

  const handleSearchConversations = useCallback(async (request: ConversationSearchRequest) => {
    setIsSearching(true);
    try {
      const results = await copilotRepository.searchConversations(request);
      setSearchResults(results);
    } catch (error) {
      console.error("Failed to search conversations:", error);
      setSearchResults([]);
    } finally {
      setIsSearching(false);
    }
  }, []);

  const handleExportConversation = async (conversationId: string, format: "markdown" | "json") => {
    try {
      const exported =
        format === "markdown"
          ? await copilotRepository.exportConversationMarkdown(conversationId)
          : await copilotRepository.exportConversationJson(conversationId);
      await navigator.clipboard.writeText(exported);
      setExportStatus(`${format === "markdown" ? "Markdown" : "JSON"} export copied to clipboard`);
      window.setTimeout(() => setExportStatus(null), 3000);
    } catch (error) {
      console.error("Failed to export conversation:", error);
      setExportStatus("Export failed");
    }
  };

  const handleSendMessage = async (content: string) => {
    if (!content.trim() || isGenerating) return;

    setIsGenerating(true);
    streaming.setStreamError(null);

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
      const response = await copilotRepository.sendMessageStream({
        conversation_id: currentConversationId,
        workspace_id: null,
        message: content,
        include_context: true,
      });

      const assistantMessage: Message = {
        id: `stream-${response.stream_id}`,
        conversation_id: response.conversation_id,
        role: "assistant",
        content: "",
        tool_calls: null,
        reasoning: null,
        sources: null,
        created_at: new Date().toISOString(),
      };

      streaming.startStream(response.stream_id, assistantMessage.id);

      // Update conversation ID if new after registering the active stream so the
      // selection effect does not replace the in-flight placeholder message.
      if (!currentConversationId) {
        setCurrentConversationId(response.conversation_id);
      }

      setMessages((prev) => [...prev, assistantMessage]);
      await streaming.loadDiagnostics();
    } catch (error) {
      console.error("Failed to send message:", error);
      // Remove optimistic user message on error
      setMessages((prev) => prev.filter((m) => m.id !== userMessage.id));
      setIsGenerating(false);
      streaming.clearActiveStream();
    }
  };

  const handleStopGeneration = async () => {
    if (!streaming.activeStreamRef.current) {
      setIsGenerating(false);
      return;
    }
    await streaming.stopStream();
  };

  const handleNewConversation = () => {
    setCurrentConversationId(null);
    setMessages([]);
    setSuggestedActions([]);
    setShowBriefing(false);
    streaming.setStreamError(null);
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

  const handleDismissNotification = async (notificationId: string) => {
    try {
      await proactiveRepository.dismissNotification(notificationId);
      setNotifications((prev) => prev.filter((n) => n.id !== notificationId));
    } catch (error) {
      console.error("Failed to dismiss notification:", error);
    }
  };

  const handleNotificationAction = async (action: string) => {
    await handleSendMessage(action);
  };

  const handleResumeWork = async () => {
    if (!resumeContext) return;
    const summary = resumeContext.unfinished_work.map((w) => w.description).join(", ");
    await handleSendMessage(`Help me resume: ${summary}`);
    setResumeContext(null);
  };

  const handleDismissResume = () => {
    setResumeContext(null);
  };

  const handleApprovePlan = async (_planId: string, permission: PermissionLevel) => {
    try {
      await proactiveRepository.setPermission("execute_plan", permission, currentWorkspaceId || undefined);
      // In production, this would trigger plan execution
      setExecutionPlan(null);
      await handleSendMessage("Execute the approved plan");
    } catch (error) {
      console.error("Failed to approve plan:", error);
    }
  };

  const handleRejectPlan = () => {
    setExecutionPlan(null);
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
        onSearch={handleSearchConversations}
        onExport={handleExportConversation}
        searchResults={searchResults}
        isSearching={isSearching}
        workspaceId={currentWorkspaceId}
      />

      {/* Main Chat Area */}
      <div className="flex flex-1 flex-col">
        {/* Workspace Context */}
        <WorkspaceContext workspaceName={currentWorkspace} />

        {/* Notification Bell */}
        <div className="flex items-center justify-end gap-2 border-b border-(--color-border-subtle) px-4 py-2">
          <button
            onClick={() => {
              setShowDiagnostics(!showDiagnostics);
              void loadLlmDiagnostics();
              void streaming.loadDiagnostics();
            }}
            className="rounded-lg p-2 text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
            title="AI health diagnostics"
            aria-label="Toggle AI health diagnostics"
          >
            <Activity className="h-5 w-5" />
          </button>
          <button
            onClick={() => setShowNotifications(!showNotifications)}
            className="relative rounded-lg p-2 text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
            title="Proactive notifications"
            aria-label="Toggle proactive notifications"
          >
            <Bell className="h-5 w-5" />
            {notifications.length > 0 && (
              <span className="absolute right-1 top-1 flex h-4 w-4 items-center justify-center rounded-full bg-(--color-accent) text-[10px] font-bold text-(--color-accent-foreground)">
                {notifications.length}
              </span>
            )}
          </button>
        </div>

        {showDiagnostics && (
          <div className="border-b border-(--color-border-subtle) bg-(--color-surface-raised) px-4 py-3">
            <div className="grid gap-3 text-xs text-(--color-muted-foreground) md:grid-cols-4">
              <DiagnosticItem label="Connection" value={isLlmConfigured ? "Configured" : "Not configured"} />
              <DiagnosticItem label="Provider" value={llmDiagnostics?.provider || "Unavailable"} />
              <DiagnosticItem label="Circuit" value={llmDiagnostics?.circuit_breaker_state || "Unknown"} />
              <DiagnosticItem label="Avg latency" value={`${(llmDiagnostics?.average_latency_ms || 0).toFixed(0)} ms`} />
              <DiagnosticItem label="Retries" value={String(llmDiagnostics?.retries || 0)} />
              <DiagnosticItem label="Rate limited" value={String(llmDiagnostics?.rate_limited_requests || 0)} />
              <DiagnosticItem label="Requests" value={String(llmDiagnostics?.total_requests || 0)} />
              <DiagnosticItem
                label="Streaming"
                value={`${streaming.diagnostics?.active_streams || 0} active / ${(
                  (streaming.diagnostics?.provider_streaming_health || 1) * 100
                ).toFixed(0)}% health`}
              />
            </div>
          </div>
        )}

        {/* Messages */}
        <div className="flex-1 overflow-y-auto">
          {isEmpty ? (
            <div className="flex h-full flex-col items-center justify-center gap-6 px-8">
              {/* Resume Context Banner */}
              {resumeContext && (
                <div className="w-full max-w-2xl">
                  <ResumeWorkspaceBanner
                    context={resumeContext}
                    onResume={handleResumeWork}
                    onDismiss={handleDismissResume}
                  />
                </div>
              )}

              {/* Proactive Notifications */}
              {showNotifications && notifications.length > 0 && (
                <div className="w-full max-w-2xl space-y-3">
                  <h3 className="text-sm font-medium text-(--color-foreground)">
                    Proactive Suggestions
                  </h3>
                  {notifications.map((notification) => (
                    <ProactiveNotificationCard
                      key={notification.id}
                      notification={notification}
                      onDismiss={handleDismissNotification}
                      onActionClick={handleNotificationAction}
                    />
                  ))}
                </div>
              )}

              {/* Execution Plan */}
              {executionPlan && (
                <div className="w-full max-w-2xl">
                  <ExecutionPlanCard
                    plan={executionPlan}
                    onApprove={handleApprovePlan}
                    onReject={handleRejectPlan}
                  />
                </div>
              )}

              {!resumeContext && !showNotifications && !executionPlan && (
                <>
                  <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-[hsl(199,89%,58%)] to-[hsl(199,89%,48%)] shadow-lg">
                    <Sparkles className="h-8 w-8 text-white" />
                  </div>
                  <div className="text-center">
                    <h2 className="font-(family-name:--font-display) text-2xl font-bold text-(--color-foreground)">
                      Welcome to ChronoDesk AI
                    </h2>
                    <p className="mt-2 text-sm text-(--color-muted-foreground)">
                      Ask me anything about your workspace, get daily briefings, or let me help you
                      with your tasks.
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
                </>
              )}
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
                  isStreaming={streaming.isStreamingMessage(message.id)}
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

        {(streaming.streamError || streaming.diagnostics) && (
          <div className="border-t border-(--color-border-subtle) px-4 py-2 text-xs text-(--color-muted-foreground)">
            {streaming.streamError ? (
              <span className="text-(--color-danger)">{streaming.streamError}</span>
            ) : streaming.diagnostics ? (
              <span>
                Streaming health {(streaming.diagnostics.provider_streaming_health * 100).toFixed(0)}% ·
                {" "}{streaming.diagnostics.active_streams} active ·
                {" "}{streaming.diagnostics.average_tokens_per_second.toFixed(1)} tokens/s
              </span>
            ) : null}
          </div>
        )}

        {exportStatus && (
          <div className="border-t border-(--color-border-subtle) px-4 py-2 text-xs text-(--color-muted-foreground)" role="status">
            {exportStatus}
          </div>
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

function DiagnosticItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-(--color-border-subtle) bg-(--color-surface) p-2">
      <div className="text-[11px] uppercase tracking-wide text-(--color-faint-foreground)">{label}</div>
      <div className="mt-1 truncate font-medium text-(--color-foreground)">{value}</div>
    </div>
  );
}
