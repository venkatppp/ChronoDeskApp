// MemoryDashboard - what ChronoDesk has learned from previous executions
// (RC-6 M1 + M2): stats, semantic search over remembered runs, workflow
// recommendations, strategies to avoid, and the vector index status with
// a manual re-index action.

import { useCallback, useEffect, useState } from "react";
import {
  BrainCircuit,
  Check,
  Database,
  History,
  RefreshCw,
  Search,
  ShieldAlert,
  TrendingUp,
  X,
} from "lucide-react";
import { DuplicateGroupsCard } from "@/features/memory/components/DuplicateGroupsCard";
import { FailurePatternsCard } from "@/features/memory/components/FailurePatternsCard";
import { LearningHealthCard } from "@/features/memory/components/LearningHealthCard";
import { LineageExplorerCard } from "@/features/memory/components/LineageExplorerCard";
import { MemoryAgingCard } from "@/features/memory/components/MemoryAgingCard";
import { RetentionManagerCard } from "@/features/memory/components/RetentionManagerCard";
import { SnapshotManagerCard } from "@/features/memory/components/SnapshotManagerCard";
import { StorageStatsCard } from "@/features/memory/components/StorageStatsCard";
import { WorkflowFamiliesCard } from "@/features/memory/components/WorkflowFamiliesCard";
import { Button } from "@/components/ui/Button";
import { GlassInput } from "@/components/ui/GlassInput";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { Stat } from "@/components/ui/Stat";
import { memoryRepository } from "@/services/memoryRepository";
import type {
  AvoidedStrategy,
  DuplicateGroup,
  ExecutionMemoryRecord,
  FailurePattern,
  LearnedWorkflow,
  LearningHealth,
  MemoryAgingSummary,
  MemoryHit,
  MemoryKind,
  MemoryRecommendation,
  MemoryStats,
  MemoryStorageStats,
  VectorIndexStatus,
  WorkflowFamily,
} from "@/types/memory";

const KIND_LABELS: Record<MemoryKind, string> = {
  execution: "Execution",
  planner_report: "Planner report",
  autonomous_session: "Autonomous session",
};

const STATUS_LABELS: Record<string, string> = {
  success: "Success",
  failed: "Failed",
  cancelled: "Cancelled",
};

function statusPill(record: ExecutionMemoryRecord) {
  const tone =
    record.status === "success"
      ? "border-(--color-success)/30 bg-(--color-success)/12 text-(--color-success)"
      : record.status === "failed"
        ? "border-(--color-danger)/30 bg-(--color-danger)/12 text-(--color-danger)"
        : "border-(--color-warning)/30 bg-(--color-warning)/12 text-(--color-warning)";
  return (
    <span
      className={`rounded-full border px-2 py-0.5 text-[11px] font-medium ${tone}`}
    >
      {STATUS_LABELS[record.status]}
    </span>
  );
}

function StatCard({ label, value }: { label: string; value: number | string }) {
  return <Stat label={label} value={value} />;
}

function RecordCard({ hit }: { hit: MemoryHit }) {
  const record = hit.record;
  return (
    <div className="rounded-[var(--radius-control)] p-3.5 transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)">
      <div className="flex items-center justify-between gap-2">
        <p className="truncate text-sm font-medium text-(--color-foreground)">{record.goal}</p>
        {statusPill(record)}
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-(--color-faint-foreground)">
        <span className="rounded bg-(--color-surface) px-1.5 py-0.5">{KIND_LABELS[record.kind]}</span>
        <span>similarity {hit.similarity.toFixed(2)}</span>
        <span>{record.outcome.completed}/{record.outcome.steps} steps</span>
        {record.outcome.duration_seconds > 0 && (
          <span>{Math.round(record.outcome.duration_seconds / 60)} min</span>
        )}
        {record.replay_count > 0 && <span>{record.replay_count} replay(s)</span>}
      </div>
      {record.tools_used.length > 0 && (
        <p className="mt-1.5 truncate font-(family-name:--font-mono) text-[11px] text-(--color-muted-foreground)">
          {record.tools_used.join(" → ")}
        </p>
      )}
      {record.error && (
        <p className="mt-1.5 truncate text-[11px] text-(--color-danger)">{record.error}</p>
      )}
    </div>
  );
}

