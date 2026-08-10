// MemoryAgingCard - RC-6 M3: memory aging visualization (fresh / aging /
// archived buckets with the average freshness of the store).

import { Hourglass, Timer } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { MemoryAgingSummary } from "@/types/memory";

interface MemoryAgingCardProps {
  summary: MemoryAgingSummary | null;
}

export function MemoryAgingCard({ summary }: MemoryAgingCardProps) {
  if (!summary) return null;

  const buckets = [
    { label: "Fresh", count: summary.fresh_records, tone: "bg-(--color-emerald)" },
    { label: "Aging", count: summary.aging_records, tone: "bg-(--color-warning)" },
    { label: "Archived", count: summary.archived_records, tone: "bg-(--color-muted-foreground)" },
  ];
  const total = Math.max(summary.total_records, 1);

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <Hourglass className="h-4 w-4 text-(--color-muted-foreground)" />
        <h2 className="text-sm font-medium text-(--color-foreground)">Memory aging</h2>
      </div>

      <div className="mt-3 flex h-3 overflow-hidden rounded-full bg-(--color-surface)">
        {buckets.map((bucket) => (
          <div
            key={bucket.label}
            className={bucket.tone}
            style={{ width: `${(bucket.count / total) * 100}%` }}
            title={`${bucket.label}: ${bucket.count}`}
          />
        ))}
      </div>

      <div className="mt-3 grid grid-cols-3 gap-2 text-xs">
        {buckets.map((bucket) => (
          <div key={bucket.label}>
            <p className="text-(--color-muted-foreground)">{bucket.label}</p>
            <p className="mt-0.5 font-medium text-(--color-foreground)">{bucket.count}</p>
          </div>
        ))}
      </div>

      <p className="mt-3 flex items-center gap-1.5 text-[11px] text-(--color-faint-foreground)">
        <Timer className="h-3 w-3" />
        Memories decay over 30 days and archive after 180 — aged knowledge ranks lower
      </p>
    </Card>
  );
}
