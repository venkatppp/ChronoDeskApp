// useExecutionStream - subscribes to live `execution:progress` events for a
// single execution and re-syncs on demand (reconnect) via IPC.

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { executionRepository } from "@/services/executionRepository";
import type { ExecutionProgress } from "@/types/execution";

export interface ExecutionStreamState {
  progress: ExecutionProgress | null;
  /** True while a snapshot is being fetched on mount/reconnect. */
  loading: boolean;
  error: string | null;
}

/**
 * `useExecutionStream` mirrors the backend lifecycle:
 * - On mount, fetch the current progress (`execution_get_progress`) to
 *   restore UI state, then subscribe to `execution:progress`.
 * - Every streamed snapshot for `executionId` overwrites local state so the
 *   dashboard stays live without polling.
 * - `refresh()` re-fetches and re-syncs (used after pause/resume/cancel and
 *   after reconnect).
 */
export function useExecutionStream(executionId: string | null) {
  const [state, setState] = useState<ExecutionStreamState>({
    progress: null,
    loading: !!executionId,
    error: null,
  });

  const unlistenRef = useRef<UnlistenFn | null>(null);

  const subscribe = useCallback(async () => {
    const unlisten = await listen<ExecutionProgress>("execution:progress", (event) => {
      if (!executionId || event.payload.execution_id !== executionId) {
        return;
      }
      setState({ progress: event.payload, loading: false, error: null });
    });
    unlistenRef.current = unlisten;
  }, [executionId]);

  const refresh = useCallback(async () => {
    if (!executionId) {
      setState({ progress: null, loading: false, error: null });
      return;
    }
    setState((prev) => ({ ...prev, loading: true, error: null }));
    try {
      const progress = await executionRepository.getProgress(executionId);
      setState({ progress, loading: false, error: null });
    } catch (err) {
      setState({
        progress: null,
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }, [executionId]);

  useEffect(() => {
    let cancelled = false;

    async function boot() {
      try {
        await refresh();
        if (cancelled) return;
        await subscribe();
      } catch (err) {
        if (!cancelled) {
          console.error("Failed to initialise execution stream:", err);
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