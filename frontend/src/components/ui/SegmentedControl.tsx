import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

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
 * Pill-style segmented control used for every filter/tab group so the
 * whole app shares one interaction vocabulary.
 */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
  ariaLabel,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="group"
      aria-label={ariaLabel}
      className={cn(
        "inline-flex items-center gap-0.5 rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface) p-1 shadow-[var(--shadow-card)]",
        className,
      )}
    >
      {options.map((option) => {
        const isActive = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={isActive}
            onClick={() => onChange(option.value)}
            className={cn(
              "flex shrink-0 items-center gap-1.5 rounded-[calc(var(--radius-control)-4px)] px-3 py-1.5 text-[13px] font-medium transition-all duration-150 ease-[var(--ease-premium)]",
              isActive
                ? "bg-(--color-surface-raised) text-(--color-foreground) shadow-[0_1px_2px_rgba(0,0,0,0.5),inset_0_1px_0_rgba(255,255,255,0.04)]"
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