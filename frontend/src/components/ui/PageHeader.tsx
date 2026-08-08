import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface PageHeaderProps {
  eyebrow?: string;
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  className?: string;
}

/** Large page title block with an optional eyebrow, description, and actions. */
export function PageHeader({ eyebrow, title, description, actions, className }: PageHeaderProps) {
  return (
    <div className={cn("flex flex-col gap-6 md:flex-row md:items-end md:justify-between", className)}>
      <div className="max-w-2xl">
        {eyebrow && (
          <p className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.16em] text-(--color-accent)">
            {eyebrow}
          </p>
        )}
        <h1 className="font-(family-name:--font-display) text-3xl font-bold tracking-tight text-(--color-foreground) md:text-[2rem] md:leading-tight">
          {title}
        </h1>
        {description && (
          <p className="mt-2 text-[15px] leading-relaxed text-(--color-muted-foreground)">{description}</p>
        )}
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2.5">{actions}</div>}
    </div>
  );
}