import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Brain,
  TrendingUp,
  Target,
  Activity,
  CheckCircle,
  XCircle,
  Clock,
  BarChart3,
  RefreshCw,
} from 'lucide-react';
import { Button } from "@/components/ui/Button";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/Card";
import { Stat } from "@/components/ui/Stat";

interface LearningStats {
  total_feedback_count: number;
  accepted_count: number;
  rejected_count: number;
  acceptance_rate: number;
  total_preferences: number;
  total_patterns: number;
  avg_confidence_adjustment: number;
  last_learning_update: string;
}

interface UserPreference {
  id: string;
  preference_type: string;
  key: string;
  value: string;
  confidence: number;
  evidence_count: number;
  last_updated: string;
}

interface BehavioralPattern {
  id: string;
  pattern_type: string;
  description: string;
  conditions: Record<string, unknown>;
  frequency: number;
  confidence: number;
  occurrences: number;
  first_seen: string;
  last_seen: string;
}

interface ConfidenceTrend {
  date: string;
  avg_confidence: number;
  adjustment_count: number;
}

interface CategoryAccuracy {
  category: string;
  accuracy: number;
  total: number;
  accepted: number;
}

interface RecommendationAccuracy {
  category_accuracy: CategoryAccuracy[];
  overall_accuracy: number;
  total_recommendations: number;
}

interface LearningInsights {
  stats: LearningStats;
  top_preferences: UserPreference[];
  recent_patterns: BehavioralPattern[];
  confidence_trends: ConfidenceTrend[];
  recommendation_accuracy: RecommendationAccuracy;
}

