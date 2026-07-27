import { Bookmark, Trash2, Calendar } from "lucide-react";
import type { SavedSearch } from "@/types/search";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

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
    <div className="py-6 border-t border-border">
      <div className="flex items-center gap-2 text-foreground font-semibold mb-4">
        <Bookmark className="h-4 w-4" />
        Saved Searches
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {savedSearches.map((search) => (
          <div
            key={search.id}
            className="flex items-center justify-between p-4 bg-background-secondary border border-border rounded-xl hover:border-primary/50 transition-all cursor-pointer group"
            onClick={() => onSelect(search.query)}
          >
            <div>
              <div className="font-medium text-foreground group-hover:text-primary transition-colors">
                {search.query}
              </div>
              <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground mt-1">
                <Calendar className="h-3 w-3" />
                {formatRelativeTime(search.createdAt)}
              </div>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onDelete(search.id);
              }}
              className="p-2 rounded-lg text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-colors opacity-0 group-hover:opacity-100"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
