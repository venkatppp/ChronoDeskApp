import { FileText, Folder, Star } from "lucide-react";
import type { SearchResult } from "@/types/search";

interface SearchResultsProps {
  results: SearchResult[];
  isLoading: boolean;
  onSelect: (result: SearchResult) => void;
}

export function SearchResults({ results, isLoading, onSelect }: SearchResultsProps) {
  if (isLoading) {
    return (
      <div className="space-y-4 py-6">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="bg-surface-hover h-32 rounded-xl animate-pulse" />
        ))}
      </div>
    );
  }

  if (results.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <div className="bg-surface-hover p-6 rounded-full mb-4">
          <FileText className="h-12 w-12 text-muted-foreground opacity-20" />
        </div>
        <h3 className="text-xl font-semibold text-foreground mb-2">No results found</h3>
        <p className="text-muted-foreground max-w-sm">
          We couldn't find anything matching your search. Try different keywords or filters.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4 py-6">
      {results.map((result) => (
        <button
          key={`${result.entityType}-${result.entityId}`}
          onClick={() => onSelect(result)}
          className="w-full text-left bg-surface-hover p-5 rounded-xl border border-border hover:border-accent/40 hover:shadow-lg transition-all group"
        >
          <div className="flex items-start gap-4">
            <div className={`p-3 rounded-lg ${
              result.entityType === "workspace" ? "bg-accent/10 text-accent" : "bg-accent-muted/10 text-accent-muted"
            }`}>
              {result.entityType === "workspace" ? (
                <Folder className="h-5 w-5" />
              ) : (
                <FileText className="h-5 w-5" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-1">
                <h4 className="font-semibold text-foreground truncate group-hover:text-accent transition-colors">
                  {result.title}
                </h4>
                <div className="flex items-center gap-2">
                  <span className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground bg-surface-hover px-2 py-0.5 rounded border border-border">
                    {result.entityType}
                  </span>
                  <div className="flex items-center text-amber-500">
                    <Star className="h-3 w-3 fill-current" />
                    <span className="text-[10px] ml-1">{(result.rank * 100).toFixed(0)}</span>
                  </div>
                </div>
              </div>
              <p
                className="text-sm text-muted-foreground line-clamp-2 leading-relaxed"
              >
                {result.snippet.replace(/<[^>]*>/g, "")}
              </p>
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}
