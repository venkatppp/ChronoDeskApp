// TrendIndicator Component
//
// Displays a trend indicator with percentage change and description.

import { TrendUp, TrendDown, Minus } from "lucide-react";
import type { TrendIndicator as TrendIndicatorType } from "@/types/analytics";

interface TrendIndicatorProps {
  trend: TrendIndicatorType;
  size?: "sm" | "md" | "lg";
}

export function TrendIndicator({ trend, size = "md" }: TrendIndicatorProps) {
  const isImproving = trend.changePercent > 0;
  const isFlat = Math.abs(trend.changePercent) < 0.1;

  const iconSizes = {
    sm: "h-3 w-3",
    md: "h-4 w-4",
    lg: "h-5 w-5",
  };

  const textSizes = {
    sm: "text-xs",
    md: "text-sm",
    lg: "text-base",
  };

  const iconSize = iconSizes[size];
  const textSize = textSizes[size];

  if (isFlat) {
    return (
      <div className={`flex items-center gap-1 text-(--color-muted-foreground) ${textSize}`}>
        <Minus className={iconSize} strokeWidth={1.75} />
        <span>No change</span>
      </div>
    );
  }

  return (
    <div
      className={`flex items-center gap-1 ${textSize} ${
        isImproving ? "text-(--color-success)" : "text-(--color-danger)"
      }`}
    >
      {isImproving ? (
        <TrendUp className={iconSize} strokeWidth={1.75} />
      ) : (
        <TrendDown className={iconSize} strokeWidth={1.75} />
      )}
      <span>
        {isImproving ? "+" : ""}
        {trend.changePercent.toFixed(1)}%
      </span>
    </div>
  );
}
