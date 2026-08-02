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
import { MemoryAgingCard } from "@/features/memory/components/MemoryAgingCard";
import { WorkflowFamiliesCard } from "@/features/memory/components/WorkflowFamiliesCard";
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
      ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-600"
      : record.status === "failed"
        ? "border-red-500/40 bg-red-500/10 text-red-500"
        : "border-amber-500/40 bg-amber-500/10 text-amber-600";
  return (
    <span
      className={`rounded-full border px-2 py-0.5 text-[11px] font-medium ${tone}`}
    >
      {STATUS_LABELS[record.status]}
    </span>
  );
}

function StatCard({ label, value }: { label: string; value: number | string }) {
  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
      <p className="text-xs text-(--color-muted-foreground)">{label}</p>
      <p className="mt-1 font-(family-name:--font-display) text-xl font-semibold text-(--color-foreground)">
        {value}
      </p>
    </div>
  );
}

function RecordCard({ hit }: { hit: MemoryHit }) {
  const record = hit.record;
  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
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
        <p className="mt-1.5 truncate font-mono text-[11px] text-(--color-muted-foreground)">
          {record.tools_used.join(" → ")}
        </p>
      )}
      {record.error && (
        <p className="mt-1.5 truncate text-[11px] text-red-500">{record.error}</p>
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
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
      <div className="flex items-center justify-between gap-2">
        <p className="truncate text-sm font-medium text-(--color-foreground)">{record.goal}</p>
        <div className="flex shrink-0 items-center gap-1.5">
          <span className="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-600">
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
        <p className="mt-1 font-mono text-[11px] text-(--color-muted-foreground)">
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
                    ? "font-medium text-emerald-600"
                    : factor.impact < 0
                      ? "font-medium text-red-500"
                      : "font-medium text-(--color-muted-foreground)"
                }
              >
                {factor.impact > 0 ? "▲" : factor.impact < 0 ? "▼" : "·"} {factor.factor}
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
          className="flex items-center gap-1 rounded-md border border-emerald-500/40 bg-emerald-500/10 px-2 py-1 text-[11px] font-medium text-emerald-600 transition-opacity hover:opacity-80"
        >
          <Check className="h-3 w-3" /> Accept
        </button>
        <button
          onClick={() => onFeedback(false)}
          title="Reject this recommendation"
          className="flex items-center gap-1 rounded-md border border-red-500/40 bg-red-500/10 px-2 py-1 text-[11px] font-medium text-red-500 transition-opacity hover:opacity-80"
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

  const loadOverview = useCallback(async () => {
    try {
      const [statsData, indexData, workflowsData, recentData, healthData, familiesData, agingData, failuresData, duplicatesData] =
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
    <div className="mx-auto max-w-5xl space-y-6 p-6">
      <header>
        <div className="flex items-center gap-2 mb-2">
          <BrainCircuit className="h-5 w-5 text-(--color-accent)" />
          <h1 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
            Execution Memory
          </h1>
        </div>
        <p className="text-sm text-(--color-muted-foreground)">
          What ChronoDesk has learned from previous runs — searchable history,
          workflow recommendations, and failed strategies to avoid.
        </p>
      </header>

      {loading && (
        <p className="text-sm text-(--color-muted-foreground)">Loading memory…</p>
      )}

      {stats && (
        <section className="grid grid-cols-2 gap-3 sm:grid-cols-4 lg:grid-cols-5">
          <StatCard label="Total runs" value={stats.total_records} />
          <StatCard label="Successful" value={stats.successful} />
          <StatCard label="Failed" value={stats.failed} />
          <StatCard label="Replays" value={stats.total_replays} />
          <StatCard label="Learned workflows" value={stats.learned_workflows} />
        </section>
      )}

      <section className="grid gap-4 md:grid-cols-2">
        <LearningHealthCard health={health} />
        <div className="space-y-4">
          <MemoryAgingCard summary={aging} />
          <FailurePatternsCard patterns={failurePatterns} />
        </div>
      </section>

      <WorkflowFamiliesCard families={families} />
      <DuplicateGroupsCard groups={duplicates} onMerged={runMergeDuplicates} />

      {indexStatus && (
        <section className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-4">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Database className="h-4 w-4 text-(--color-accent)" />
              <h2 className="text-sm font-medium text-(--color-foreground)">Vector index</h2>
            </div>
            <button
              onClick={runReindex}
              disabled={reindexing}
              className="flex items-center gap-1.5 rounded-md border border-(--color-border) px-3 py-1.5 text-xs font-medium text-(--color-foreground) transition-opacity hover:opacity-80 disabled:opacity-50"
            >
              <RefreshCw className={reindexing ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />
              {reindexing ? "Indexing…" : "Index now"}
            </button>
          </div>
          <div className="mt-3 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
            <StatCard
              label="Indexed"
              value={`${indexStatus.indexed}/${indexStatus.total_records}`}
            />
            <StatCard label="Pending" value={indexStatus.pending} />
            <StatCard
              label="Provider"
              value={`${indexStatus.provider} · ${indexStatus.dimensions}d`}
            />
            <StatCard
              label="Cache hit rate"
              value={`${Math.round(indexStatus.cache_hit_rate * 100)}%`}
            />
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

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <Search className="h-4 w-4 text-(--color-accent)" />
          <h2 className="text-sm font-medium text-(--color-foreground)">Semantic search</h2>
        </div>
        <div className="flex gap-2">
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runSearch()}
            placeholder="Search remembered goals, e.g. resume my focus session"
            className="min-w-0 flex-1 rounded-md border border-(--color-border) bg-(--color-surface-raised) px-3 py-2 text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground)"
          />
          <button
            onClick={runSearch}
            className="rounded-md bg-(--color-accent) px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90"
          >
            Search
          </button>
        </div>
        {searched && (
          <div className="space-y-2">
            {searched.length === 0 && (
              <p className="text-sm text-(--color-muted-foreground)">No matching memories.</p>
            )}
            {searched.map((hit) => (
              <RecordCard key={hit.record.id} hit={hit} />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <TrendingUp className="h-4 w-4 text-(--color-accent)" />
          <h2 className="text-sm font-medium text-(--color-foreground)">Recommend workflows</h2>
        </div>
        <div className="flex gap-2">
          <input
            value={recommendGoal}
            onChange={(e) => setRecommendGoal(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && runRecommend()}
            placeholder="Goal to plan for — e.g. resume my focus session"
            className="min-w-0 flex-1 rounded-md border border-(--color-border) bg-(--color-surface-raised) px-3 py-2 text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground)"
          />
          <button
            onClick={runRecommend}
            className="rounded-md bg-(--color-accent) px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90"
          >
            Recommend
          </button>
        </div>
        {recommendations && (
          <div className="space-y-2">
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
          <div className="space-y-2">
            <p className="flex items-center gap-1.5 text-xs text-amber-600">
              <ShieldAlert className="h-3.5 w-3.5" /> Strategies to avoid:
            </p>
            {avoided.map((strategy) => (
              <div key={strategy.record.id} className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-3">
                <div className="flex items-center justify-between gap-2">
                  <p className="truncate text-sm font-medium text-(--color-foreground)">
                    {strategy.record.goal}
                  </p>
                  <span className="text-[11px] text-(--color-faint-foreground)">
                    similarity {strategy.similarity.toFixed(2)}
                  </span>
                </div>
                <p className="mt-1 text-xs text-amber-600">{strategy.failure}</p>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex items-center gap-2">
          <History className="h-4 w-4 text-(--color-accent)" />
          <h2 className="text-sm font-medium text-(--color-foreground)">Recent memories</h2>
        </div>
        <div className="space-y-2">
          {recent.length === 0 && (
            <p className="text-sm text-(--color-muted-foreground)">
              No executions remembered yet. Run a plan or autonomous session and it will appear here.
            </p>
          )}
          {recent.map((hit) => (
            <RecordCard key={hit.record.id} hit={hit} />
          ))}
        </div>
      </section>

      {workflows.length > 0 && (
        <section className="space-y-3">
          <div className="flex items-center gap-2">
            <BrainCircuit className="h-4 w-4 text-(--color-accent)" />
            <h2 className="text-sm font-medium text-(--color-foreground)">Learned workflows</h2>
          </div>
          <div className="space-y-2">
            {workflows.slice(0, 5).map((workflow) => (
              <div key={workflow.goal_fingerprint} className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
                <div className="flex items-center justify-between gap-2">
                  <p className="truncate text-sm font-medium text-(--color-foreground)">{workflow.goal}</p>
                  <span className="text-[11px] text-(--color-faint-foreground)">
                    {workflow.success_count} ok · {workflow.failure_count} failed
                  </span>
                </div>
                {workflow.best_plan && (
                  <p className="mt-1 font-mono text-[11px] text-(--color-muted-foreground)">
                    {workflow.best_plan.tasks.map((t) => t.description).join(" → ")}
                  </p>
                )}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
