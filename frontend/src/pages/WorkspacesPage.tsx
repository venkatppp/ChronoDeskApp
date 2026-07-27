import { useState, useEffect, useCallback } from "react";
import { Plus, Search, Folder, Calendar, Shield, Trash2, Archive, ArrowRight } from "lucide-react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { Workspace, WorkspaceStatus } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

export function WorkspacesPage() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<WorkspaceStatus | "all">("all");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const workspaceRepo = getWorkspaceRepository();

  const fetchWorkspaces = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const all = await workspaceRepo.listActiveWorkspaces();
      // In a real app we might also fetch archived ones separately if there was a command for it
      setWorkspaces(all);
    } catch (err) {
      console.error("Failed to fetch workspaces:", err);
      setError("Failed to load workspaces. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [workspaceRepo]);

  useEffect(() => {
    fetchWorkspaces();
  }, [fetchWorkspaces]);

  const filteredWorkspaces = workspaces.filter((w) => {
    const matchesSearch = w.name.toLowerCase().includes(searchQuery.toLowerCase()) || 
                          (w.description?.toLowerCase().includes(searchQuery.toLowerCase()) ?? false);
    const matchesStatus = statusFilter === "all" || w.status === statusFilter;
    return matchesSearch && matchesStatus;
  });

  const handleDelete = async (id: string) => {
    if (!confirm("Are you sure you want to delete this workspace? This action cannot be undone.")) return;
    try {
      await workspaceRepo.deleteWorkspace(id);
      fetchWorkspaces();
    } catch (err) {
      console.error("Failed to delete workspace:", err);
      alert("Failed to delete workspace.");
    }
  };

  return (
    <div className="max-w-6xl mx-auto px-6 py-10">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 mb-10">
        <div>
          <h1 className="text-4xl font-bold text-foreground mb-2">Workspaces</h1>
          <p className="text-muted-foreground text-lg">Manage your project environments and watched folders.</p>
        </div>
        <button className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-xl font-bold shadow-lg shadow-primary/20 hover:scale-[1.02] active:scale-[0.98] transition-all">
          <Plus className="h-5 w-5" />
          Create Workspace
        </button>
      </div>

      <div className="flex flex-col md:flex-row gap-4 mb-8">
        <div className="relative flex-1">
          <Search className="absolute left-4 top-1/2 -translate-y-1/2 h-5 w-5 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search by name or description..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full h-12 pl-12 pr-4 bg-background-secondary border border-border rounded-xl focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all text-foreground"
          />
        </div>
        <div className="flex bg-background-secondary p-1 rounded-xl border border-border">
          {(["all", "active", "archived"] as const).map((status) => (
            <button
              key={status}
              onClick={() => setStatusFilter(status)}
              className={`px-6 py-2 text-sm font-bold rounded-lg transition-all capitalize ${
                statusFilter === status
                  ? "bg-primary text-primary-foreground shadow-md"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {status}
            </button>
          ))}
        </div>
      </div>

      {isLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="h-48 bg-background-secondary border border-border rounded-2xl animate-pulse" />
          ))}
        </div>
      ) : error ? (
        <div className="py-20 text-center bg-destructive/5 border border-destructive/10 rounded-3xl">
          <div className="text-4xl mb-4">⚠️</div>
          <h3 className="text-xl font-bold text-foreground mb-2">{error}</h3>
          <button onClick={fetchWorkspaces} className="text-primary font-bold hover:underline">Try again</button>
        </div>
      ) : filteredWorkspaces.length === 0 ? (
        <div className="py-32 text-center bg-background-secondary/30 border-2 border-dashed border-border rounded-3xl">
          <Folder className="h-16 w-16 mx-auto mb-6 text-muted-foreground opacity-20" />
          <h3 className="text-2xl font-bold text-foreground mb-2">No workspaces found</h3>
          <p className="text-muted-foreground max-w-sm mx-auto">
            {searchQuery ? "Try adjusting your search or filters." : "Start by creating your first workspace to organize your files."}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {filteredWorkspaces.map((workspace) => (
            <div
              key={workspace.id}
              className="group bg-background-secondary border border-border rounded-2xl p-6 hover:border-primary/40 hover:shadow-2xl transition-all flex flex-col"
            >
              <div className="flex items-start justify-between mb-4">
                <div className={`p-3 rounded-xl ${workspace.status === 'active' ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'}`}>
                  <Folder className="h-6 w-6" />
                </div>
                <div className="flex items-center gap-1">
                  <div className={`w-2 h-2 rounded-full ${workspace.healthScore > 80 ? 'bg-emerald-500' : workspace.healthScore > 40 ? 'bg-amber-500' : 'bg-destructive'}`} />
                  <span className="text-[10px] font-bold text-muted-foreground uppercase">{workspace.healthScore}% Healthy</span>
                </div>
              </div>
              
              <h3 className="text-xl font-bold text-foreground mb-2 group-hover:text-primary transition-colors truncate">
                {workspace.name}
              </h3>
              <p className="text-sm text-muted-foreground line-clamp-2 mb-6 flex-1 italic">
                {workspace.description || "No description provided."}
              </p>

              <div className="space-y-3 mb-6">
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Calendar className="h-3.5 w-3.5" />
                  Last active {formatRelativeTime(workspace.lastActiveAt)}
                </div>
                {workspace.rootPath && (
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <Shield className="h-3.5 w-3.5" />
                    <span className="truncate">{workspace.rootPath}</span>
                  </div>
                )}
              </div>

              <div className="flex items-center justify-between pt-4 border-t border-border">
                <div className="flex items-center gap-2">
                  <button 
                    onClick={() => handleDelete(workspace.id)}
                    className="p-2 rounded-lg text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-all"
                    title="Delete Workspace"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                  <button 
                    className="p-2 rounded-lg text-muted-foreground hover:bg-background-tertiary hover:text-foreground transition-all"
                    title="Archive Workspace"
                  >
                    <Archive className="h-4 w-4" />
                  </button>
                </div>
                <button className="flex items-center gap-1 text-sm font-bold text-primary hover:gap-2 transition-all">
                  Open
                  <ArrowRight className="h-4 w-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
