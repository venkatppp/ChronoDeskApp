// Context intelligence panel (RC-8 M3): knowledge summary, per-signal
// confidence breakdown, top inference hits, and — for workspace nodes —
// cross-workspace relationships (with recompute), goal clusters, and
// graph context snapshots (with capture). All data comes through the
// thin M3 graph commands; failures degrade to an inline error without
// breaking the rest of the inspector panel.

import { useCallback, useEffect, useState } from "react";
import { Brain, Camera, RefreshCw } from "lucide-react";
import type { GraphRepository } from "@/services/graphRepository";
import type { KgNode } from "@/types/graph";
import type {
  ContextInference,
  ContextIntelSnapshot,
  ContextSignalType,
  GoalCluster,
  KnowledgeSummary,
  WorkspaceSimilarityResult,
} from "@/types/contextIntel";

const SIGNAL_LABELS: Record<ContextSignalType, string> = {
  structural: "Structural",
  semantic: "Semantic",
  temporal: "Recency",
  goalOverlap: "Goal",
  memory: "Memory",
};

interface ContextIntelPanelProps {
  node: KgNode;
  repository: GraphRepository;
}

export function ContextIntelPanel({ node, repository }: ContextIntelPanelProps) {
  const [summary, setSummary] = useState<KnowledgeSummary | null>(null);
  const [inference, setInference] = useState<ContextInference | null>(null);
  const [similarity, setSimilarity] = useState<WorkspaceSimilarityResult | null>(null);
  const [clusters, setClusters] = useState<GoalCluster[]>([]);
  const [snapshots, setSnapshots] = useState<ContextIntelSnapshot[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);

  const isWorkspace = node.nodeType === "workspace";
  const entityId = node.entityId;

  const loadWorkspace = useCallback(async () => {
    const [sim, clusters, snapshots] = await Promise.all([
      repository.graphWorkspaceSimilarity(entityId, true),
      repository.graphGoalClusters(entityId, true),
      repository.graphSnapshotList(entityId, 3),
    ]);
    setSimilarity(sim);
    setClusters(clusters);
    setSnapshots(snapshots);
  }, [repository, entityId]);

  const load = useCallback(async () => {
    setIsBusy(true);
    setError(null);
    setSummary(null);
    setInference(null);
    setSimilarity(null);
    setClusters([]);
    setSnapshots([]);
    try {
      const [summary, inference] = await Promise.all([
        repository.graphKnowledgeSummary(node.nodeType, entityId, true),
        repository.graphInferContext(node.nodeType, entityId, 12, true),
      ]);
      setSummary(summary);
      setInference(inference);
      if (isWorkspace) {
        await loadWorkspace();
      }
    } catch (err) {
      console.error("Failed to load context intelligence:", err);
      setError("Context intelligence failed to load.");
    } finally {
      setIsBusy(false);
    }
  }, [node, entityId, isWorkspace, repository, loadWorkspace]);

  useEffect(() => {
    load();
  }, [load]);

  const handleRecompute = useCallback(async () => {
    setIsBusy(true);
    setError(null);
    try {
      const result = await repository.graphDiscoverCrossWorkspaceRelationships(entityId);
      setSimilarity(result);
    } catch (err) {
      console.error("Failed to recompute workspace relationships:", err);
      setError("Recompute failed.");
    } finally {
      setIsBusy(false);
    }
  }, [repository, entityId]);

  const handleSnapshot = useCallback(async () => {
    setIsBusy(true);
    setError(null);
    try {
      await repository.graphSnapshotCreate(entityId, "manual");
      setSnapshots(await repository.graphSnapshotList(entityId, 3));
    } catch (err) {
      console.error("Failed to capture a context snapshot:", err);
      setError("Snapshot capture failed.");
    } finally {
      setIsBusy(false);
    }
  }, [repository, entityId]);

  const breakdown = inference?.confidence;
  const topHits = inference?.related.slice(0, 6) ?? [];

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Brain className="h-3.5 w-3.5 text-(--color-accent)" strokeWidth={1.75} />
        <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
          Context intelligence
        </p>
        {isBusy && (
          <RefreshCw className="h-3 w-3 animate-spin text-(--color-faint-foreground)" strokeWidth={1.75} />
        )}
      </div>

      {error && (
        <p className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-3 py-2 text-[10px] text-(--color-danger)">
          {error}
        </p>
      )}

      {summary && (
        <div className="grid grid-cols-2 gap-2">
          {summary.points.map((point) => (
            <div
              key={point.label}
              className="rounded-[var(--radius-control)] border border-(--color-border-subtle) bg-(--color-surface-hover) px-2.5 py-1.5"
            >
              <p className="text-[9px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
                {point.label}
              </p>
              <p className="truncate text-xs text-(--color-foreground)" title={point.value}>
                {point.value}
              </p>
              {point.detail && (
                <p className="truncate text-[9px] text-(--color-faint-foreground)" title={point.detail}>
                  {point.detail}
                </p>
              )}
            </div>
          ))}
        </div>
      )}

      {breakdown && (
        <div className="space-y-1.5">
          <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
            Confidence breakdown
          </p>
          {[
            { label: "Structural", value: breakdown.structural },
            { label: "Semantic", value: breakdown.semantic },
            { label: "Memory", value: breakdown.memory },
          ].map((entry) => (
            <div key={entry.label} className="flex items-center gap-2">
              <span className="w-14 shrink-0 text-[10px] text-(--color-muted-foreground)">{entry.label}</span>
              <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-(--color-surface-hover)">
                <div
                  className="h-full rounded-full bg-(--color-accent)"
                  style={{ width: `${Math.round(entry.value * 100)}%` }}
                />
              </div>
              <span className="w-7 shrink-0 text-right font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
                {(entry.value * 100).toFixed(0)}%
              </span>
            </div>
          ))}
          <div className="flex items-center gap-2">
            <span className="w-14 shrink-0 text-[10px] font-semibold text-(--color-foreground)">Total</span>
            <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-(--color-surface-hover)">
              <div
                className="h-full rounded-full bg-(--color-accent-muted)"
                style={{ width: `${Math.round(breakdown.total * 100)}%` }}
              />
            </div>
            <span className="w-7 shrink-0 text-right font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
              {(breakdown.total * 100).toFixed(0)}%
            </span>
          </div>
        </div>
      )}

      {topHits.length > 0 && (
        <div className="space-y-1">
          <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
            Top inferred hits
          </p>
          {topHits.map((hit) => (
            <div
              key={`${hit.node.nodeType}-${hit.node.entityId}`}
              className="flex items-center gap-2 rounded-[var(--radius-control)] px-1.5 py-1"
            >
              <span className="min-w-0 flex-1">
                <span className="block truncate text-xs text-(--color-foreground)">{hit.node.title}</span>
                <span className="block truncate text-[9px] text-(--color-faint-foreground)">{hit.reason}</span>
              </span>
              <span className="shrink-0 rounded bg-(--color-surface-hover) px-1.5 py-0.5 text-[8px] font-semibold uppercase tracking-wide text-(--color-muted-foreground)">
                {SIGNAL_LABELS[hit.signal]}
              </span>
              <span className="w-7 shrink-0 text-right font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
                {(hit.score * 100).toFixed(0)}%
              </span>
            </div>
          ))}
        </div>
      )}

      {isWorkspace && (
        <>
          {similarity && (
            <div className="space-y-1">
              <div className="flex items-center justify-between">
                <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
                  Related workspaces
                </p>
                <button
                  onClick={handleRecompute}
                  disabled={isBusy}
                  className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2 py-0.5 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                >
                  <RefreshCw className={`h-2.5 w-2.5 ${isBusy ? "animate-spin" : ""}`} strokeWidth={1.75} />
                  Recompute
                </button>
              </div>
              {similarity.related.length === 0 ? (
                <p className="text-[10px] text-(--color-faint-foreground)">
                  No related workspaces above the similarity floor.
                </p>
              ) : (
                similarity.related.map((related) => {
                  return (
                    <div key={related.targetWorkspaceId} className="flex items-center gap-2 rounded-[var(--radius-control)] px-1.5 py-1">
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-xs text-(--color-foreground)">{related.targetName}</span>
                        <span className="block truncate text-[9px] text-(--color-faint-foreground)">
                          {related.signals.map((signal) => SIGNAL_LABELS[signal.signal]).join(" · ")}
                          {related.persisted ? " · persisted" : ""}
                        </span>
                      </span>
                      <span className="shrink-0 font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
                        {(related.similarity * 100).toFixed(0)}%
                      </span>
                      <span className="shrink-0 font-(family-name:--font-mono) text-[9px] text-(--color-accent-muted)">
                        conf {(related.confidence * 100).toFixed(0)}%
                      </span>
                    </div>
                  );
                })
              )}
            </div>
          )}

          {clusters.length > 0 && (
            <div className="space-y-1">
              <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
                Goal clusters
              </p>
              {clusters.slice(0, 4).map((cluster) => (
                <div key={cluster.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-1.5 py-1">
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-xs text-(--color-foreground)">{cluster.name}</span>
                    <span className="block truncate text-[9px] text-(--color-faint-foreground)">
                      {cluster.centroidTerms.join(", ")}
                    </span>
                  </span>
                  <span className="shrink-0 rounded bg-(--color-surface-hover) px-1.5 py-0.5 text-[8px] font-semibold text-(--color-muted-foreground)">
                    {cluster.memberCount} member{cluster.memberCount === 1 ? "" : "s"}
                  </span>
                </div>
              ))}
            </div>
          )}

          {snapshots && (
            <div className="space-y-1">
              <div className="flex items-center justify-between">
                <p className="text-[10px] font-bold uppercase tracking-wider text-(--color-faint-foreground)">
                  Context snapshots
                </p>
                <button
                  onClick={handleSnapshot}
                  disabled={isBusy}
                  className="flex items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border-subtle) px-2 py-0.5 text-[10px] font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) disabled:opacity-50"
                >
                  <Camera className="h-2.5 w-2.5" strokeWidth={1.75} />
                  Capture
                </button>
              </div>
              {snapshots.length === 0 ? (
                <p className="text-[10px] text-(--color-faint-foreground)">No snapshots captured yet.</p>
              ) : (
                snapshots.map((snapshot) => (
                  <div key={snapshot.id} className="flex items-center gap-2 rounded-[var(--radius-control)] px-1.5 py-1">
                    <span className="min-w-0 flex-1">
                      <span className="block text-xs text-(--color-foreground)">
                        {snapshot.nodeCount} nodes · {snapshot.edgeCount} edges
                      </span>
                      <span className="block text-[9px] text-(--color-faint-foreground)">
                        {snapshot.snapshotType} · {new Date(snapshot.createdAt).toLocaleString()}
                      </span>
                    </span>
                    <span className="shrink-0 font-(family-name:--font-mono) text-[9px] text-(--color-faint-foreground)">
                      {(snapshot.confidence * 100).toFixed(0)}%
                    </span>
                  </div>
                ))
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
