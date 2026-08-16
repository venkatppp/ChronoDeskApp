import { useEffect, useState } from "react";

/**
 * Window focus tracking for the chrome recede behavior (macOS: glass
 * recedes when the window loses focus). Uses the Tauri webview's native
 * focus events when running inside the desktop shell, and falls back to
 * the DOM window focus/blur events for browser dev.
 */
export function useWindowFocus(): boolean {
  const [focused, setFocused] = useState(true);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    if (isTauri) {
      import("@tauri-apps/api/webviewWindow")
        .then(({ getCurrentWebviewWindow }) => {
          if (!mounted) return;
          const win = getCurrentWebviewWindow();
          win
            .onFocusChanged(({ payload }) => {
              if (mounted) setFocused(payload);
            })
            .then((fn) => {
              if (!mounted) fn();
              else unlisten = fn;
            })
            .catch(() => {});
        })
        .catch(() => {});
    }

    const onBlur = () => setFocused(false);
    const onFocus = () => setFocused(true);
    window.addEventListener("blur", onBlur);
    window.addEventListener("focus", onFocus);

    return () => {
      mounted = false;
      unlisten?.();
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  return focused;
}