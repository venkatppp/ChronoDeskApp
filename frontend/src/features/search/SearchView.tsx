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
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";

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
    <PageContainer className="gap-6">
      <PageHeader
        eyebrow="Intelligence"
        title="Search"
        description="Find anything across your workspaces and files."
        actions={
          query ? (
            <Button variant="secondary" size="sm" onClick={handleSaveSearch}>
              Bookmark this search
            </Button>
          ) : undefined
        }
      />

      <div className="sticky top-0 z-20 -mx-6 px-6 pt-1 pb-3 lg:-mx-8 lg:px-8">
        <GlassSurface material="chrome" className="flex flex-col rounded-2xl px-4 py-3.5">
          <SearchBar onSearch={handleSearch} isLoading={isLoading} />
          <FilterPanel
            entityTypes={entityTypes}
            onEntityTypesChange={handleEntityTypesChange}
            workspaceId={workspaceId}
            onWorkspaceChange={handleWorkspaceChange}
            onClear={handleClearFilters}
          />
        </GlassSurface>
      </div>

      {error && (
        <div className="flex items-center gap-2.5 rounded-[var(--radius-card)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-4 py-3 text-sm text-(--color-danger)">
          {error}
        </div>
      )}

      {!query && (
        <div className="flex animate-(--animate-fade-in) flex-col gap-2">
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
            <EmptyState
              icon={<Search className="h-4 w-4" strokeWidth={1.75} />}
              title="Start typing to search"
              description="Search across your workspaces and files. ChronoDesk remembers recent queries and lets you bookmark searches you run often."
            />
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
    </PageContainer>
  );
}
