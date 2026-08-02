import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { copilotRepository } from "@/services/copilotRepository";
import type { StreamEventPayload, StreamingDiagnostics } from "@/types/copilot";

interface ActiveStream {
  streamId: string;
  messageId: string;
}

interface UseCopilotStreamingOptions {
  onChunk: (messageId: string, content: string) => void;
  onFinished: (conversationId: string) => void;
  onCancelled: (messageId: string) => void;
  onError: (messageId: string, error: string) => void;
  onTerminal: () => void;
}

export function useCopilotStreaming(options: UseCopilotStreamingOptions) {
  const [diagnostics, setDiagnostics] = useState<StreamingDiagnostics | null>(null);
  const [streamError, setStreamError] = useState<string | null>(null);
  const activeStreamRef = useRef<ActiveStream | null>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    let cancelled = false;
    const unlistenFns: UnlistenFn[] = [];

    async function subscribe() {
      const handlers: Record<string, (payload: StreamEventPayload) => void> = {
        stream_chunk: handleStreamChunk,
        stream_finished: handleStreamFinished,
        stream_cancelled: handleStreamCancelled,
        stream_error: handleStreamError,
      };

      try {
        for (const [eventName, handler] of Object.entries(handlers)) {
          const unlisten = await listen<StreamEventPayload>(eventName, (event) => {
            handler(event.payload);
          });
          if (cancelled) {
            unlisten();
            continue;
          }
          unlistenFns.push(unlisten);
        }
      } catch (error) {
        console.error("Failed to subscribe to copilot streaming events:", error);
      }
    }

    void subscribe();

    return () => {
      cancelled = true;
      unlistenFns.forEach((unlisten) => unlisten());
    };
  }, []);

  async function loadDiagnostics() {
    try {
      const nextDiagnostics = await copilotRepository.getStreamingDiagnostics();
      setDiagnostics(nextDiagnostics);
    } catch (error) {
      console.error("Failed to load streaming diagnostics:", error);
    }
  }

  function startStream(streamId: string, messageId: string) {
    activeStreamRef.current = { streamId, messageId };
  }

  async function stopStream() {
    const activeStream = activeStreamRef.current;
    if (!activeStream) return;

    try {
      await copilotRepository.cancelStream(activeStream.streamId);
    } catch (error) {
      console.error("Failed to stop generation:", error);
    }
  }

  function clearActiveStream() {
    activeStreamRef.current = null;
  }

  function isStreamingMessage(messageId: string) {
    return activeStreamRef.current?.messageId === messageId;
  }

  function matchingActiveStream(payload: StreamEventPayload) {
    const activeStream = activeStreamRef.current;
    if (!activeStream || payload.stream_id !== activeStream.streamId) return null;
    return activeStream;
  }

  function handleStreamChunk(payload: StreamEventPayload) {
    const activeStream = matchingActiveStream(payload);
    if (!activeStream || !payload.content) return;
    optionsRef.current.onChunk(activeStream.messageId, payload.content);
  }

  function handleStreamFinished(payload: StreamEventPayload) {
    const activeStream = matchingActiveStream(payload);
    if (!activeStream) return;
    clearActiveStream();
    optionsRef.current.onFinished(payload.conversation_id);
    optionsRef.current.onTerminal();
    void loadDiagnostics();
  }

  function handleStreamCancelled(payload: StreamEventPayload) {
    const activeStream = matchingActiveStream(payload);
    if (!activeStream) return;
    clearActiveStream();
    optionsRef.current.onCancelled(activeStream.messageId);
    optionsRef.current.onTerminal();
    void loadDiagnostics();
  }

  function handleStreamError(payload: StreamEventPayload) {
    const activeStream = matchingActiveStream(payload);
    if (!activeStream) return;
    const error = payload.error || "Streaming response failed";
    clearActiveStream();
    setStreamError(error);
    optionsRef.current.onError(activeStream.messageId, error);
    optionsRef.current.onTerminal();
    void loadDiagnostics();
  }

  return {
    activeStreamRef,
    diagnostics,
    streamError,
    setStreamError,
    startStream,
    stopStream,
    clearActiveStream,
    isStreamingMessage,
    loadDiagnostics,
  };
}
