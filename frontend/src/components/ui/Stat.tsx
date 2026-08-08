import { type ReactNode } from "react";
import { Card } from "@/components/ui/Card";
import { cn } from "@/utils/cn";

interface StatProps {
  label: ReactNode;
  value: ReactNode;
  icon?: ReactNode;
  hint?: ReactNode;
  accent?: "accent" | "success" | "warning" | "danger" | "neutral";
  className?: string;
}

const ACCENTS: Record<NonNullable<StatProps["accent"]>, { icon: string; glow: string }> = {
  accent: { icon: "text-(--color-accent) bg-(--color-accent-muted)", glow: "bg-(--color-accent)/20" },
  success: { icon: "text-(--color-success) bg-(--color-success)/12", glow: "bg-(--color-success)/20" },
  warning: { icon: "text-(--color-warning) bg-(--color-warning)/12", glow: "bg-(--color-warning)/20" },
  danger: { icon: "text-(--color-danger) bg-(--color-danger)/12", glow: "bg-(--color-danger)/20" },
  neutral: { icon: "text-(--color-muted-foreground) bg-(--color-surface-hover)", glow: "bg-(--color-faint-foreground)/20" },
};

/** Consistent headline number card used across dashboards. */
export function Stat({ label, value, icon, hint, accent = "neutral", className }: StatProps) {
  const tone = ACCENTS[accent];
  return (
    <Card className={cn("group relative overflow-hidden p-5", className)}>
      <span
        className={cn(
          "pointer-events-none absolute -right-6 -top-6 h-24 w-24 rounded-full blur-2xl transition-opacity duration-300 group-hover:opacity-100",
          tone.glow,
        )}
        style={{ opacity: 0.55 }}
        aria-hidden="true"
      />
      <div className="relative flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-xs font-medium text-(--color-muted-foreground)">{label}</p>
          <div className="mt-2 font-(family-name:--font-display) text-2xl font-bold tabular-nums tracking-tight text-(--color-foreground)">
            {value}
          </div>
          {hint && <div className="mt-1.5 text-xs text-(--color-faint-foreground)">{hint}</div>}
        </div>
        {icon && (
          <span className={cn("flex h-9 w-9 shrink-0 items-center justify-center rounded-xl", tone.icon)}>
            {icon}
          </span>
        )}
      </div>
    </Card>
  );
}