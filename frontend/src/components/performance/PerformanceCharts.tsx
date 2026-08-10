import { cn } from "@/utils/cn";

/** One row of a bar list. */
export interface BarDatum {
  label: string;
  value: number;
  sublabel?: string;
}

/**
 * Dependency-free horizontal bar list. Bars are sized relative to the
 * largest value so durations, sizes, and counts stay directly comparable
 * without pulling in a charting library.
 */
export function BarList({
  items,
  valueFormatter = (v) => String(v),
  color = "var(--color-accent-soft)",
  max,
  onSelect,
  className,
}: {
  items: BarDatum[];
  valueFormatter?: (value: number) => string;
  color?: string;
  max?: number;
  onSelect?: (index: number) => void;
  className?: string;
}) {
  const ceiling = max ?? Math.max(...items.map((item) => item.value), 1);
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      {items.map((item, index) => {
        const bar = (
          <>
            <span className="w-36 shrink-0 truncate text-xs text-(--color-muted-foreground)">
              {item.label}
            </span>
            <div className="relative h-5 flex-1 overflow-hidden rounded-[var(--radius-card)] bg-(--color-border-subtle)">
              <div
                className="flex h-full items-center rounded-[var(--radius-card)] px-1.5 transition-all duration-[350ms] ease-[cubic-bezier(0.32,0.08,0.24,1)]"
                style={{
                  width: `${Math.max((item.value / ceiling) * 100, 2)}%`,
                  backgroundColor: color,
                }}
                title={`${item.label}: ${valueFormatter(item.value)}`}
              >
                {item.sublabel && (
                  <span className="truncate text-[10px] text-(--color-accent-foreground) opacity-80">
                    {item.sublabel}
                  </span>
                )}
              </div>
            </div>
            <span className="w-16 shrink-0 text-right font-(family-name:--font-display) text-xs tabular-nums">
              {valueFormatter(item.value)}
            </span>
          </>
        );
        return onSelect ? (
          <button
            key={item.label}
            type="button"
            onClick={() => onSelect(index)}
            className="flex w-full items-center gap-3 rounded-[var(--radius-control)] text-left transition-colors hover:bg-(--color-surface-hover)"
          >
            {bar}
          </button>
        ) : (
          <div key={item.label} className="flex items-center gap-3">
            {bar}
          </div>
        );
      })}
    </div>
  );
}

/** Small label/value stat tile used by the diagnostics grid. */
export function StatCard({
  label,
  value,
  sublabel,
  tone,
}: {
  label: string;
  value: string;
  sublabel?: string;
  tone?: "success" | "warning" | "danger";
}) {
  const toneClass =
    tone === "success"
      ? "text-(--color-success)"
      : tone === "warning"
        ? "text-(--color-warning)"
        : tone === "danger"
          ? "text-(--color-danger)"
          : "text-(--color-accent)";
  return (
    <div className="rounded-[var(--radius-card)] border border-(--color-border-subtle) bg-(--color-surface) p-4">
      <p className="text-[11px] uppercase tracking-wide text-(--color-faint-foreground)">{label}</p>
      <p className={cn("mt-1 font-(family-name:--font-display) text-2xl font-semibold tabular-nums", toneClass)}>
        {value}
      </p>
      {sublabel && <p className="mt-0.5 text-xs text-(--color-muted-foreground)">{sublabel}</p>}
    </div>
  );
}

/** Thin progress bar used to render percentage gauges. */
export function ProgressBar({ percent, tone }: { percent: number; tone?: "success" | "warning" | "danger" }) {
  const clamped = Math.max(0, Math.min(100, percent));
  const color =
    tone === "danger"
      ? "var(--color-danger)"
      : tone === "warning"
        ? "var(--color-warning)"
        : "var(--color-success)";
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-(--color-border-subtle)">
      <div
        className="h-full rounded-full transition-[width] duration-[500ms]"
        style={{ width: `${clamped}%`, backgroundColor: color }}
      />
    </div>
  );
}