import { PlayCircle, Archive, Copy, AlertTriangle, type LucideIcon } from "lucide-react";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/Card";
import type { Recommendation, RecommendationKind } from "@/types/workspace";

const KIND_ICON: Record<RecommendationKind, LucideIcon> = {
  resume: PlayCircle,
  archive: Archive,
  duplicate: Copy,
  deadline: AlertTriangle,
};

interface RecommendationsPanelProps {
  recommendations: Recommendation[];
  isLoading: boolean;
}

export function RecommendationsPanel({ recommendations, isLoading }: RecommendationsPanelProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Suggested</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        {isLoading && (
          <div className="flex flex-col gap-2">
            {[0, 1, 2].map((i) => (
              <div key={i} className="h-8 animate-pulse rounded-[var(--radius-control)] bg-(--color-surface-hover)" />
            ))}
          </div>
        )}

        {!isLoading && recommendations.length === 0 && (
          <p className="py-2 text-sm text-(--color-faint-foreground)">Nothing needs your attention right now.</p>
        )}

        {!isLoading &&
          recommendations.map((rec) => {
            const Icon = KIND_ICON[rec.kind];
            return (
              <button
                key={rec.id}
                className="flex items-center gap-2.5 rounded-[var(--radius-control)] px-2 py-2 text-left text-sm
                           text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
              >
                <Icon className="h-4 w-4 shrink-0 text-(--color-accent)" strokeWidth={1.75} />
                <span className="truncate">{rec.message}</span>
              </button>
            );
          })}
      </CardContent>
    </Card>
  );
}
