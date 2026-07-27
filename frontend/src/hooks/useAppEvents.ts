import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * Subscribes to one or more backend events (emitted via
 * `app_events::emit` on the Rust side — see `src-tauri/src/app_events.rs`
 * for the exact event name constants) and calls `onEvent` whenever any of
 * them fires, for as long as the calling component is mounted.
 *
 * This is the piece that makes the dashboard "just update" instead of
 * needing a manual refresh: `useDashboardData` passes a callback that
 * re-runs its fetch, subscribed to `workspace:created`/`:updated`/
 * `:deleted` and `timeline:event_added`.
 */
export function useAppEvents(eventNames: string[], onEvent: () => void) {
  // Keep the latest callback in a ref so the effect below doesn't need
  // `onEvent` in its dependency array — re-subscribing on every render
  // (which a fresh inline callback would otherwise cause) would mean a
  // brief window on every render where events could be missed between
  // unlisten and re-listen.
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    let cancelled = false;
    const unlistenFns: UnlistenFn[] = [];

    async function subscribe() {
      const results = await Promise.all(
        eventNames.map((name) => listen(name, () => onEventRef.current())),
      );
      if (cancelled) {
        // Component unmounted while the listen() calls were in flight —
        // clean up immediately instead of leaking the subscriptions.
        results.forEach((unlisten) => unlisten());
        return;
      }
      unlistenFns.push(...results);
    }

    void subscribe();

    return () => {
      cancelled = true;
      unlistenFns.forEach((unlisten) => unlisten());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- eventNames is expected to be a stable literal array at each call site.
  }, [eventNames.join(",")]);
}
