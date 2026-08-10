import { History, X, RotateCcw } from "lucide-react";
import { SectionLabel } from "@/components/ui/SectionLabel";

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
    <section className="glass-panel rounded-[var(--radius-card)] p-5">
      <SectionLabel
        icon={<History className="h-3.5 w-3.5" strokeWidth={1.75} />}
        right={
          <button
            onClick={onClear}
            className="flex items-center gap-1 text-[11px] font-medium text-(--color-faint-foreground) transition-colors hover:text-(--color-danger)"
          >
            <RotateCcw className="h-3 w-3" strokeWidth={1.75} />
            Clear history
          </button>
        }
      >
        Recent Searches
      </SectionLabel>
      <div className="mt-3 flex flex-col">
        {history.map((query) => (
          <div
            key={query}
            className="group flex cursor-pointer items-center gap-2 rounded-[var(--radius-control)] px-2 py-2 transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)"
            onClick={() => onSelect(query)}
          >
            <span className="min-w-0 flex-1 truncate text-[13px] text-(--color-muted-foreground) transition-colors group-hover:text-(--color-foreground)">
              {query}
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRemove(query);
              }}
              aria-label={`Remove ${query} from history`}
              className="rounded-md p-1 text-(--color-faint-foreground) opacity-0 transition-all hover:bg-(--color-danger)/10 hover:text-(--color-danger) group-hover:opacity-100"
            >
              <X className="h-3.5 w-3.5" strokeWidth={2} />
            </button>
          </div>
        ))}
      </div>
    </section>
  );
}
