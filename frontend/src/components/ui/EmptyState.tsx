import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface EmptyStateProps {
  icon: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  primaryAction?: ReactNode;
  secondaryAction?: ReactNode;
  className?: string;
}

/**
 * A useful empty state: illustration ring, explanation, and actions.
 * Replaces the old "No X" placeholder messages app-wide.
 */
export function EmptyState({
  icon,
  title,
  description,
  primaryAction,
  secondaryAction,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "relative flex flex-col items-center gap-5 overflow-hidden rounded-[var(--radius-card)] border border-(--color-border-subtle) bg-(--color-surface) px-8 py-16 text-center shadow-[var(--shadow-card)]",
        className,
      )}
    >
      <div
        className="pointer-events-none absolute inset-0 bg-dotgrid opacity-60"
        aria-hidden="true"
      />
      <div className="relative flex h-16 w-16 items-center justify-center rounded-2xl border border-(--color-border) bg-(--color-surface-raised) shadow-[var(--shadow-pop)]">
        <div className="absolute inset-0 rounded-2xl bg-(--color-accent)/10 blur-xl" aria-hidden="true" />
        <span className="relative flex h-9 w-9 items-center justify-center rounded-xl bg-(--color-accent-muted) text-(--color-accent)">
          {icon}
        </span>
      </div>
      <div className="relative flex max-w-md flex-col items-center gap-2">
        <h3 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
          {title}
        </h3>
        {description && (
          <p className="text-sm leading-relaxed text-(--color-muted-foreground)">{description}</p>
        )}
      </div>
      {(primaryAction || secondaryAction) && (
        <div className="relative flex flex-wrap items-center justify-center gap-2.5">
          {primaryAction}
          {secondaryAction}
        </div>
      )}
    </div>
  );
}