export default function LearningPage() {
  const [insights, setInsights] = useState<LearningInsights | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadInsights();
  }, []);

  async function loadInsights() {
    try {
      setLoading(true);
      setError(null);
      const data = await invoke<LearningInsights>('get_learning_insights');
      setInsights(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <PageContainer>
        <div className="space-y-4">
          <div className="h-24 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
          <div className="h-64 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
          <div className="h-40 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
        </div>
      </PageContainer>
    );
  }

  if (error) {
    return (
      <PageContainer>
        <div className="flex flex-col items-center gap-4 rounded-[var(--radius-card)] border border-(--color-danger)/20 bg-(--color-danger)/5 px-6 py-20 text-center">
          <XCircle className="h-10 w-10 text-(--color-danger)" strokeWidth={1.5} />
          <p className="max-w-md text-sm text-(--color-muted-foreground)">{error}</p>
          <Button variant="outline" size="sm" onClick={loadInsights}>
            Retry
          </Button>
        </div>
      </PageContainer>
    );
  }

  if (!insights) {
    return (
      <PageContainer>
        <div className="flex flex-col items-center gap-4 rounded-[var(--radius-card)] border border-(--color-border-subtle) bg-(--color-surface) px-6 py-20 text-center">
          <Brain className="h-10 w-10 text-(--color-faint-foreground)" strokeWidth={1.5} />
          <p className="text-sm text-(--color-muted-foreground)">No learning data available yet.</p>
        </div>
      </PageContainer>
    );
  }

  const { stats, top_preferences, recent_patterns, confidence_trends, recommendation_accuracy } = insights;

  return (
    <PageContainer>
      <PageHeader
        eyebrow="Intelligence"
        title="Learning Insights"
        description="Adaptive learning from your behavior and feedback — preferences, patterns, and recommendation accuracy."
        actions={
          <Button variant="outline" onClick={loadInsights}>
            <RefreshCw className="h-4 w-4" strokeWidth={1.75} />
            Refresh
          </Button>
        }
      />

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Stat
          label="Total Feedback"
          value={stats.total_feedback_count}
          icon={<Activity className="h-4 w-4" strokeWidth={1.75} />}
          accent="accent"
        />
        <Stat
          label="Acceptance Rate"
          value={`${(stats.acceptance_rate * 100).toFixed(1)}%`}
          icon={<CheckCircle className="h-4 w-4" strokeWidth={1.75} />}
          accent="success"
        />
        <Stat
          label="Preferences"
          value={stats.total_preferences}
          icon={<Target className="h-4 w-4" strokeWidth={1.75} />}
          accent="warning"
        />
        <Stat
          label="Patterns"
          value={stats.total_patterns}
          icon={<BarChart3 className="h-4 w-4" strokeWidth={1.75} />}
          accent="neutral"
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-(--color-success)/12 text-(--color-success)">
              <TrendingUp className="h-4 w-4" strokeWidth={1.75} />
            </span>
            Recommendation Accuracy
          </CardTitle>
          <CardDescription>
            How often the learning engine's suggestions match your choices, per category.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          {recommendation_accuracy.total_recommendations === 0 ? (
            <div className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-raised) px-4 py-6 text-center">
              <p className="text-sm font-medium text-(--color-foreground)">Insufficient data</p>
              <p className="mx-auto mt-1 max-w-sm text-xs leading-relaxed text-(--color-muted-foreground)">
                Accuracy is calculated from accepted/rejected feedback on your recommendations.
                Once you accept or reject suggestions from the dashboard or copilot, ChronoDesk
                can measure how often its suggestions match your choices.
              </p>
            </div>
          ) : (
            <>
              <div>
                <div className="mb-2 flex items-baseline justify-between">
                  <span className="text-sm text-(--color-muted-foreground)">
                    Overall Accuracy
                    <span className="ml-1.5 text-xs text-(--color-faint-foreground)">
                      ({recommendation_accuracy.total_recommendations} recommendations)
                    </span>
                  </span>
                  <span className="font-(family-name:--font-display) text-xl font-bold tabular-nums text-(--color-foreground)">
                    {(recommendation_accuracy.overall_accuracy * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-(--color-surface-hover)">
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-(--color-success)/70 to-(--color-success) animate-(--animate-grow-bar)"
                    style={{ width: `${recommendation_accuracy.overall_accuracy * 100}%` }}
                  />
                </div>
              </div>
              <div className="space-y-3.5">
                {recommendation_accuracy.category_accuracy.map((cat) => (
                  <div key={cat.category}>
                    <div className="mb-1.5 flex items-center justify-between">
                      <span className="text-sm font-medium text-(--color-foreground)">{cat.category}</span>
                      <span className="text-xs tabular-nums text-(--color-muted-foreground)">
                        {cat.accepted}/{cat.total} ({(cat.accuracy * 100).toFixed(0)}%)
                      </span>
                    </div>
                    <div className="h-1.5 overflow-hidden rounded-full bg-(--color-surface-hover)">
                      <div
                        className="h-full rounded-full bg-(--color-accent) transition-all duration-700 ease-[var(--ease-premium)]"
                        style={{ width: `${cat.accuracy * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <div className="grid items-start gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-(--color-warning)/12 text-(--color-warning)">
                <Target className="h-4 w-4" strokeWidth={1.75} />
              </span>
              Top Preferences
            </CardTitle>
            <CardDescription>Things the engine has learned you prefer.</CardDescription>
          </CardHeader>
          <CardContent>
            {top_preferences.length === 0 ? (
              <p className="text-sm text-(--color-muted-foreground)">No preferences learned yet.</p>
            ) : (
              <div className="space-y-2">
                {top_preferences.map((pref) => (
                  <div
                    key={pref.id}
                    className="flex items-center justify-between rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-raised) px-3.5 py-3 transition-colors hover:border-(--color-border)"
                  >
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium text-(--color-foreground)">{pref.key}</p>
                      <p className="text-xs text-(--color-muted-foreground)">
                        {pref.preference_type} · {pref.evidence_count} occurrences
                      </p>
                    </div>
                    <div className="ml-3 text-right">
                      <p className="font-(family-name:--font-display) text-base font-semibold tabular-nums text-(--color-foreground)">
                        {(pref.confidence * 100).toFixed(0)}%
                      </p>
                      <p className="text-[10px] uppercase tracking-wider text-(--color-faint-foreground)">confidence</p>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-(--color-accent)/12 text-(--color-accent)">
                <Activity className="h-4 w-4" strokeWidth={1.75} />
              </span>
              Behavioral Patterns
            </CardTitle>
            <CardDescription>Recurring sequences the engine has detected.</CardDescription>
          </CardHeader>
          <CardContent>
            {recent_patterns.length === 0 ? (
              <p className="text-sm text-(--color-muted-foreground)">No patterns detected yet.</p>
            ) : (
              <div className="space-y-2">
                {recent_patterns.map((pattern) => (
                  <div
                    key={pattern.id}
                    className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-raised) px-3.5 py-3"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-(--color-foreground)">{pattern.description}</p>
                        <p className="mt-0.5 text-xs text-(--color-muted-foreground)">
                          {pattern.pattern_type} · {pattern.occurrences} occurrences
                        </p>
                      </div>
                      <span className="shrink-0 font-(family-name:--font-display) text-sm font-semibold tabular-nums text-(--color-accent)">
                        {(pattern.confidence * 100).toFixed(0)}%
                      </span>
                    </div>
                    <div className="mt-2 flex items-center gap-2 text-xs text-(--color-faint-foreground)">
                      <Clock className="h-3 w-3" strokeWidth={1.75} />
                      <span>Frequency: {pattern.frequency.toFixed(2)}</span>
                      <span>·</span>
                      <span>Last seen: {new Date(pattern.last_seen).toLocaleDateString()}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid items-start gap-6 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-(--color-accent)/12 text-(--color-accent)">
                <TrendingUp className="h-4 w-4" strokeWidth={1.75} />
              </span>
              Confidence Trends (30 days)
            </CardTitle>
            <CardDescription>Average recommendation confidence over time.</CardDescription>
          </CardHeader>
          <CardContent>
          {confidence_trends.length === 0 ? (
            <p className="text-sm text-(--color-muted-foreground)">No trend data available yet.</p>
          ) : (
            <div className="space-y-2.5">
              {confidence_trends.map((trend) => (
                <div key={trend.date} className="flex items-center gap-3">
                  <span className="w-24 shrink-0 text-xs tabular-nums text-(--color-muted-foreground)">
                    {new Date(trend.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
                  </span>
                  <div className="h-2 flex-1 overflow-hidden rounded-full bg-(--color-surface-hover)">
                    <div
                      className="h-full rounded-full bg-gradient-to-r from-(--color-accent)/60 to-(--color-accent) transition-all duration-700 ease-[var(--ease-premium)]"
                      style={{ width: `${trend.avg_confidence * 100}%` }}
                    />
                  </div>
                  <span className="w-14 shrink-0 text-right text-sm font-medium tabular-nums text-(--color-foreground)">
                    {(trend.avg_confidence * 100).toFixed(0)}%
                  </span>
                  <span className="w-14 shrink-0 text-right text-xs tabular-nums text-(--color-faint-foreground)">
                    {trend.adjustment_count} adj.
                  </span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <span className="flex h-7 w-7 items-center justify-center rounded-lg bg-(--color-warning)/12 text-(--color-warning)">
              <Activity className="h-4 w-4" strokeWidth={1.75} />
            </span>
            Feedback Summary
          </CardTitle>
          <CardDescription>How the engine's suggestions were received.</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-4">
            <div className="flex items-center gap-3.5 rounded-[var(--radius-control)] border border-(--color-success)/20 bg-(--color-success)/5 px-4 py-3.5">
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-(--color-success)/12 text-(--color-success)">
                <CheckCircle className="h-5 w-5" strokeWidth={1.75} />
              </span>
              <div>
                <p className="font-(family-name:--font-display) text-2xl font-bold tabular-nums text-(--color-foreground)">
                  {stats.accepted_count}
                </p>
                <p className="text-xs text-(--color-muted-foreground)">Accepted</p>
              </div>
            </div>
            <div className="flex items-center gap-3.5 rounded-[var(--radius-control)] border border-(--color-danger)/20 bg-(--color-danger)/5 px-4 py-3.5">
              <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-(--color-danger)/12 text-(--color-danger)">
                <XCircle className="h-5 w-5" strokeWidth={1.75} />
              </span>
              <div>
                <p className="font-(family-name:--font-display) text-2xl font-bold tabular-nums text-(--color-foreground)">
                  {stats.rejected_count}
                </p>
                <p className="text-xs text-(--color-muted-foreground)">Rejected</p>
              </div>
            </div>
          </div>
          <div className="mt-4 flex flex-wrap items-center justify-between gap-x-6 gap-y-2 border-t border-(--color-border-subtle) pt-4">
            <div className="flex min-w-0 flex-col">
              <span className="text-xs text-(--color-muted-foreground)">Avg confidence adjustment</span>
              <span className="font-(family-name:--font-display) text-sm font-semibold tabular-nums text-(--color-foreground)">
                {stats.avg_confidence_adjustment > 0
                  ? `${(stats.avg_confidence_adjustment * 100).toFixed(1)}%`
                  : "No adjustments yet"}
              </span>
            </div>
            <div className="flex min-w-0 flex-col">
              <span className="text-xs text-(--color-muted-foreground)">Last update</span>
              <span className="text-sm tabular-nums text-(--color-foreground)">
                {stats.total_feedback_count + stats.total_preferences + stats.total_patterns > 0
                  ? new Date(stats.last_learning_update).toLocaleDateString()
                  : "No learning activity yet"}
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
      </div>
    </PageContainer>
  );
}
