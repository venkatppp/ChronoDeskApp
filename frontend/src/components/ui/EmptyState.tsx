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

/** A useful empty state: quiet symbol, explanation, and actions. */
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
        "glass-panel flex flex-col items-center gap-4 rounded-[var(--radius-card)] px-8 py-14 text-center",
        className,
      )}
    >
      <div className="flex h-12 w-12 items-center justify-center rounded-xl border border-(--color-border-subtle) bg-(--color-surface-raised) text-(--color-muted-foreground)">
        {icon}
      </div>
      <div className="flex max-w-md flex-col items-center gap-1.5">
        <h3 className="font-(family-name:--font-display) text-base font-semibold text-(--color-foreground)">
          {title}
        </h3>
        {description && (
          <p className="text-sm leading-relaxed text-(--color-muted-foreground)">{description}</p>
        )}
      </div>
      {(primaryAction || secondaryAction) && (
        <div className="flex flex-wrap items-center justify-center gap-2.5">{primaryAction}</div>
      )}
    </div>
  );
}
