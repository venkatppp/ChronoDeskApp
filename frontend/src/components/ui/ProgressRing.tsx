import { cn } from "@/utils/cn";

interface ProgressRingProps {
  /** 0–100 */
  value: number;
  size?: number;
  strokeWidth?: number;
  className?: string;
  label?: string;
}

/**
 * The dashboard's signature visual: a small circular "activity ring"
 * standing in for a workspace's health score, echoing the product's core
 * metaphor (a workspace as a living context, not a static folder) rather
 * than a generic horizontal progress bar.
 */
export function ProgressRing({ value, size = 44, strokeWidth = 4, className, label }: ProgressRingProps) {
  const clamped = Math.min(100, Math.max(0, value));
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (clamped / 100) * circumference;

  const tone =
    clamped >= 70 ? "var(--color-success)" : clamped >= 40 ? "var(--color-accent)" : "var(--color-warning)";

  return (
    <div className={cn("relative inline-flex items-center justify-center", className)}>
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="var(--color-border-subtle)"
          strokeWidth={strokeWidth}
        />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke={tone}
          strokeWidth={strokeWidth}
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          className="transition-[stroke-dashoffset] duration-500 ease-out"
        />
      </svg>
      <span className="absolute font-(family-name:--font-mono) text-[11px] font-medium text-(--color-foreground)">
        {label ?? `${Math.round(clamped)}`}
      </span>
    </div>
  );
}
