// useAutonomousStream - subscribes to live `autonomous:session` snapshots
// and `autonomous:reasoning` events for a single session, and re-syncs on
// demand (reconnect) via IPC.

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { autonomousRepository } from "@/services/autonomousRepository";
import type {
  AutonomousSessionProgress,
  ReasoningEvent,
} from "@/types/autonomous";

export interface AutonomousStreamState {
  progress: AutonomousSessionProgress | null;
  reasoning: ReasoningEvent[];
  /** True while a snapshot is being fetched on mount/reconnect. */
  loading: boolean;
  error: string | null;
}

/**
 * `useAutonomousStream` mirrors the backend session lifecycle:
 * - On mount, fetch the current snapshot (`autonomous_get_progress`) to
 *   restore UI state, then subscribe to `autonomous:session`.
 * - Streamed `autonomous:session` snapshots overwrite local state so the
 *   page stays live without polling.
 * - `autonomous:reasoning` events for this session append to a bounded log.
 * - `refresh()` re-fetches and re-syncs (used after pause/resume/cancel and
 *   after reconnect).
 */
export function useAutonomousStream(sessionId: string | null) {
  const [state, setState] = useState<AutonomousStreamState>({
    progress: null,
    reasoning: [],
    loading: !!sessionId,
    error: null,
  });

  const unlistenRef = useRef<UnlistenFn | null>(null);

  const subscribe = useCallback(async () => {
    const unlisten = await listen<AutonomousSessionProgress>(
      "autonomous:session",
      (event) => {
        if (!sessionId || event.payload.session_id !== sessionId) {
          return;
        }
        setState((prev) => ({
          ...prev,
          progress: event.payload,
          loading: false,
          error: null,
        }));
      }
    );
    const unlistenReasoning = await listen<ReasoningEvent>(
      "autonomous:reasoning",
      (event) => {
        if (!sessionId || event.payload.session_id !== sessionId) {
          return;
        }
        setState((prev) => ({
          ...prev,
          reasoning: [...prev.reasoning, event.payload].slice(-200),
        }));
      }
    );
    return () => {
      unlisten();
      unlistenReasoning();
    };
  }, [sessionId]);

  const refresh = useCallback(async () => {
    if (!sessionId) {
      setState({ progress: null, reasoning: [], loading: false, error: null });
      return;
    }
    setState((prev) => ({ ...prev, loading: true, error: null }));
    try {
      const progress = await autonomousRepository.getProgress(sessionId);
      setState(() => ({
        progress,
        reasoning: progress.reasoning ?? [],
        loading: false,
        error: null,
      }));
    } catch (err) {
      setState((prev) => ({
        ...prev,
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      }));
    }
  }, [sessionId]);

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        await refresh();
        if (cancelled) return;
        const unlisten = await subscribe();
        unlistenRef.current = unlisten;
      } catch (err) {
        if (!cancelled) {
          console.error("Failed to initialise autonomous stream:", err);
        }
      }
    }

    void boot();

    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, [refresh, subscribe]);

  return {
    ...state,
    refresh,
  };
}