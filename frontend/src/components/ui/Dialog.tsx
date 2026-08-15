import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/utils/cn";
import { GlassSurface } from "@/components/ui/GlassSurface";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  footer?: ReactNode;
  className?: string;
}

/**
 * macOS-style sheet dialog on liquid glass. Owns the backdrop, Escape
 * handling, and focus-on-open so pages never duplicate dialog chrome.
 * Focus moves into the sheet when it opens and returns to the element
 * that opened it when it closes.
 *
 * Motion mirrors the skill's material rules: the sheet "materializes"
 * on the way in (opacity + scale + blur radius together) and exits
 * along the same path (scale down + blur out) instead of unmounting
 * with a hard cut. Exit plays at ~0.18s and unmounts on animation end.
 */
export function Dialog({ open, onClose, title, description, children, footer, className }: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const [closing, setClosing] = useState(false);

  const requestClose = useCallback(() => {
    setClosing(true);
  }, []);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !closing) requestClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, closing, requestClose]);

  useEffect(() => {
    if (!open) return;
    previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const focusTarget = panel?.querySelector<HTMLElement>("input, [tabindex], button");
    (focusTarget ?? panel)?.focus();
    return () => {
      previouslyFocusedRef.current?.focus?.();
      previouslyFocusedRef.current = null;
    };
  }, [open]);

  const handleAnimationEnd = () => {
    if (!closing) return;
    setClosing(false);
    onClose();
  };

  if (!open) return null;

  return (
    <div
      className={cn(
        "fixed inset-0 z-50 flex items-center justify-center bg-(--color-overlay) p-6 backdrop-blur-[6px]",
        closing ? "overlay-exit" : "overlay-enter",
      )}
      role="dialog"
      aria-modal="true"
      aria-label={typeof title === "string" ? title : undefined}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) requestClose();
      }}
    >
      <GlassSurface
        material="sheet"
        onAnimationEnd={handleAnimationEnd}
        className={cn(
          "w-full max-w-md rounded-2xl p-5 outline-none",
          closing ? "sheet-exit" : "sheet-enter",
          className,
        )}
      >
        <div ref={panelRef}>
          <h2 className="font-(family-name:--font-display) text-lg font-semibold tracking-tight text-(--color-foreground)">
            {title}
          </h2>
          {description && <p className="mt-1 text-sm text-(--color-muted-foreground) text-on-glass">{description}</p>}
          <div className="mt-4">{children}</div>
          {footer && <div className="mt-5 flex justify-end gap-2">{footer}</div>}
        </div>
      </GlassSurface>
    </div>
  );
}
