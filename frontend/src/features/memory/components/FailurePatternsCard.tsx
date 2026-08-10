// FailurePatternsCard - RC-6 M3: detected failure patterns (repeated
// failures, unstable workflows, low-confidence plans).

import { ShieldAlert } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { FailurePattern, FailurePatternType } from "@/types/memory";

interface FailurePatternsCardProps {
  patterns: FailurePattern[];
}

const PATTERN_LABELS: Record<FailurePatternType, string> = {
  repeated_failure: "Repeated failure",
  unstable_workflow: "Unstable workflow",
  low_confidence_plan: "Low-confidence plan",
};

const PATTERN_TONES: Record<FailurePatternType, string> = {
  repeated_failure: "border-(--color-danger)/30 bg-(--color-danger)/12 text-(--color-danger)",
  unstable_workflow: "border-(--color-warning)/30 bg-(--color-warning)/12 text-(--color-warning)",
  low_confidence_plan: "border-(--color-orange)/30 bg-(--color-orange)/12 text-(--color-orange)",
};

export function FailurePatternsCard({ patterns }: FailurePatternsCardProps) {
  if (patterns.length === 0) return null;

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <ShieldAlert className="h-4 w-4 text-(--color-muted-foreground)" />
        <h2 className="text-sm font-medium text-(--color-foreground)">Failure patterns</h2>
      </div>

      <div className="mt-3 space-y-1">
        {patterns.map((pattern) => (
          <div
            key={`${pattern.pattern_type}-${pattern.goal_fingerprint}`}
            className="rounded-[var(--radius-control)] px-1 py-2"
          >
            <div className="flex items-center justify-between gap-2">
              <p className="truncate text-sm font-medium text-(--color-foreground)">
                {pattern.goal}
              </p>
              <span
                className={`shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-medium ${PATTERN_TONES[pattern.pattern_type]}`}
              >
                {PATTERN_LABELS[pattern.pattern_type]}
              </span>
            </div>
            <p className="mt-1 text-xs text-(--color-muted-foreground)">{pattern.description}</p>
            <p className="mt-1 text-[11px] text-(--color-faint-foreground)">
              severity {pattern.severity.toFixed(2)} · {pattern.occurrences} run(s)
              {pattern.avg_plan_confidence !== null &&
                ` · avg plan confidence ${Math.round(pattern.avg_plan_confidence * 100)}%`}
            </p>
          </div>
        ))}
      </div>
    </Card>
  );
}
