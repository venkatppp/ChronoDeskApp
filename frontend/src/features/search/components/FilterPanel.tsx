import { useState, useEffect } from "react";
import { Check, ChevronDown, X } from "lucide-react";
import type { SearchEntityType } from "@/types/search";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { Workspace } from "@/types/workspace";

interface FilterPanelProps {
  entityTypes: SearchEntityType[];
  onEntityTypesChange: (types: SearchEntityType[]) => void;
  workspaceId: string | null;
  onWorkspaceChange: (id: string | null) => void;
  onClear: () => void;
}

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
      onEntityTypesChange(entityTypes.filter((t) => t !== type));
    } else {
      onEntityTypesChange([...entityTypes, type]);
    }
  };

  const activeFilterCount =
    (entityTypes.length > 0 ? 1 : 0) + (workspaceId ? 1 : 0);

  const selectedWorkspace = workspaces.find((w) => w.id === workspaceId);

  return (
    <div className="flex flex-wrap items-center gap-4 py-4 border-b border-(--color-border)">
      <div className="flex bg-(--color-background)-secondary p-1 rounded-lg border border-(--color-border)">
        {(["workspace", "file"] as SearchEntityType[]).map((type) => (
          <button
            key={type}
            onClick={() => toggleEntityType(type)}
            className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              entityTypes.includes(type)
                ? "bg-(--color-accent) text-(--color-accent-foreground) shadow-sm"
                : "text-(--color-muted-foreground) hover:text-(--color-foreground)"
            }`}
          >
            {type.charAt(0).toUpperCase() + type.slice(1)}
          </button>
        ))}
      </div>

      <div className="relative">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={`flex items-center gap-2 px-4 py-1.5 bg-(--color-background)-secondary border border-(--color-border) rounded-lg text-sm font-medium transition-all ${
            workspaceId ? "text-(--color-foreground)" : "text-(--color-muted-foreground)"
          } hover:border-(--color-accent)/50`}
        >
          {selectedWorkspace ? selectedWorkspace.name : "All Workspaces"}
          <ChevronDown className={`h-4 w-4 transition-transform ${isOpen ? "rotate-180" : ""}`} />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 mt-2 w-64 rounded-xl border border-(--color-border) bg-(--color-surface-raised) py-2 shadow-2xl shadow-black/50 z-50 overflow-hidden animate-in fade-in slide-in-from-top-2">
            <button
              onClick={() => {
                onWorkspaceChange(null);
                setIsOpen(false);
              }}
              className="w-full px-4 py-2 text-left text-sm hover:bg-(--color-surface-hover) flex items-center justify-between"
            >
              All Workspaces
              {!workspaceId && <Check className="h-4 w-4 text-(--color-accent)" />}
            </button>
            <div className="h-px bg-(--color-border-subtle) my-1" />
            <div className="max-h-60 overflow-y-auto">
              {workspaces.map((w) => (
                <button
                  key={w.id}
                  onClick={() => {
                    onWorkspaceChange(w.id);
                    setIsOpen(false);
                  }}
                  className="w-full px-4 py-2 text-left text-sm hover:bg-(--color-surface-hover) flex items-center justify-between"
                >
                  <span className="truncate">{w.name}</span>
                  {workspaceId === w.id && <Check className="h-4 w-4 text-(--color-accent)" />}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {activeFilterCount > 0 && (
        <button
          onClick={onClear}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-(--color-muted-foreground) hover:text-(--color-danger) transition-colors ml-auto"
        >
          <X className="h-4 w-4" />
          Clear all
          <span className="bg-(--color-surface-hover) px-1.5 py-0.5 rounded-full text-[10px] text-(--color-muted-foreground)">
            {activeFilterCount}
          </span>
        </button>
      )}
    </div>
  );
}
