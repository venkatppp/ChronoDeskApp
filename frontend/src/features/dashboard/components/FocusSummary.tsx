// FocusSummary Component
//
// Displays focus session statistics.

import { Target } from "lucide-react";
import { Card } from "@/components/ui/Card";

interface FocusSummaryProps {
  sessionCount: number;
  longestSession?: number;
  averageSession?: number;
}

export function FocusSummary({
  sessionCount,
  longestSession,
  averageSession,
}: FocusSummaryProps) {
  const formatDuration = (seconds: number): string => {
    const minutes = Math.floor(seconds / 60);
    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      const mins = minutes % 60;
      return `${hours}h ${mins}m`;
    }
    return `${minutes}m`;
  };

  return (
    <Card className="p-4">
      <div className="mb-3 flex items-center gap-2">
        <Target className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
        <h3 className="text-sm font-medium text-(--color-foreground)">Focus Sessions</h3>
      </div>

      <div className="space-y-2">
        <div className="flex items-baseline justify-between">
          <span className="text-xs text-(--color-muted-foreground)">Total sessions</span>
          <span className="text-lg font-bold text-(--color-foreground)">{sessionCount}</span>
        </div>

        {longestSession !== undefined && longestSession > 0 && (
          <div className="flex items-baseline justify-between">
            <span className="text-xs text-(--color-muted-foreground)">Longest</span>
            <span className="text-sm font-medium text-(--color-foreground)">
              {formatDuration(longestSession)}
            </span>
          </div>
        )}

        {averageSession !== undefined && averageSession > 0 && (
          <div className="flex items-baseline justify-between">
            <span className="text-xs text-(--color-muted-foreground)">Average</span>
            <span className="text-sm font-medium text-(--color-foreground)">
              {formatDuration(averageSession)}
            </span>
          </div>
        )}
      </div>
    </Card>
  );
}
