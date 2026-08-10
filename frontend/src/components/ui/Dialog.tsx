import { useEffect, type ReactNode } from "react";
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
 */
export function Dialog({ open, onClose, title, description, children, footer, className }: DialogProps) {
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-(--color-overlay) p-6"
      role="dialog"
      aria-modal="true"
      aria-label={typeof title === "string" ? title : undefined}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <GlassSurface
        material="sheet"
        className={cn(
          "w-full max-w-md animate-scale-in rounded-2xl p-5",
          className,
        )}
      >
        <h2 className="font-(family-name:--font-display) text-lg font-semibold tracking-tight text-(--color-foreground)">
          {title}
        </h2>
        {description && <p className="mt-1 text-sm text-(--color-muted-foreground)">{description}</p>}
        <div className="mt-4">{children}</div>
        {footer && <div className="mt-5 flex justify-end gap-2">{footer}</div>}
      </GlassSurface>
    </div>
  );
}
