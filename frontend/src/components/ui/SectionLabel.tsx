import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface SectionLabelProps {
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
  right?: ReactNode;
}

/** Consistent small uppercase section label used to frame every page region. */
export function SectionLabel({ icon, children, className, right }: SectionLabelProps) {
  return (
    <div className={cn("flex items-center justify-between gap-3", className)}>
      <h2 className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.12em] text-(--color-faint-foreground)">
        {icon && <span className="text-(--color-muted-foreground)">{icon}</span>}
        <span className="truncate">{children}</span>
      </h2>
      {right}
    </div>
  );
}
