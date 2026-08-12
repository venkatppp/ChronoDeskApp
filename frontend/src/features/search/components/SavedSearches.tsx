import { Bookmark, Trash2, Calendar } from "lucide-react";
import type { SavedSearch } from "@/types/search";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import { Card } from "@/components/ui/Card";
import { SectionLabel } from "@/components/ui/SectionLabel";

interface SavedSearchesProps {
  savedSearches: SavedSearch[];
  onSelect: (query: string) => void;
  onDelete: (id: string) => void;
}

export function SavedSearches({
  savedSearches,
  onSelect,
  onDelete,
}: SavedSearchesProps) {
  if (savedSearches.length === 0) return null;

  return (
    <Card className="p-5">
      <SectionLabel icon={<Bookmark className="h-3.5 w-3.5" strokeWidth={1.75} />}>
        Saved Searches
      </SectionLabel>
      <div className="mt-3 flex flex-col">
        {savedSearches.map((search) => (
          <div
            key={search.id}
            className="group flex cursor-pointer items-center justify-between gap-3 rounded-[var(--radius-control)] px-2 py-2.5 transition-colors duration-150 ease-[var(--ease-premium)] hover:bg-(--color-surface-hover)"
            onClick={() => onSelect(search.query)}
          >
            <div className="min-w-0">
              <div className="truncate text-sm font-medium text-(--color-foreground) transition-colors group-hover:text-(--color-accent)">
                {search.query}
              </div>
              <div className="mt-0.5 flex items-center gap-1.5 text-[11px] text-(--color-faint-foreground)">
                <Calendar className="h-3 w-3" strokeWidth={1.75} />
                Saved {formatRelativeTime(search.createdAt)}
              </div>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDelete(search.id);
              }}
              aria-label={`Delete saved search ${search.query}`}
              className="shrink-0 rounded-lg p-1.5 text-(--color-faint-foreground) opacity-0 transition-all hover:bg-(--color-danger)/10 hover:text-(--color-danger) group-hover:opacity-100"
            >
              <Trash2 className="h-4 w-4" strokeWidth={1.75} />
            </button>
          </div>
        ))}
      </div>
    </Card>
  );
}
