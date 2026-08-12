import { FileCode, FileJson, FileText, Folder, Star, Loader2 } from "lucide-react";
import type { SearchResult } from "@/types/search";
import { cn } from "@/utils/cn";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";

interface SearchResultsProps {
  results: SearchResult[];
  isLoading: boolean;
  onSelect: (result: SearchResult) => void;
}

function extensionOf(title: string): string | null {
  const ext = title.split(".").pop()?.toLowerCase();
  if (!ext || title.endsWith(ext)) return ext && title.includes(".") ? ext : null;
  return null;
}

/** SF-Symbol-like file icon for a result's extension. */
function FileGlyph({ title, className }: { title: string; className?: string }) {
  const ext = extensionOf(title);
  if (ext === "ts" || ext === "tsx" || ext === "rs") {
    return <FileCode className={className} strokeWidth={1.75} />;
  }
  if (ext === "js" || ext === "jsx" || ext === "json" || ext === "toml" || ext === "yml" || ext === "yaml") {
    return <FileJson className={className} strokeWidth={1.75} />;
  }
  return <FileText className={className} strokeWidth={1.75} />;
}

export function SearchResults({ results, isLoading, onSelect }: SearchResultsProps) {
  if (isLoading) {
    return (
      <div className="flex flex-col gap-3">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-20 animate-pulse rounded-[var(--radius-card)] bg-(--color-surface)" />
        ))}
      </div>
    );
  }

  if (results.length === 0) {
    return (
      <EmptyState
        icon={<FileText className="h-5 w-5" strokeWidth={1.5} />}
        title="No results found"
        description="We couldn't find anything matching your search. Try different keywords or widen the filters."
      />
    );
  }

  return (
    <div className="flex flex-col gap-3 pb-8">
      <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-(--color-faint-foreground)">
        <Loader2 className="h-3 w-3" strokeWidth={1.75} />
        {results.length} result{results.length !== 1 ? "s" : ""}
      </div>
      <Card className="divide-y divide-(--color-border-subtle) overflow-hidden">
        {results.map((result) => {
          const isWorkspace = result.entityType === "workspace";
          return (
            <button
              key={`${result.entityType}-${result.entityId}`}
              onClick={() => onSelect(result)}
              className="group flex w-full items-start gap-4 px-5 py-4 text-left transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)"
            >
              <span
                className={cn(
                  "flex h-10 w-10 shrink-0 items-center justify-center rounded-[var(--radius-control)] ring-1 ring-(--color-border-subtle)",
                  isWorkspace
                    ? "bg-(--color-blue)/12 text-(--color-blue)"
                    : "bg-(--color-cyan)/12 text-(--color-cyan)",
                )}
              >
                {isWorkspace ? (
                  <Folder className="h-4.5 w-4.5" strokeWidth={1.75} />
                ) : (
                  <FileGlyph title={result.title} className="h-4.5 w-4.5" />
                )}
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <h4 className="truncate text-sm font-semibold text-(--color-foreground) transition-colors group-hover:text-(--color-accent)">
                    {result.title}
                  </h4>
                  <span className="shrink-0 text-[10px] font-medium uppercase tracking-wider text-(--color-faint-foreground)">
                    {result.entityType}
                  </span>
                  <span className="ml-auto flex shrink-0 items-center gap-1 text-[10px] font-medium text-(--color-faint-foreground)">
                    <Star className="h-3 w-3 fill-(--color-warning) text-(--color-warning)" strokeWidth={1.75} />
                    {(result.rank * 100).toFixed(0)}
                  </span>
                </div>
                <p className="mt-1 line-clamp-2 text-[13px] leading-relaxed text-(--color-muted-foreground)">
                  {result.snippet.replace(/<[^>]*>/g, "")}
                </p>
              </div>
            </button>
          );
        })}
      </Card>
    </div>
  );
}
