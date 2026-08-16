import { AnimatePresence, motion } from "motion/react";
import {
  cloneElement,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { cn } from "@/utils/cn";
import { springs } from "@/lib/springs";
import { useReducedMotion } from "@/utils/motion";

interface AnchorRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
}

export interface GlassMenuProps {
  /** The control that opens the menu (button or link). */
  trigger: ReactNode;
  /** Menu content: rows of actions, typically glass-control buttons. */
  children: ReactNode;
  /** Horizontal alignment against the trigger. */
  align?: "start" | "end";
  /** Which side of the trigger the bubble opens on. */
  side?: "bottom" | "top";
  /** Close when an interactive row (button/a/[role=menuitem]) is clicked. */
  closeOnItemClick?: boolean;
  /** Accessible name for the menu. */
  label?: string;
  className?: string;
  /** Optional controlled open state. */
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

/**
 * GlassMenu — Apple's "the bubble simply pops open" transition: content
 * stays exactly where the trigger is, the pane materializes with a spring
 * (scale + opacity + slight rise) anchored to the control, and exits
 * along the same path. Keyboard: Escape closes and returns focus to the
 * trigger; ArrowUp/Down move between focusable rows; focus moves into the
 * menu on open.
 */
export function GlassMenu({
  trigger,
  children,
  align = "start",
  side = "bottom",
  closeOnItemClick = false,
  label,
  className,
  open: controlledOpen,
  onOpenChange,
}: GlassMenuProps) {
  const [internalOpen, setInternalOpen] = useState(false);
  const open = controlledOpen ?? internalOpen;
  const setOpen = useCallback(
    (next: boolean) => {
      if (controlledOpen === undefined) setInternalOpen(next);
      onOpenChange?.(next);
    },
    [controlledOpen, onOpenChange],
  );

  const wrapperRef = useRef<HTMLSpanElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const [anchor, setAnchor] = useState<AnchorRect | null>(null);
  const titleId = useId();
  const reduced = useReducedMotion();

  const measure = useCallback(() => {
    const el = triggerRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setAnchor({ left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width });
  }, []);

  useEffect(() => {
    if (!open) return;
    measure();
    const onScroll = () => measure();
    const onResize = () => measure();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onResize);
    };
  }, [open, measure]);

  // Outside click + Escape.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node;
      if (wrapperRef.current?.contains(target)) return;
      if (panelRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open, setOpen]);

  // Focus management: move focus into the menu when it opens; the panel
  // closes with focus returning to the trigger.
  useEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    const first = panel?.querySelector<HTMLElement>("[role='menuitem'], button, a, input");
    (first ?? panel)?.focus();
  }, [open]);

  const handlePanelClick = (e: React.MouseEvent) => {
    if (!closeOnItemClick) return;
    const target = e.target as HTMLElement;
    if (target.closest("[role='menuitem'], button, a")) {
      setOpen(false);
      triggerRef.current?.focus();
    }
  };

  const triggerNode = trigger as React.ReactElement<Record<string, unknown>>;

  return (
    <span ref={wrapperRef} className="relative inline-flex">
      {cloneElement(triggerNode, {
        ref: triggerRef,
        "aria-expanded": open,
        "aria-haspopup": "menu",
        "aria-controls": titleId,
        onClick: (e: React.MouseEvent) => {
          (triggerNode.props.onClick as React.MouseEventHandler | undefined)?.(e);
          setOpen(!open);
        },
      })}

      <AnimatePresence>
        {open && anchor && (
          <motion.div
            key="menu"
            id={titleId}
            ref={panelRef}
            role="menu"
            aria-label={label}
            tabIndex={-1}
            className={cn(
              "glass-sheet fixed z-50 min-w-44 rounded-2xl p-1.5 outline-none",
              align === "end" && "-translate-x-full",
              side === "top" && "-translate-y-full",
              className,
            )}
            style={{
              left: align === "start" ? anchor.left : anchor.right,
              top: side === "bottom" ? anchor.bottom + 8 : anchor.top - 8,
            }}
            initial={{ opacity: 0, scale: reduced ? 1 : 0.95, y: side === "bottom" ? 6 : -6 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: reduced ? 1 : 0.97, y: side === "bottom" ? 4 : -4 }}
            transition={springs.material}
            onClick={handlePanelClick}
          >
            {children}
          </motion.div>
        )}
      </AnimatePresence>
    </span>
  );
}