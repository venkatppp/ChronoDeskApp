import { useState, useCallback, useEffect } from "react";
import { SearchBar } from "./components/SearchBar";
import { FilterPanel } from "./components/FilterPanel";
import { SearchResults } from "./components/SearchResults";
import { SearchHistory } from "./components/SearchHistory";
import { SavedSearches } from "./components/SavedSearches";
import { getSearchRepository } from "@/services/searchRepository";
import type { SearchResult, SearchEntityType, SavedSearch } from "@/types/search";
import { useAppEvents } from "@/hooks/useAppEvents";
import { Search } from "lucide-react";

export function SearchView() {
  const [query, setQuery] = useState("");
  const [entityTypes, setEntityTypes] = useState<SearchEntityType[]>(["workspace", "file"]);
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [history, setHistory] = useState<string[]>([]);
  const [savedSearches, setSavedSearches] = useState<SavedSearch[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const searchRepo = getSearchRepository();

  const refreshHistory = useCallback(async () => {
    try {
      const h = await searchRepo.getSearchHistory(10);
      setHistory(h);
    } catch (err) {
      console.error("Failed to fetch history:", err);
    }
  }, [searchRepo]);

  const performSearch = useCallback(async (q: string, types: SearchEntityType[], wId: string | null) => {
    if (!q.trim()) {
      setResults([]);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const searchResults = await searchRepo.search({
        query: q,
        entityTypes: types,
        workspaceId: wId,
      });
      setResults(searchResults);
      if (q.trim()) {
        await searchRepo.saveSearchQuery(q);
        refreshHistory();
      }
    } catch (err) {
      console.error("Search failed:", err);
      setError("Failed to perform search. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [searchRepo, refreshHistory]);

  const refreshSavedSearches = useCallback(async () => {
    try {
      const s = await searchRepo.listSavedSearches();
      setSavedSearches(s);
    } catch (err) {
      console.error("Failed to fetch saved searches:", err);
    }
  }, [searchRepo]);

  useEffect(() => {
    refreshHistory();
    refreshSavedSearches();
  }, [refreshHistory, refreshSavedSearches]);

  useAppEvents(["search:indexed"], () => {
    if (query) performSearch(query, entityTypes, workspaceId);
  });

  const handleSearch = (newQuery: string) => {
    setQuery(newQuery);
    performSearch(newQuery, entityTypes, workspaceId);
  };

  const handleEntityTypesChange = (types: SearchEntityType[]) => {
    setEntityTypes(types);
    performSearch(query, types, workspaceId);
  };

  const handleWorkspaceChange = (id: string | null) => {
    setWorkspaceId(id);
    performSearch(query, entityTypes, id);
  };

  const handleClearFilters = () => {
    setEntityTypes(["workspace", "file"]);
    setWorkspaceId(null);
    performSearch(query, ["workspace", "file"], null);
  };

  const handleRemoveHistoryItem = (q: string) => {
    // No per-item delete IPC command exists (only clear_search_history for
    // the whole list), so remove it from local UI state optimistically.
    setHistory((prev) => prev.filter((item) => item !== q));
  };

  const handleClearHistory = async () => {
    await searchRepo.clearSearchHistory();
    refreshHistory();
  };

  const handleSaveSearch = async () => {
    if (!query) return;
    await searchRepo.saveSearch(query);
    refreshSavedSearches();
  };

  const handleDeleteSavedSearch = async (id: string) => {
    await searchRepo.deleteSavedSearch(id);
    refreshSavedSearches();
  };

  return (
    <div className="max-w-5xl mx-auto px-6 py-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold text-(--color-foreground) mb-2">Search</h1>
          <p className="text-(--color-muted-foreground)">Find anything across your workspaces and files.</p>
        </div>
        {query && (
          <button
            onClick={handleSaveSearch}
            className="flex items-center gap-2 px-4 py-2 bg-(--color-accent)/10 text-(--color-accent) hover:bg-(--color-accent)/20 rounded-lg font-medium transition-all"
          >
            Save Query
          </button>
        )}
      </div>

      <div className="sticky top-0 z-10 bg-(--color-background)/80 backdrop-blur-md pb-4">
        <SearchBar onSearch={handleSearch} isLoading={isLoading} />
        <FilterPanel
          entityTypes={entityTypes}
          onEntityTypesChange={handleEntityTypesChange}
          workspaceId={workspaceId}
          onWorkspaceChange={handleWorkspaceChange}
          onClear={handleClearFilters}
        />
      </div>

      {error && (
        <div className="my-6 p-4 bg-(--color-danger)/10 border border-(--color-danger)/20 rounded-xl text-(--color-danger) text-sm">
          {error}
        </div>
      )}

      {!query && (
        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500">
          <SearchHistory
            history={history}
            onSelect={handleSearch}
            onRemove={handleRemoveHistoryItem}
            onClear={handleClearHistory}
          />
          <SavedSearches
            savedSearches={savedSearches}
            onSelect={handleSearch}
            onDelete={handleDeleteSavedSearch}
          />
          
          {history.length === 0 && savedSearches.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-center opacity-50">
              <Search className="h-16 w-16 mb-4 text-(--color-muted-foreground)" />
              <p className="text-lg font-medium">Start typing to search...</p>
            </div>
          )}
        </div>
      )}

      {query && (
        <SearchResults
          results={results}
          isLoading={isLoading}
          onSelect={(result) => console.log("Selected:", result)}
        />
      )}
    </div>
  );
}
