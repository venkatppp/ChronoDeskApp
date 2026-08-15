import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface PageHeaderProps {
  eyebrow?: string;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  className?: string;
}

/** Large page title block — quiet, Apple-style hierarchy. */
export function PageHeader({ eyebrow, title, description, actions, className }: PageHeaderProps) {
  return (
    <div className={cn("flex flex-col gap-5 md:flex-row md:items-end md:justify-between", className)}>
      <div className="min-w-0 shrink-0">
        {eyebrow && (
          <p className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
            {eyebrow}
          </p>
        )}
        <h1 className="font-(family-name:--font-display) text-[1.75rem] font-semibold tracking-[-0.02em] text-(--color-foreground)">
          {title}
        </h1>
        {description && (
          <p className="mt-1.5 text-sm leading-relaxed text-(--color-muted-foreground)">{description}</p>
        )}
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2.5">{actions}</div>}
    </div>
  );
}
