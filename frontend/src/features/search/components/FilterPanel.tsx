import { useState, useEffect } from "react";
import { Check, ChevronDown, Filter, X } from "lucide-react";
import type { SearchEntityType } from "@/types/search";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { Workspace } from "@/types/workspace";
import { cn } from "@/utils/cn";

interface FilterPanelProps {
  entityTypes: SearchEntityType[];
  onEntityTypesChange: (types: SearchEntityType[]) => void;
  workspaceId: string | null;
  onWorkspaceChange: (id: string | null) => void;
  onClear: () => void;
}

const ENTITY_LABELS: Record<SearchEntityType, string> = {
  workspace: "Workspaces",
  file: "Files",
};

export function FilterPanel({
  entityTypes,
  onEntityTypesChange,
  workspaceId,
  onWorkspaceChange,
  onClear,
}: FilterPanelProps) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    getWorkspaceRepository().listActiveWorkspaces().then(setWorkspaces);
  }, []);

  const toggleEntityType = (type: SearchEntityType) => {
    if (entityTypes.includes(type)) {
      if (entityTypes.length === 1) return;
      onEntityTypesChange(entityTypes.filter((t) => t !== type));
    } else {
      onEntityTypesChange([...entityTypes, type]);
    }
  };

  const activeFilterCount = (entityTypes.length < 2 ? 1 : 0) + (workspaceId ? 1 : 0);

  const selectedWorkspace = workspaces.find((w) => w.id === workspaceId);

  return (
    <div className="flex flex-wrap items-center gap-3 pt-3">
      <div className="glass-control inline-flex items-center gap-0.5 rounded-[var(--radius-control)] p-1">
        {(["workspace", "file"] as SearchEntityType[]).map((type) => {
          const active = entityTypes.includes(type);
          return (
            <button
              key={type}
              type="button"
              aria-pressed={active}
              onClick={() => toggleEntityType(type)}
              className={cn(
                "flex items-center gap-1.5 rounded-[calc(var(--radius-control)-4px)] px-3 py-1.5 text-[13px] font-medium transition-all duration-150 ease-[var(--ease-premium)]",
                active
                  ? "material-selected text-(--color-foreground)"
                  : "text-(--color-muted-foreground) hover:text-(--color-foreground)",
              )}
            >
              {ENTITY_LABELS[type]}
            </button>
          );
        })}
      </div>

      <div className="relative">
        <button
          onClick={() => setIsOpen(!isOpen)}
          aria-haspopup="listbox"
          aria-expanded={isOpen}
          className={cn(
            "glass-control flex items-center gap-2 rounded-[var(--radius-control)] px-3 py-1.5 text-[13px] font-medium transition-all duration-150",
            workspaceId
              ? "border-(--color-accent)/40 text-(--color-foreground)"
              : "text-(--color-muted-foreground) hover:text-(--color-foreground)",
          )}
        >
          <Filter className="h-3.5 w-3.5" strokeWidth={1.75} />
          <span className="max-w-40 truncate">{selectedWorkspace ? selectedWorkspace.name : "All Workspaces"}</span>
          <ChevronDown className={`h-3.5 w-3.5 transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`} strokeWidth={1.75} />
        </button>

        {isOpen && (
          <div
            role="listbox"
            className="glass-panel absolute top-full left-0 z-50 mt-2 w-64 animate-(--animate-scale-in) overflow-hidden rounded-[var(--radius-control)] py-1.5 shadow-[var(--shadow-pop)]"
          >
            <button
              role="option"
              aria-selected={!workspaceId}
              onClick={() => {
                onWorkspaceChange(null);
                setIsOpen(false);
              }}
              className="flex w-full items-center justify-between px-3.5 py-2 text-left text-[13px] transition-colors hover:bg-(--color-surface-hover)"
            >
              All Workspaces
              {!workspaceId && <Check className="h-4 w-4 text-(--color-accent)" strokeWidth={2} />}
            </button>
            <div className="mx-3 h-px bg-(--color-border-subtle)" />
            <div className="max-h-60 overflow-y-auto">
              {workspaces.map((w) => (
                <button
                  key={w.id}
                  role="option"
                  aria-selected={workspaceId === w.id}
                  onClick={() => {
                    onWorkspaceChange(w.id);
                    setIsOpen(false);
                  }}
                  className="flex w-full items-center justify-between gap-2 px-3.5 py-2 text-left text-[13px] transition-colors hover:bg-(--color-surface-hover)"
                >
                  <span className="truncate">{w.name}</span>
                  {workspaceId === w.id && <Check className="h-4 w-4 shrink-0 text-(--color-accent)" strokeWidth={2} />}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {activeFilterCount > 0 && (
        <button
          onClick={onClear}
          className="ml-auto flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs font-medium text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-danger)"
        >
          <X className="h-3.5 w-3.5" strokeWidth={1.75} />
          Clear filters
        </button>
      )}
    </div>
  );
}
