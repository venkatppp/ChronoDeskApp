import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/utils/cn";
import { useReducedMotion } from "@/utils/motion";

interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
  count?: number;
}

interface SegmentedControlProps<T extends string> {
  options: SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
  ariaLabel?: string;
}

/**
 * macOS-style segmented control — translucent well, raised selection,
 * quiet text. The selection is a single measured pill that slides to the
 * active segment with premium easing (instant under prefers-reduced-motion).
 * Shared by every filter/tab group in the app.
 */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
  ariaLabel,
}: SegmentedControlProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const optionRefs = useRef(new Map<string, HTMLButtonElement>());
  const reducedMotion = useReducedMotion();
  const [pill, setPill] = useState({ left: 0, width: 0 });

  useLayoutEffect(() => {
    const container = containerRef.current;
    const active = optionRefs.current.get(value);
    if (!container || !active) return;

    // offsetLeft is already relative to the container (its offsetParent).
    const measure = () => {
      setPill({ left: active.offsetLeft, width: active.offsetWidth });
    };
    measure();

    if (typeof ResizeObserver === "function") {
      const ro = new ResizeObserver(measure);
      ro.observe(container);
      window.addEventListener("resize", measure);
      return () => {
        ro.disconnect();
        window.removeEventListener("resize", measure);
      };
    }
  }, [value]);

  return (
    <div
      ref={containerRef}
      role="group"
      aria-label={ariaLabel}
      className={cn(
        "glass-control relative inline-flex items-center gap-0.5 rounded-[var(--radius-control)] p-0.5",
        className,
      )}
    >
      <span
        aria-hidden
        className={cn(
          "pointer-events-none absolute top-0.5 bottom-0.5 rounded-[calc(var(--radius-control)-3px)] bg-(--color-surface-hover) shadow-[inset_0_1px_0_rgba(255,255,255,0.1),0_1px_2px_rgba(0,0,0,0.35)]",
          reducedMotion ? "" : "transition-[left,width] duration-300 ease-[var(--ease-premium)]",
        )}
        style={{ left: pill.left, width: pill.width }}
      />
      {options.map((option) => {
        const isActive = option.value === value;
        return (
          <button
            key={option.value}
            ref={(node) => {
              if (node) optionRefs.current.set(option.value, node);
              else optionRefs.current.delete(option.value);
            }}
            type="button"
            aria-pressed={isActive}
            onClick={() => onChange(option.value)}
            className={cn(
              "relative flex shrink-0 items-center gap-1.5 rounded-[calc(var(--radius-control)-3px)] px-3 py-1 text-[13px] font-medium transition-colors duration-100 ease-out motion-safe:active:scale-[0.97] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/60",
              isActive
                ? "text-(--color-foreground)"
                : "text-(--color-muted-foreground) hover:text-(--color-foreground)",
            )}
          >
            {option.label}
            {option.count !== undefined && (
              <span
                className={cn(
                  "rounded-full px-1.5 text-[10px] tabular-nums",
                  isActive ? "bg-(--color-accent-muted) text-(--color-accent)" : "bg-(--color-surface-hover) text-(--color-faint-foreground)",
                )}
              >
                {option.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}