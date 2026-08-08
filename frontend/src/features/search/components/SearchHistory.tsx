import { History, X } from "lucide-react";

interface SearchHistoryProps {
  history: string[];
  onSelect: (query: string) => void;
  onRemove: (query: string) => void;
  onClear: () => void;
}

export function SearchHistory({
  history,
  onSelect,
  onRemove,
  onClear,
}: SearchHistoryProps) {
  if (history.length === 0) return null;

  return (
    <div className="py-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2 text-(--color-foreground) font-semibold">
          <History className="h-4 w-4" />
          Recent Searches
        </div>
        <button
          onClick={onClear}
          className="text-xs font-medium text-(--color-muted-foreground) hover:text-(--color-danger) transition-colors"
        >
          Clear History
        </button>
      </div>
      <div className="flex flex-wrap gap-2">
        {history.map((query) => (
          <div
            key={query}
            className="flex items-center gap-1 group bg-(--color-background)-secondary border border-(--color-border) rounded-full pl-3 pr-2 py-1.5 hover:border-(--color-accent)/50 transition-all cursor-pointer"
            onClick={() => onSelect(query)}
          >
            <span className="text-sm text-(--color-muted-foreground) group-hover:text-(--color-foreground) transition-colors">
              {query}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRemove(query);
              }}
              className="p-1 rounded-full text-(--color-muted-foreground) hover:bg-(--color-background)-tertiary hover:text-(--color-foreground) transition-colors"
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
