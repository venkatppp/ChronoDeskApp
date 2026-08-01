// SourcesList - Display information sources with relevance indicators

import { FileText, Clock, Brain, Folder, Share2 } from "lucide-react";
import { cn } from "@/utils/cn";
import type { Source, SourceType } from "@/types/copilot";

interface SourcesListProps {
  sources: Source[];
}

const SOURCE_ICONS: Record<SourceType, typeof FileText> = {
  timeline_event: Clock,
  context_memory: Brain,
  workspace_file: FileText,
  session_history: Folder,
  knowledge_graph: Share2,
};

const SOURCE_LABELS: Record<SourceType, string> = {
  timeline_event: "Timeline",
  context_memory: "Memory",
  workspace_file: "File",
  session_history: "Session",
  knowledge_graph: "Graph",
};

export function SourcesList({ sources }: SourcesListProps) {
  if (sources.length === 0) return null;

  return (
    <div className="mt-2">
      <div className="mb-2 text-xs font-medium text-(--color-muted-foreground)">
        Sources ({sources.length})
      </div>
      <div className="space-y-1.5">
        {sources.map((source, idx) => {
          const Icon = SOURCE_ICONS[source.source_type];
          const relevance = Math.round(source.relevance * 100);

          return (
            <div
              key={idx}
              className="flex items-start gap-2 rounded-lg border border-(--color-border-subtle) bg-(--color-surface-raised) p-2 text-xs"
            >
              <Icon className="mt-0.5 h-3.5 w-3.5 shrink-0 text-(--color-accent)" />
              <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-(--color-foreground)">{source.title}</span>
                  <span className="text-(--color-faint-foreground)">·</span>
                  <span className="text-(--color-faint-foreground)">
                    {SOURCE_LABELS[source.source_type]}
                  </span>
                </div>
                <span className="truncate text-(--color-muted-foreground)">{source.reference}</span>
              </div>
              <div
                className={cn(
                  "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium",
                  relevance >= 80 && "bg-(--color-success)/10 text-(--color-success)",
                  relevance >= 50 && relevance < 80 && "bg-(--color-accent-muted) text-(--color-accent)",
                  relevance < 50 && "bg-(--color-surface-hover) text-(--color-muted-foreground)"
                )}
              >
                {relevance}%
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
