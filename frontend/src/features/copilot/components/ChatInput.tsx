// ChatInput - Message input with auto-resize and keyboard shortcuts

import { Send, StopCircle } from "lucide-react";
import { useState, useRef, useEffect, type KeyboardEvent } from "react";
import { cn } from "@/utils/cn";

interface ChatInputProps {
  onSend: (message: string) => void;
  onStop?: () => void;
  disabled?: boolean;
  isGenerating?: boolean;
  placeholder?: string;
}

export function ChatInput({
  onSend,
  onStop,
  disabled,
  isGenerating,
  placeholder = "Ask ChronoDesk anything...",
}: ChatInputProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea
  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
  }, [value]);

  const handleSubmit = () => {
    if (!value.trim() || disabled || isGenerating) return;
    onSend(value.trim());
    setValue("");
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const handleStop = () => {
    if (onStop) onStop();
  };

  return (
    <div className="border-t border-(--color-border) bg-(--color-surface) p-4">
      <div className="relative flex items-end gap-2">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={disabled || isGenerating}
          rows={1}
          className={cn(
            "min-h-[44px] w-full resize-none rounded-lg border border-(--color-border) bg-(--color-surface-raised) px-4 py-3 pr-12",
            "text-sm text-(--color-foreground) placeholder:text-(--color-muted-foreground)",
            "focus:border-(--color-accent) focus:outline-none focus:ring-1 focus:ring-(--color-accent)",
            "disabled:cursor-not-allowed disabled:opacity-50",
            "transition-colors"
          )}
        />

        {isGenerating ? (
          <button
            onClick={handleStop}
            className="absolute bottom-2 right-2 flex h-8 w-8 items-center justify-center rounded-lg bg-(--color-danger) text-white transition-colors hover:bg-(--color-danger)/90"
            title="Stop generating"
          >
            <StopCircle className="h-4 w-4" />
          </button>
        ) : (
          <button
            onClick={handleSubmit}
            disabled={!value.trim() || disabled}
            className={cn(
              "absolute bottom-2 right-2 flex h-8 w-8 items-center justify-center rounded-lg bg-(--color-accent) text-(--color-accent-foreground) transition-colors",
              "hover:bg-(--color-accent)/90",
              "disabled:cursor-not-allowed disabled:opacity-50"
            )}
            title="Send message (Enter)"
          >
            <Send className="h-4 w-4" />
          </button>
        )}
      </div>

      <div className="mt-2 flex items-center justify-between text-xs text-(--color-faint-foreground)">
        <span>Press Enter to send, Shift+Enter for new line</span>
        <span>{value.length} characters</span>
      </div>
    </div>
  );
}
