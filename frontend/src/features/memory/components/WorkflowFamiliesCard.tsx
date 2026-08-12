// WorkflowFamiliesCard - RC-6 M3: workflow clustering — reusable
// workflow families grouped by shared tools, with confidence bars.

import { GitBranch } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { WorkflowFamily } from "@/types/memory";

interface WorkflowFamiliesCardProps {
  families: WorkflowFamily[];
}

export function WorkflowFamiliesCard({ families }: WorkflowFamiliesCardProps) {
  if (families.length === 0) return null;

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <GitBranch className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
        <h2 className="text-sm font-medium text-(--color-foreground)">Workflow families</h2>
      </div>

      <div className="mt-3 space-y-1">
        {families.slice(0, 6).map((family) => (
          <div key={family.family_id} className="rounded-[var(--radius-control)] px-1 py-2">
            <div className="flex items-center justify-between gap-2">
              <p className="truncate text-sm font-medium text-(--color-foreground)">
                {family.name}
              </p>
              <span className="shrink-0 text-[11px] text-(--color-faint-foreground)">
                {family.member_count} workflow(s) · {family.total_successes} ok ·{" "}
                {family.total_failures} failed
              </span>
            </div>
            {family.shared_tools.length > 0 && (
              <p className="mt-1 truncate font-mono text-[11px] text-(--color-muted-foreground)">
                {family.shared_tools.join(" → ")}
              </p>
            )}
            {family.avg_confidence > 0 && (
              <div className="mt-2 flex items-center gap-2">
                <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-(--color-surface)">
                  <div
                    className="h-full rounded-full bg-(--color-accent-soft)"
                    style={{ width: `${Math.round(family.avg_confidence * 100)}%` }}
                  />
                </div>
                <span className="text-[11px] text-(--color-faint-foreground)">
                  confidence {Math.round(family.avg_confidence * 100)}%
                </span>
              </div>
            )}
          </div>
        ))}
      </div>
    </Card>
  );
}
