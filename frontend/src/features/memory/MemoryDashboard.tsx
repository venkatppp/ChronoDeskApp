// MemoryDashboard - what ChronoDesk has learned from previous executions
// (RC-6 M1): stats, semantic search over remembered runs, workflow
// recommendations for a goal, and strategies to avoid.

import { useCallback, useEffect, useState } from "react";
import { BrainCircuit, History, Search, TrendingUp, ShieldAlert } from "lucide-react";
import { memoryRepository } from "@/services/memoryRepository";
import type {
  AvoidedStrategy,
  ExecutionMemoryRecord,
  LearnedWorkflow,
  MemoryHit,
  MemoryKind,
  MemoryRecommendation,
  MemoryStats,
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

export function MemoryDashboard() {
  const [stats, setStats] = useState<MemoryStats | null>(null);
  const [workflows, setWorkflows] = useState<LearnedWorkflow[]>([]);
  const [recent, setRecent] = useState<MemoryHit[]>([]);
  const [query, setQuery] = useState("");
  const [searched, setSearched] = useState<MemoryHit[] | null>(null);
  const [recommendGoal, setRecommendGoal] = useState("");
  const [recommendations, setRecommendations] = useState<MemoryRecommendation[] | null>(null);
  const [avoided, setAvoided] = useState<AvoidedStrategy[] | null>(null);
  const [loading, setLoading] = useState(true);

  const loadOverview = useCallback(async () => {
    try {
      const [statsData, workflowsData, recentData] = await Promise.all([
        memoryRepository.stats(),
        memoryRepository.learnedWorkflows(),
        memoryRepository.search("", { limit: 6 }),
      ]);
      setStats(statsData);
      setWorkflows(workflowsData);
      setRecent(recentData);
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
              <div key={rec.record.id} className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
                <div className="flex items-center justify-between gap-2">
                  <p className="truncate text-sm font-medium text-(--color-foreground)">{rec.record.goal}</p>
                  <span className="rounded-full border border-emerald-500/40 bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-600">
                    score {rec.score.toFixed(2)}
                  </span>
                </div>
                {rec.record.plan && (
                  <p className="mt-1 font-mono text-[11px] text-(--color-muted-foreground)">
                    {rec.record.plan.tasks.map((t) => t.description).join(" → ")}
                  </p>
                )}
              </div>
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
