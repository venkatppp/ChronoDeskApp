import { useEffect, useRef, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { cn } from "@/utils/cn";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { springs } from "@/lib/springs";
import { useReducedMotion } from "@/utils/motion";

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
 * Motion mirrors Apple's material rules: the sheet "materializes" on the
 * way in — spring scale + opacity + a slight rise — and dematerializes
 * along the same path on exit. Under prefers-reduced-motion this becomes
 * an opacity cross-fade.
 */
export function Dialog({ open, onClose, title, description, children, footer, className }: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const reduced = useReducedMotion();

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

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

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          key="dialog-root"
          className="fixed inset-0 z-50 flex items-center justify-center bg-(--color-overlay) p-6 backdrop-blur-[6px]"
          role="dialog"
          aria-modal="true"
          aria-label={typeof title === "string" ? title : undefined}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={springs.overlay}
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) onClose();
          }}
        >
          <GlassSurface
            material="sheet"
            className={cn("w-full max-w-md rounded-2xl p-5 outline-none", className)}
          >
            <motion.div
              ref={panelRef}
              initial={{ opacity: 0, scale: reduced ? 1 : 0.95, y: reduced ? 0 : 10 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: reduced ? 1 : 0.96, y: reduced ? 0 : 8 }}
              transition={springs.material}
            >
              <h2 className="font-(family-name:--font-display) text-lg font-semibold tracking-tight text-(--color-foreground)">
                {title}
              </h2>
              {description && <p className="mt-1 text-sm text-(--color-muted-foreground) text-on-glass">{description}</p>}
              <div className="mt-4">{children}</div>
              {footer && <div className="mt-5 flex justify-end gap-2">{footer}</div>}
            </motion.div>
          </GlassSurface>
        </motion.div>
      )}
    </AnimatePresence>
  );
}