function RecommendationCard({
  recommendation,
  onFeedback,
}: {
  recommendation: MemoryRecommendation;
  onFeedback: (accepted: boolean) => void;
}) {
  const record = recommendation.record;
  return (
    <div className="rounded-[var(--radius-control)] p-3.5 transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)">
      <div className="flex items-center justify-between gap-2">
        <p className="truncate text-sm font-medium text-(--color-foreground)">{record.goal}</p>
        <div className="flex shrink-0 items-center gap-1.5">
          <span className="rounded-full border border-(--color-emerald)/30 bg-(--color-emerald)/12 px-2 py-0.5 text-[11px] font-medium text-(--color-emerald)">
            score {recommendation.score.toFixed(2)}
          </span>
          <span
            className="rounded-full border border-(--color-border) bg-(--color-surface) px-2 py-0.5 text-[11px] font-medium text-(--color-foreground)"
            title={recommendation.explanation
              .map((factor) => `${factor.factor}: ${factor.description}`)
              .join("\n")}
          >
            confidence {recommendation.confidence_score.toFixed(2)}
          </span>
        </div>
      </div>
      {record.plan && (
        <p className="mt-1 font-(family-name:--font-mono) text-[11px] text-(--color-muted-foreground)">
          {record.plan.tasks.map((task) => task.description).join(" → ")}
        </p>
      )}
      {recommendation.explanation.length > 0 && (
        <ul className="mt-2 space-y-1 border-t border-(--color-border) pt-2">
          {recommendation.explanation.map((factor) => (
            <li key={factor.factor} className="flex items-baseline gap-2 text-[11px]">
              <span
                className={
                  factor.impact > 0
                    ? "font-medium text-(--color-emerald)"
                    : factor.impact < 0
                      ? "font-medium text-(--color-danger)"
                      : "font-medium text-(--color-muted-foreground)"
                }
              >
                {factor.impact > 0 ? "↑" : factor.impact < 0 ? "↓" : "·"} {factor.factor}
              </span>
              <span className="text-(--color-faint-foreground)">{factor.description}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="mt-2 flex items-center gap-2">
        <button
          onClick={() => onFeedback(true)}
          title="Accept this recommendation"
          className="flex items-center gap-1 rounded-md border border-(--color-emerald)/30 bg-(--color-emerald)/12 px-2 py-1 text-[11px] font-medium text-(--color-emerald) transition-opacity hover:opacity-80"
        >
          <Check className="h-3 w-3" /> Accept
        </button>
        <button
          onClick={() => onFeedback(false)}
          title="Reject this recommendation"
          className="flex items-center gap-1 rounded-md border border-(--color-danger)/30 bg-(--color-danger)/12 px-2 py-1 text-[11px] font-medium text-(--color-danger) transition-opacity hover:opacity-80"
        >
          <X className="h-3 w-3" /> Reject
        </button>
        <span className="text-[11px] text-(--color-faint-foreground)">
          {recommendation.replay_count} replay(s)
        </span>
      </div>
    </div>
  );
}

function SectionHeading({ icon, title }: { icon: React.ReactNode; title: string }) {
  return (
    <div className="flex items-center gap-2">
      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-(--color-surface-raised) text-(--color-muted-foreground)">
        {icon}
      </span>
      <h2 className="font-(family-name:--font-display) text-sm font-semibold tracking-tight text-(--color-foreground)">
        {title}
      </h2>
    </div>
  );
}

export function MemoryDashboard() {
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [indexStatus, setIndexStatus] = useState<VectorIndexStatus | null>(null);
  const [workflows, setWorkflows] = useState<LearnedWorkflow[]>([]);
  const [recent, setRecent] = useState<MemoryHit[]>([]);
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState<MemoryHit[] | null>(null);
  const [recommendGoal, setRecommendGoal] = useState("");
  const [recommendations, setRecommendations] = useState<MemoryRecommendation[] | null>(null);
  const [avoided, setAvoided] = useState<AvoidedStrategy[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [reindexing, setReindexing] = useState(false);
  const [health, setHealth] = useState<LearningHealth | null>(null);
  const [families, setFamilies] = useState<WorkflowFamily[]>([]);
  const [aging, setAging] = useState<MemoryAgingSummary | null>(null);
  const [failurePatterns, setFailurePatterns] = useState<FailurePattern[]>([]);
  const [duplicates, setDuplicates] = useState<DuplicateGroup[]>([]);
  const [storageStats, setStorageStats] = useState<MemoryStorageStats | null>(null);

  const loadOverview = useCallback(async () => {
    try {
      const [statsData, indexData, workflowsData, recentData, healthData, familiesData, agingData, failuresData, duplicatesData, storageData] =
        await Promise.all([
          memoryRepository.stats(),
          memoryRepository.indexStatus(),
          memoryRepository.learnedWorkflows(),
          memoryRepository.search("", { limit: 6 }),
          memoryRepository.learningHealth(),
          memoryRepository.workflowFamilies(),
          memoryRepository.agingSummary(),
          memoryRepository.failurePatterns(),
          memoryRepository.duplicateGroups(),
          memoryRepository.storageStats(),
        ]);
      setStats(statsData);
      setIndexStatus(indexData);
      setWorkflows(workflowsData);
      setRecent(recentData);
      setHealth(healthData);
      setFamilies(familiesData);
      setAging(agingData);
      setFailurePatterns(failuresData);
      setDuplicates(duplicatesData);
      setStorageStats(storageData);
    } catch (err) {
      console.error("Failed to load memory overview:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadOverview();
  }, [loadOverview]);

  const runSearch = async () => {
    try {
      const hits = await memoryRepository.search(query, { limit: 10 });
      setSearched(hits);
    } catch (err) {
      console.error("Memory search failed:", err);
    }
  };

  const runRecommend = async () => {
    const goal = recommendGoal.trim();
    if (!goal) return;
    try {
      const [recs, avoid] = await Promise.all([
        memoryRepository.recommend(goal),
        memoryRepository.avoid(goal),
      ]);
      setRecommendations(recs);
      setAvoided(avoid);
    } catch (err) {
      console.error("Memory recommend failed:", err);
    }
  };

  const sendFeedback = async (memoryId: string, accepted: boolean) => {
    try {
      await memoryRepository.recommendationFeedback(memoryId, accepted);
      await runRecommend();
    } catch (err) {
      console.error("Recommendation feedback failed:", err);
    }
  };

  const runMergeDuplicates = async () => {
    await loadOverview();
  };

  const runReindex = async () => {
    if (reindexing) return;
    setReindexing(true);
    try {
      const result = await memoryRepository.reindex();
      console.info(
        `Re-indexed ${result.indexed}/${result.requested} memories (${result.failed} failed)`
      );
      await loadOverview();
    } catch (err) {
      console.error("Memory re-index failed:", err);
    } finally {
      setReindexing(false);
    }
  };

  return (
    <PageContainer>
      <PageHeader
        eyebrow="Intelligence"
        title="Memory"
        description="What ChronoDesk has learned over time — searchable history, workflow recommendations, and failed strategies to avoid."
      />

      {loading && (
        <div className="space-y-4">
          <div className="h-20 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
          <div className="h-64 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
        </div>
      )}

      <section className="grid items-start gap-8 lg:grid-cols-2">
        {/* 1. SEARCH — the primary interaction */}
        <section className="glass-panel rounded-[var(--radius-card)] p-5">
          <SectionHeading icon={<Search className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Semantic search" />
          <p className="mt-1 text-xs text-(--color-muted-foreground)">
            Find past goals and sessions the way you would ask — not by keyword.
          </p>
          <div className="mt-3 flex gap-2">
            <GlassInput
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runSearch()}
              placeholder="Search remembered goals, e.g. resume my focus session"
              aria-label="Search remembered goals"
            />
            <Button onClick={runSearch}>
              <Search className="h-3.5 w-3.5" strokeWidth={1.75} />
              Search
            </Button>
          </div>
          {searched && (
            <div className="mt-3 space-y-2">
              {searched.length === 0 && (
                <p className="text-sm text-(--color-muted-foreground)">No matching memories.</p>
              )}
              {searched.map((hit) => (
                <RecordCard key={hit.record.id} hit={hit} />
              ))}
            </div>
          )}
        </section>

        {/* 2. RECOMMEND — the second most valuable interaction */}
        <section className="glass-panel rounded-[var(--radius-card)] p-5">
          <SectionHeading icon={<TrendingUp className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Recommend workflows" />
          <p className="mt-1 text-xs text-(--color-muted-foreground)">
            Describe a goal and ChronoDesk plans it from the workflows it has already learned.
          </p>
          <div className="mt-3 flex gap-2">
            <GlassInput
              value={recommendGoal}
              onChange={(e) => setRecommendGoal(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && runRecommend()}
              placeholder="Goal to plan for — e.g. resume my focus session"
              aria-label="Goal to plan for"
            />
            <Button onClick={runRecommend}>
              <BrainCircuit className="h-3.5 w-3.5" strokeWidth={1.75} />
              Recommend
            </Button>
          </div>
          {recommendations && (
            <div className="mt-3 space-y-2">
              {recommendations.length === 0 && (
                <p className="text-sm text-(--color-muted-foreground)">
                  No successful workflow learned yet for this goal.
                </p>
              )}
              {recommendations.map((rec) => (
                <RecommendationCard
                  key={rec.record.id}
                  recommendation={rec}
                  onFeedback={(accepted) => void sendFeedback(rec.record.id, accepted)}
                />
              ))}
            </div>
          )}
          {avoided && avoided.length > 0 && (
            <div className="mt-3 space-y-2">
              <p className="flex items-center gap-1.5 text-xs font-medium text-(--color-warning)">
                <ShieldAlert className="h-3.5 w-3.5" strokeWidth={1.75} /> Strategies to avoid:
              </p>
              {avoided.map((strategy) => (
                <div key={strategy.record.id} className="rounded-lg border border-(--color-warning)/25 bg-(--color-warning)/5 p-3">
                  <div className="flex items-center justify-between gap-2">
                    <p className="truncate text-sm font-medium text-(--color-foreground)">
                      {strategy.record.goal}
                    </p>
                    <span className="text-[11px] text-(--color-faint-foreground)">
                      similarity {strategy.similarity.toFixed(2)}
                    </span>
                  </div>
                  <p className="mt-1 text-xs text-(--color-warning)">{strategy.failure}</p>
                </div>
              ))}
            </div>
          )}
        </section>
      </section>

      <section className="grid items-start gap-8 lg:grid-cols-2">
        {/* 3. RECENT MEMORIES */}
        <section className="space-y-3">
          <SectionHeading icon={<History className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Recent memories" />
          <div className="space-y-2">
            {recent.length === 0 && (
              <p className="text-sm text-(--color-muted-foreground)">
                Nothing remembered yet. Completed sessions and runs will appear here.
              </p>
            )}
            {recent.map((hit) => (
              <RecordCard key={hit.record.id} hit={hit} />
            ))}
          </div>
        </section>

        {/* 4. LEARNED CONTEXT */}
        <section className="space-y-3">
          {workflows.length > 0 && (
            <>
              <SectionHeading icon={<BrainCircuit className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Learned workflows" />
              <div className="space-y-2">
                {workflows.slice(0, 5).map((workflow) => (
                  <div key={workflow.goal_fingerprint} className="rounded-[var(--radius-control)] p-3 transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)">
                    <div className="flex items-center justify-between gap-2">
                      <p className="truncate text-sm font-medium text-(--color-foreground)">{workflow.goal}</p>
                      <span className="text-[11px] text-(--color-faint-foreground)">
                        {workflow.success_count} ok · {workflow.failure_count} failed
                      </span>
                    </div>
                    {workflow.best_plan && (
                      <p className="mt-1 font-(family-name:--font-mono) text-[11px] text-(--color-muted-foreground)">
                        {workflow.best_plan.tasks.map((t) => t.description).join(" → ")}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            </>
          )}
        </section>
      </section>

      {stats && (
        <section className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
          <StatCard label="Total runs" value={stats.total_records} />
          <StatCard label="Successful" value={stats.successful} />
          <StatCard label="Failed" value={stats.failed} />
          <StatCard label="Replays" value={stats.total_replays} />
          <StatCard label="Learned workflows" value={stats.learned_workflows} />
        </section>
      )}

      <section className="space-y-5">
        <div>
          <SectionHeading icon={<BrainCircuit className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Learning health" />
          <p className="mt-1.5 text-xs text-(--color-muted-foreground)">
            How well memory is consolidating — retention quality, aging, and failure patterns.
          </p>
        </div>
        <div className="grid items-start gap-4 lg:grid-cols-3">
          <LearningHealthCard health={health} />
          <MemoryAgingCard summary={aging} />
          <FailurePatternsCard patterns={failurePatterns} />
        </div>
      </section>

      <section className="space-y-5 border-t border-(--color-border-subtle) pt-8">
        <div>
          <SectionHeading icon={<Database className="h-3.5 w-3.5" strokeWidth={1.75} />} title="Memory lifecycle" />
          <p className="mt-1.5 text-xs text-(--color-muted-foreground)">
            Vector index, retention policies, snapshots, lineage, and storage.
          </p>
        </div>
        {indexStatus && (
          <section className="glass-panel rounded-[var(--radius-card)] p-5">
            <div className="flex items-center justify-between gap-2">
              <h3 className="flex items-center gap-2 text-[13px] font-semibold text-(--color-foreground)">
                <Database className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
                Vector index
              </h3>
              <Button
                onClick={runReindex}
                disabled={reindexing}
                variant="outline"
                size="sm"
              >
                <RefreshCw className={`h-3.5 w-3.5 ${reindexing ? "animate-spin" : ""}`} strokeWidth={1.75} />
                {reindexing ? "Indexing…" : "Index now"}
              </Button>
            </div>
            <div className="mt-4 grid grid-cols-2 gap-2.5 sm:grid-cols-3">
              <StatCard label="Indexed" value={`${indexStatus.indexed}/${indexStatus.total_records}`} />
              <StatCard label="Pending" value={indexStatus.pending} />
              <StatCard label="Provider" value={`${indexStatus.provider} · ${indexStatus.dimensions}d`} />
              <StatCard label="Cache hit rate" value={`${Math.round(indexStatus.cache_hit_rate * 100)}%`} />
              <StatCard
                label="Last indexed"
                value={
                  indexStatus.last_indexed_at
                    ? new Date(indexStatus.last_indexed_at).toLocaleTimeString()
                    : "never"
                }
              />
            </div>
          </section>
        )}
        <div className="grid gap-4 lg:grid-cols-2">
          <StorageStatsCard stats={storageStats} />
          <RetentionManagerCard />
          <SnapshotManagerCard />
          <LineageExplorerCard />
        </div>
      </section>

      <WorkflowFamiliesCard families={families} />
      <DuplicateGroupsCard groups={duplicates} onMerged={runMergeDuplicates} />
    </PageContainer>
  );
}
