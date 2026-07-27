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
    <div className="flex flex-wrap items-center gap-4 py-4 border-b border-border">
      <div className="flex bg-background-secondary p-1 rounded-lg border border-border">
        {(["workspace", "file"] as SearchEntityType[]).map((type) => (
          <button
            key={type}
            onClick={() => toggleEntityType(type)}
            className={`px-4 py-1.5 text-sm font-medium rounded-md transition-all ${
              entityTypes.includes(type)
                ? "bg-primary text-primary-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {type.charAt(0).toUpperCase() + type.slice(1)}
          </button>
        ))}
      </div>

      <div className="relative">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={`flex items-center gap-2 px-4 py-1.5 bg-background-secondary border border-border rounded-lg text-sm font-medium transition-all ${
            workspaceId ? "text-foreground" : "text-muted-foreground"
          } hover:border-primary/50`}
        >
          {selectedWorkspace ? selectedWorkspace.name : "All Workspaces"}
          <ChevronDown className={`h-4 w-4 transition-transform ${isOpen ? "rotate-180" : ""}`} />
        </button>

        {isOpen && (
          <div className="absolute top-full left-0 mt-2 w-64 bg-background-secondary border border-border rounded-xl shadow-2xl z-50 py-2 overflow-hidden animate-in fade-in slide-in-from-top-2">
            <button
              onClick={() => {
                onWorkspaceChange(null);
                setIsOpen(false);
              }}
              className="w-full px-4 py-2 text-left text-sm hover:bg-background-tertiary flex items-center justify-between"
            >
              All Workspaces
              {!workspaceId && <Check className="h-4 w-4 text-primary" />}
            </button>
            <div className="h-px bg-border my-1" />
            <div className="max-h-60 overflow-y-auto">
              {workspaces.map((w) => (
                <button
                  key={w.id}
                  onClick={() => {
                    onWorkspaceChange(w.id);
                    setIsOpen(false);
                  }}
                  className="w-full px-4 py-2 text-left text-sm hover:bg-background-tertiary flex items-center justify-between"
                >
                  <span className="truncate">{w.name}</span>
                  {workspaceId === w.id && <Check className="h-4 w-4 text-primary" />}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {activeFilterCount > 0 && (
        <button
          onClick={onClear}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-muted-foreground hover:text-destructive transition-colors ml-auto"
        >
          <X className="h-4 w-4" />
          Clear all
          <span className="bg-muted px-1.5 py-0.5 rounded-full text-[10px] text-muted-foreground">
            {activeFilterCount}
          </span>
        </button>
      )}
    </div>
  );
}
