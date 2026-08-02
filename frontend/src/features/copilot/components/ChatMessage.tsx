// ChatMessage - Individual message with markdown rendering and tool visualization

import { memo, Suspense, lazy, useMemo } from "react";
import { User, Sparkles, Copy, ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/utils/cn";
import type { Message } from "@/types/copilot";
import { ToolExecutionCard } from "./ToolExecutionCard";
import { SourcesList } from "./SourcesList";
import { useState } from "react";

const MarkdownRenderer = lazy(() =>
  import("./MarkdownRenderer").then((module) => ({ default: module.MarkdownRenderer }))
);

interface ChatMessageProps {
  message: Message;
  isStreaming?: boolean;
}

function ChatMessageComponent({ message, isStreaming }: ChatMessageProps) {
  const [showReasoning, setShowReasoning] = useState(false);
  const isUser = message.role === "user";
  const isAssistant = message.role === "assistant";

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message.content);
  };

  const formattedTime = useMemo(() => {
    const date = new Date(message.created_at);
    return date.toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit" });
  }, [message.created_at]);

  return (
    <div
      className={cn(
        "group flex w-full gap-3 px-4 py-4",
        isUser && "bg-(--color-surface-raised)",
        isAssistant && "bg-(--color-surface)"
      )}
    >
      {/* Avatar */}
      <div
        className={cn(
          "flex h-8 w-8 shrink-0 items-center justify-center rounded-lg",
          isUser && "bg-(--color-accent-muted)",
          isAssistant && "bg-gradient-to-br from-[hsl(199,89%,58%)] to-[hsl(199,89%,48%)]"
        )}
      >
        {isUser ? (
          <User className="h-4 w-4 text-(--color-accent)" />
        ) : (
          <Sparkles className="h-4 w-4 text-white" />
        )}
      </div>

      {/* Content */}
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="flex items-baseline gap-2">
          <span className="text-sm font-semibold text-(--color-foreground)">
            {isUser ? "You" : "ChronoDesk AI"}
          </span>
          <span className="text-xs text-(--color-faint-foreground)">{formattedTime}</span>
        </div>

        {/* Message Content */}
        <div
          className={cn(
            "prose prose-invert max-w-none text-sm text-(--color-foreground)",
            isStreaming && "animate-pulse"
          )}
        >
          <Suspense fallback={<span>{message.content}</span>}>
            <MarkdownRenderer content={message.content} />
          </Suspense>
        </div>

        {/* Tool Executions */}
        {message.tool_calls && message.tool_calls.length > 0 && (
          <div className="mt-2 space-y-2">
            {message.tool_calls.map((toolCall) => (
              <ToolExecutionCard key={toolCall.id} toolCall={toolCall} />
            ))}
          </div>
        )}

        {/* Reasoning (Collapsible) */}
        {message.reasoning && (
          <div className="mt-2">
            <button
              onClick={() => setShowReasoning(!showReasoning)}
              className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
            >
              {showReasoning ? (
                <ChevronUp className="h-3.5 w-3.5" />
              ) : (
                <ChevronDown className="h-3.5 w-3.5" />
              )}
              <span>Reasoning</span>
            </button>
            {showReasoning && (
              <div className="mt-2 rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3 text-xs text-(--color-muted-foreground)">
                {message.reasoning}
              </div>
            )}
          </div>
        )}

        {/* Sources */}
        {message.sources && message.sources.length > 0 && (
          <SourcesList sources={message.sources} />
        )}

        {/* Actions */}
        {isAssistant && !isStreaming && (
          <div className="mt-1 flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
            <button
              onClick={handleCopy}
              className="rounded-md p-1.5 text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
              title="Copy message"
            >
              <Copy className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export const ChatMessage = memo(ChatMessageComponent);
