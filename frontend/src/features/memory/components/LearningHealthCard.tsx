// LearningHealthCard - RC-6 M3: confidence averages, workflow quality,
// success trends, and memory utilization (learning health over IPC).

import { Activity, BrainCircuit, Gauge, Sparkles, TrendingUp } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { LearningHealth } from "@/types/memory";

interface LearningHealthCardProps {
  health: LearningHealth | null;
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-[11px] text-(--color-muted-foreground)">{label}</p>
      <p className="mt-1 font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
        {value}
      </p>
    </div>
  );
}

function percent(value: number) {
  return `${Math.round(value * 100)}%`;
}

export function LearningHealthCard({ health }: LearningHealthCardProps) {
  if (!health) return null;

  const { workflow_quality: quality, memory_utilization: utilization } = health;

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <BrainCircuit className="h-4 w-4 text-(--color-muted-foreground)" />
        <h2 className="text-sm font-medium text-(--color-foreground)">Learning health</h2>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-4">
        <Metric label="Avg confidence" value={percent(health.confidence_average)} />
        <Metric label="Successful confidence" value={percent(health.confidence_successful)} />
        <Metric label="Acceptance rate" value={percent(health.acceptance_rate)} />
        <Metric label="Learned score" value={percent(health.score_average)} />
      </div>

      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <div>
          <p className="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            <Gauge className="h-3.5 w-3.5" /> Workflow quality
          </p>
          <div className="mt-2 grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Workflows</span>
              <span className="font-medium text-(--color-foreground)">{quality.workflow_count}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Success rate</span>
              <span className="font-medium text-(--color-foreground)">
                {percent(quality.avg_success_rate)}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Plan confidence</span>
              <span className="font-medium text-(--color-foreground)">
                {percent(quality.avg_plan_confidence)}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Replay adoption</span>
              <span className="font-medium text-(--color-foreground)">
                {percent(quality.replay_adoption_rate)}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Avg duration</span>
              <span className="font-medium text-(--color-foreground)">
                {quality.avg_duration_seconds > 0
                  ? `${Math.round(quality.avg_duration_seconds / 60)} min`
                  : "—"}
              </span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-(--color-muted-foreground)">Replays / run</span>
              <span className="font-medium text-(--color-foreground)">
                {quality.replay_per_run.toFixed(2)}
              </span>
            </div>
          </div>
        </div>

        <div>
          <p className="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            <Sparkles className="h-3.5 w-3.5" /> Memory utilization
          </p>
          <div className="mt-2 space-y-2 text-xs">
            <div className="flex items-center justify-between text-(--color-muted-foreground)">
              <span>Active share</span>
              <span className="font-medium text-(--color-foreground)">
                {percent(utilization.utilization_ratio)}
              </span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-(--color-surface)">
              <div
                className="h-full rounded-full bg-(--color-accent-soft)"
                style={{ width: percent(utilization.utilization_ratio) }}
              />
            </div>
            <div className="flex items-center justify-between text-(--color-muted-foreground)">
              <span>Average freshness</span>
              <span className="font-medium text-(--color-foreground)">
                {percent(utilization.avg_freshness)}
              </span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-(--color-surface)">
              <div
                className="h-full rounded-full bg-(--color-emerald)/70"
                style={{ width: percent(utilization.avg_freshness) }}
              />
            </div>
            <div className="flex items-center justify-between text-(--color-muted-foreground)">
              <span>Workflows per memory</span>
              <span className="font-medium text-(--color-foreground)">
                {utilization.workflows_per_record.toFixed(2)}
              </span>
            </div>
          </div>
        </div>
      </div>

      {health.success_trends.length > 0 && (
        <div className="mt-4">
          <p className="flex items-center gap-1.5 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
            <TrendingUp className="h-3.5 w-3.5" /> Success trend (14 days)
          </p>
          <div className="mt-2 flex h-16 items-end gap-1">
            {health.success_trends.map((trend) => {
              const total = Math.max(trend.successes + trend.failures, 1);
              const successHeight = (trend.successes / total) * 100;
              const failureHeight = (trend.failures / total) * 100;
              return (
                <div key={trend.date} className="flex flex-1 flex-col justify-end" title={trend.date}>
                  <div
                    className="w-full rounded-t-sm bg-(--color-danger)/60"
                    style={{ height: `${failureHeight}%` }}
                  />
                  <div
                    className="w-full rounded-t-sm bg-(--color-emerald)/70"
                    style={{ height: `${successHeight}%` }}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      <p className="mt-3 flex items-center gap-1.5 text-[11px] text-(--color-faint-foreground)">
        <Activity className="h-3 w-3" />
        Confidence blends similarity, success history, replay history, freshness, and usage count
      </p>
    </Card>
  );
}
