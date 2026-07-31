// DailyBriefing Component
//
// Intelligent daily briefing with greeting, activity summary, insights, and suggestions.

import { Card } from "@/components/ui/Card";
import { ActivitySummary } from "./ActivitySummary";
import { Lightbulb, Sparkles } from "lucide-react";
import type { DailyBriefing as DailyBriefingType } from "@/types/analytics";

interface DailyBriefingProps {
  briefing: DailyBriefingType;
}

export function DailyBriefing({ briefing }: DailyBriefingProps) {
  return (
    <Card className="p-5">
      <div className="mb-4">
        <h2 className="font-(family-name:--font-display) text-xl font-bold text-(--color-foreground)">
          {briefing.greeting}
        </h2>
        {briefing.summary.durationSeconds > 0 && (
          <p className="mt-1 text-sm text-(--color-muted-foreground)">
            Here's what you accomplished {briefing.summary.timeRange.toLowerCase()}
          </p>
        )}
      </div>

      <ActivitySummary summary={briefing.summary} />

      {briefing.insights.length > 0 && (
        <div className="mt-4 space-y-2">
          <div className="flex items-center gap-2">
            <Lightbulb className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
            <h3 className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Insights
            </h3>
          </div>
          <ul className="space-y-1">
            {briefing.insights.map((insight, i) => (
              <li key={i} className="text-sm text-(--color-muted-foreground)">
                • {insight}
              </li>
            ))}
          </ul>
        </div>
      )}

      {briefing.suggestions.length > 0 && (
        <div className="mt-4 space-y-2">
          <div className="flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-(--color-warning)" strokeWidth={1.75} />
            <h3 className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
              Suggestions
            </h3>
          </div>
          <ul className="space-y-1">
            {briefing.suggestions.map((suggestion, i) => (
              <li key={i} className="text-sm text-(--color-muted-foreground)">
                • {suggestion}
              </li>
            ))}
          </ul>
        </div>
      )}
    </Card>
  );
}
