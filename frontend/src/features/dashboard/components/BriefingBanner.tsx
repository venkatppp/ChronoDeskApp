import { Sparkles, AlertTriangle, CheckCircle2, Clock, ArrowRight, FileText, Code } from "lucide-react";
import type { ProductivityBrief } from "@/types/workspace";

interface BriefingBannerProps {
  briefing: ProductivityBrief | null;
  isLoading: boolean;
}

export function BriefingBanner({ briefing, isLoading }: BriefingBannerProps) {
  if (isLoading) {
    return (
      <div className="glass-panel relative overflow-hidden rounded-[var(--radius-card)] p-5">
        <div className="animate-pulse space-y-3">
          <div className="h-5 w-48 rounded bg-(--color-surface-hover)" />
          <div className="h-4 w-96 rounded bg-(--color-surface-hover)" />
          <div className="flex gap-3">
            <div className="h-4 w-32 rounded bg-(--color-surface-hover)" />
            <div className="h-4 w-32 rounded bg-(--color-surface-hover)" />
            <div className="h-4 w-32 rounded bg-(--color-surface-hover)" />
          </div>
        </div>
      </div>
    );
  }

  if (!briefing || briefing.workspacesCount === 0) {
    return (
      <div className="glass-panel relative overflow-hidden rounded-[var(--radius-card)] p-5">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-(--color-surface-raised)">
            <Sparkles className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
          </div>
          <div>
            <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">Getting Started</p>
            <p className="mt-1 text-sm leading-relaxed text-(--color-foreground)">
              No active workspaces yet. Create one, or watch a folder from Settings to get started.
            </p>
          </div>
        </div>
      </div>
    );
  }

  const {
    greeting, lastActiveRelative, workspacesCount, healthyCount, attentionCount,
    topWorkspaceName, attentionWorkspaces, hoursWorked, filesEdited,
    mostActiveLanguage,
  } = briefing;

  return (
    <div className="glass-panel relative overflow-hidden rounded-[var(--radius-card)] p-5">
      <div className="absolute right-0 top-0 h-24 w-24 opacity-[0.03]">
        <div className="h-full w-full rounded-bl-full bg-(--color-violet)" />
      </div>
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
          <Sparkles className="h-3.5 w-3.5" strokeWidth={1.75} />
          Today&apos;s Brief
        </div>
        <p className="text-xl font-semibold text-(--color-foreground)">
          {greeting}.
        </p>
        <div className="space-y-1.5 text-sm leading-relaxed text-(--color-muted-foreground)">
          <p>
            {lastActiveRelative ? (
              <>You worked on <span className="font-medium text-(--color-foreground)">{topWorkspaceName}</span> {lastActiveRelative}.</>
            ) : (
              <>{workspacesCount} workspace{workspacesCount !== 1 ? "s" : ""} available.</>
            )}
          </p>
          <div className="flex flex-wrap gap-x-6 gap-y-1">
            <span className="inline-flex items-center gap-1.5">
              <CheckCircle2 className="h-3.5 w-3.5 text-(--color-success)" strokeWidth={2} />
              {healthyCount} healthy
            </span>
            {attentionCount > 0 && (
              <span className="inline-flex items-center gap-1.5">
                <AlertTriangle className="h-3.5 w-3.5 text-(--color-warning)" strokeWidth={2} />
                {attentionCount} need{attentionCount === 1 ? "s" : ""} attention
              </span>
            )}
            <span className="inline-flex items-center gap-1.5">
              <Clock className="h-3.5 w-3.5 text-(--color-muted-foreground)" strokeWidth={2} />
              {workspacesCount} active workspace{workspacesCount !== 1 ? "s" : ""}
            </span>
          </div>
        </div>

        {(hoursWorked > 0 || filesEdited > 0 || mostActiveLanguage) && (
          <div className="flex flex-wrap gap-4 text-sm">
            {hoursWorked > 0 && (
              <span className="inline-flex items-center gap-1.5 text-(--color-muted-foreground)">
                <Clock className="h-3.5 w-3.5" strokeWidth={1.75} />
                <span className="font-medium text-(--color-foreground)">{hoursWorked}h</span> worked today
              </span>
            )}
            {filesEdited > 0 && (
              <span className="inline-flex items-center gap-1.5 text-(--color-muted-foreground)">
                <FileText className="h-3.5 w-3.5" strokeWidth={1.75} />
                <span className="font-medium text-(--color-foreground)">{filesEdited}</span> edits
              </span>
            )}
            {mostActiveLanguage && (
              <span className="inline-flex items-center gap-1.5 text-(--color-muted-foreground)">
                <Code className="h-3.5 w-3.5" strokeWidth={1.75} />
                <span className="font-medium text-(--color-foreground)">{mostActiveLanguage}</span>
              </span>
            )}
          </div>
        )}

        {attentionWorkspaces.length > 0 && (
          <div className="mt-1 flex flex-wrap gap-2">
            {attentionWorkspaces.map((w) => (
              <span
                key={w.name}
                className="inline-flex items-center gap-1.5 rounded-full bg-(--color-warning)/10 px-2.5 py-1 text-xs font-medium text-(--color-warning)"
              >
                <AlertTriangle className="h-3 w-3" strokeWidth={2} />
                {w.name} ({w.health}%)
              </span>
            ))}
          </div>
        )}
        {lastActiveRelative && (
          <div className="mt-1 flex items-center gap-1.5 text-sm">
            <span className="text-(--color-muted-foreground)">Recommended next:</span>
            <span className="inline-flex items-center gap-1 font-medium text-(--color-accent)">
              Continue {topWorkspaceName}
              <ArrowRight className="h-3 w-3" strokeWidth={2} />
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
