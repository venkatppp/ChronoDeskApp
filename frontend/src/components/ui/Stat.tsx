import { type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface StatProps {
  label: ReactNode;
  value: ReactNode;
  icon?: ReactNode;
  hint?: ReactNode;
  accent?: "accent" | "success" | "warning" | "danger" | "neutral" | "violet" | "cyan" | "orange";
  className?: string;
}

const ACCENTS: Record<NonNullable<StatProps["accent"]>, string> = {
  accent: "text-(--color-accent)",
  cyan: "text-(--color-cyan)",
  violet: "text-(--color-violet)",
  orange: "text-(--color-orange)",
  success: "text-(--color-success)",
  warning: "text-(--color-warning)",
  danger: "text-(--color-danger)",
  neutral: "text-(--color-muted-foreground)",
};

/** Quiet headline metric — floats over the background: a small-caps label,
 *  a big number, and at most a tinted icon. No box, no glow. */
export function Stat({ label, value, icon, hint, accent = "neutral", className }: StatProps) {
  const tone = ACCENTS[accent];
  return (
    <div className={cn("flex items-start justify-between gap-3", className)}>
      <div className="min-w-0">
        <p className="truncate text-[11px] font-semibold uppercase tracking-wider text-(--color-faint-foreground)">
          {label}
        </p>
        <div className="mt-1 font-(family-name:--font-display) text-2xl font-semibold tabular-nums tracking-tight text-(--color-foreground)">
          {value}
        </div>
        {hint && <div className="mt-1 text-xs text-(--color-faint-foreground)">{hint}</div>}
      </div>
      {icon && <span className={cn("mt-1 shrink-0", tone)}>{icon}</span>}
    </div>
  );
}
