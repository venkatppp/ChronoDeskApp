// TrendIndicator Component
//
// Displays a metric with trend comparison and percentage change.

import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import { Card } from "@/components/ui/Card";

interface TrendIndicatorProps {
  label: string;
  current: number;
  previous: number;
  format?: (value: number) => string;
}

export function TrendIndicator({ label, current, previous, format }: TrendIndicatorProps) {
  const formatValue = format || ((val: number) => val.toString());
  const changePercent = previous > 0 ? ((current - previous) / previous) * 100 : 0;
  const isImproving = changePercent > 0;
  const isFlat = Math.abs(changePercent) < 0.1;

  return (
    <Card className="flex items-center gap-3 p-4">
      <div className="min-w-0 flex-1">
        <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
          {label}
        </p>
        <p className="text-lg font-bold text-(--color-foreground)">{formatValue(current)}</p>
        <div className="mt-1 flex items-center gap-1 text-xs">
          {isFlat ? (
            <>
              <Minus className="h-3 w-3 text-(--color-muted-foreground)" strokeWidth={1.75} />
              <span className="text-(--color-muted-foreground)">No change</span>
            </>
          ) : isImproving ? (
            <>
              <TrendingUp className="h-3 w-3 text-(--color-success)" strokeWidth={1.75} />
              <span className="text-(--color-success)">+{changePercent.toFixed(1)}%</span>
            </>
          ) : (
            <>
              <TrendingDown className="h-3 w-3 text-(--color-danger)" strokeWidth={1.75} />
              <span className="text-(--color-danger)">{changePercent.toFixed(1)}%</span>
            </>
          )}
        </div>
      </div>
    </Card>
  );
}
