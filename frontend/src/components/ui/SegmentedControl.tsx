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
 * macOS-style segmented control — translucent well, raised selection,
 * quiet text. Shared by every filter/tab group in the app.
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
        "glass-control inline-flex items-center gap-0.5 rounded-[var(--radius-control)] p-0.5",
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
              "flex shrink-0 items-center gap-1.5 rounded-[calc(var(--radius-control)-3px)] px-3 py-1 text-[13px] font-medium transition-all duration-150 ease-[var(--ease-premium)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/60",
              isActive
                ? "bg-(--color-surface-hover) text-(--color-foreground) shadow-[inset_0_1px_0_rgba(255,255,255,0.1),0_1px_2px_rgba(0,0,0,0.35)]"
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
