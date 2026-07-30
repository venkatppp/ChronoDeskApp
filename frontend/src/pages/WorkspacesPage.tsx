import { useState, useEffect, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, Search, Folder, Calendar, Shield, Trash2, Archive, ArrowRight, Pencil, RotateCcw } from "lucide-react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { UpdateWorkspaceInput, Workspace, WorkspaceStatus } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";

export function WorkspacesPage() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<WorkspaceStatus | "all">("all");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Dialog state for creating workspace
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [workspaceName, setWorkspaceName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Dialog state for editing workspace
  const [editingWorkspace, setEditingWorkspace] = useState<Workspace | null>(null);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [isUpdating, setIsUpdating] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);

  const workspaceRepo = getWorkspaceRepository();
  const navigate = useNavigate();
  const handleOpenWorkspace = async (workspace: Workspace) => {
    try {
      await workspaceRepo.switchWorkspace(workspace.id);
      localStorage.setItem("activeWorkspaceId", workspace.id);
      navigate("/timeline");
    } catch (err) {
      console.error("Failed to switch workspace:", err);
      alert("Failed to open workspace.");
    }
  };

  const fetchWorkspaces = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      let all: Workspace[];
      if (statusFilter === "archived") {
        all = await workspaceRepo.listArchivedWorkspaces();
      } else if (statusFilter === "active") {
        all = await workspaceRepo.listActiveWorkspaces();
      } else {
        const [active, archived] = await Promise.all([
          workspaceRepo.listActiveWorkspaces(),
          workspaceRepo.listArchivedWorkspaces(),
        ]);
        all = [...active, ...archived];
      }
      setWorkspaces(all);
    } catch (err) {
      console.error("Failed to fetch workspaces:", err);
      setError("Failed to load workspaces. Please try again.");
    } finally {
      setIsLoading(false);
    }
  }, [workspaceRepo, statusFilter]);

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

  const handleArchive = async (workspace: Workspace) => {
    try {
      await workspaceRepo.updateWorkspace(workspace.id, { status: "archived" });
      fetchWorkspaces();
    } catch (err) {
      console.error("Failed to archive workspace:", err);
      alert("Failed to archive workspace.");
    }
  };

  const handleRestore = async (workspace: Workspace) => {
    try {
      await workspaceRepo.updateWorkspace(workspace.id, { status: "active" });
      fetchWorkspaces();
    } catch (err) {
      console.error("Failed to restore workspace:", err);
      alert("Failed to restore workspace.");
    }
  };

  // Dialog helpers
  function closeCreateDialog() {
    setShowCreateDialog(false);
    setWorkspaceName("");
    setCreateError(null);
  }

  const handleCreateWorkspace = async () => {
    const name = workspaceName.trim();
    if (!name) return;
    setIsCreating(true);
    setCreateError(null);
    try {
      await workspaceRepo.createWorkspace({ name });
      await fetchWorkspaces();
      closeCreateDialog();
    } catch (err: any) {
      setCreateError(err?.message || "Failed to create workspace. Please try again.");
    } finally {
      setIsCreating(false);
    }
  };

  function openEditDialog(workspace: Workspace) {
    setEditingWorkspace(workspace);
    setEditName(workspace.name);
    setEditDescription(workspace.description ?? "");
    setUpdateError(null);
  }

  function closeEditDialog() {
    setEditingWorkspace(null);
    setEditName("");
    setEditDescription("");
    setUpdateError(null);
  }

  const handleUpdateWorkspace = async () => {
    const ws = editingWorkspace;
    if (!ws || !hasEditChanges) return;
    const trimmed = editName.trim();
    if (!trimmed) return;
    setIsUpdating(true);
    setUpdateError(null);
    const input: UpdateWorkspaceInput = {};
    if (trimmed !== ws.name) input.name = trimmed;
    if (editDescription !== (ws.description ?? "")) {
      input.description = editDescription || null;
    }
    try {
      await workspaceRepo.updateWorkspace(ws.id, input);
      await fetchWorkspaces();
      closeEditDialog();
    } catch (err: any) {
      setUpdateError(err?.message || "Failed to update workspace.");
    } finally {
      setIsUpdating(false);
    }
  };

  // Derived: whether the edit form differs from the original workspace
  const hasEditChanges = editingWorkspace
    ? editName.trim() !== editingWorkspace.name ||
      editDescription !== (editingWorkspace.description ?? "")
    : false;

  // Refs for auto-focus
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (showCreateDialog && inputRef.current) {
      inputRef.current.focus();
    }
  }, [showCreateDialog]);

  const editInputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (editingWorkspace && editInputRef.current) {
      editInputRef.current.focus();
    }
  }, [editingWorkspace]);

  return (
    <>
      <div className="max-w-6xl mx-auto px-6 py-10">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-6 mb-10">
          <div>
            <h1 className="text-4xl font-bold text-foreground mb-2">Workspaces</h1>
            <p className="text-muted-foreground text-lg">Manage your project environments and watched folders.</p>
          </div>
          <button
            className="flex items-center gap-2 px-6 py-3 bg-primary text-primary-foreground rounded-xl font-bold shadow-lg shadow-primary/20 hover:scale-[1.02] active:scale-[0.98] transition-all"
            onClick={() => setShowCreateDialog(true)}
            disabled={isCreating}
          >
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

        {createError && (
          <div className="mb-6 bg-destructive/10 border border-destructive/30 rounded-xl px-6 py-4 text-destructive font-bold text-center">
            {createError}
          </div>
        )}
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
                      onClick={() => openEditDialog(workspace)}
                      className="p-2 rounded-lg text-muted-foreground hover:bg-background-tertiary hover:text-foreground transition-all"
                      title="Edit Workspace"
                    >
                      <Pencil className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => handleDelete(workspace.id)}
                      className="p-2 rounded-lg text-muted-foreground hover:bg-destructive/10 hover:text-destructive transition-all"
                      title="Delete Workspace"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                    {workspace.status === "active"
                      ? <button
                          onClick={() => handleArchive(workspace)}
                          className="p-2 rounded-lg text-muted-foreground hover:bg-background-tertiary hover:text-foreground transition-all"
                          title="Archive Workspace"
                        >
                          <Archive className="h-4 w-4" />
                        </button>
                      : <button
                          onClick={() => handleRestore(workspace)}
                          className="p-2 rounded-lg text-emerald-600 hover:bg-emerald-500/10 transition-all"
                          title="Restore Workspace"
                        >
                          <RotateCcw className="h-4 w-4" />
                        </button>
                    }
                  </div>
                  <button
                    onClick={() => handleOpenWorkspace(workspace)}
                    className="flex items-center gap-1 text-sm font-bold text-primary hover:gap-2 transition-all"
                  >
                    Open
                    <ArrowRight className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      {/* Edit Workspace Modal */}
      {editingWorkspace && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
          tabIndex={-1}
          onMouseDown={e => {
            if (e.target === e.currentTarget) closeEditDialog();
          }}
          onKeyDown={e => {
            if (e.key === "Escape") closeEditDialog();
          }}
        >
          <div
            className="bg-background rounded-2xl shadow-2xl p-8 w-[95vw] max-w-md border border-border"
            onMouseDown={e => e.stopPropagation()}
          >
            <h2 className="text-xl font-bold mb-2">Edit Workspace</h2>
            <p className="text-muted-foreground mb-6 text-sm">Update the name or description for this workspace.</p>
            <input
              ref={editInputRef}
              type="text"
              placeholder="Workspace name"
              value={editName}
              onChange={e => setEditName(e.target.value)}
              onKeyDown={e => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (!isUpdating && editName.trim() && hasEditChanges) handleUpdateWorkspace();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  closeEditDialog();
                }
              }}
              disabled={isUpdating}
              className="w-full h-12 px-4 mb-4 border border-border rounded-lg bg-background-secondary focus:outline-none focus:ring-2 focus:ring-primary text-foreground font-medium"
            />
            <textarea
              placeholder="Description (optional)"
              value={editDescription}
              onChange={e => setEditDescription(e.target.value)}
              disabled={isUpdating}
              rows={3}
              className="w-full px-4 py-3 mb-6 border border-border rounded-lg bg-background-secondary focus:outline-none focus:ring-2 focus:ring-primary text-foreground font-medium resize-none"
            />
            {updateError && (
              <div className="mb-4 bg-destructive/10 border border-destructive/30 rounded-lg px-4 py-3 text-destructive font-bold text-sm text-center">
                {updateError}
              </div>
            )}
            <div className="flex justify-end gap-3">
              <button
                className="px-5 py-2 rounded-lg font-bold bg-background border border-border text-foreground hover:bg-background-tertiary transition-all"
                onClick={closeEditDialog}
                disabled={isUpdating}
                type="button"
              >
                Cancel
              </button>
              <button
                className="px-5 py-2 rounded-lg font-bold bg-primary text-primary-foreground shadow-md hover:scale-[1.03] active:scale-[0.98] transition-all disabled:opacity-60"
                onClick={handleUpdateWorkspace}
                disabled={isUpdating || !editName.trim() || !hasEditChanges}
                type="button"
              >
                {isUpdating ? "Saving…" : "Save"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Create Workspace Modal */}
      {showCreateDialog && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
          tabIndex={-1}
          onMouseDown={e => {
            if (e.target === e.currentTarget) closeCreateDialog();
          }}
          onKeyDown={e => {
            if (e.key === "Escape") closeCreateDialog();
          }}
        >
          <div
            className="bg-background rounded-2xl shadow-2xl p-8 w-[95vw] max-w-md border border-border"
            onMouseDown={e => e.stopPropagation()}
          >
            <h2 className="text-xl font-bold mb-2">Create Workspace</h2>
            <p className="text-muted-foreground mb-6 text-sm">Enter a name for your new workspace.</p>
            <input
              ref={inputRef}
              type="text"
              placeholder="Workspace name"
              value={workspaceName}
              onChange={e => setWorkspaceName(e.target.value)}
              onKeyDown={e => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (!isCreating && workspaceName.trim()) handleCreateWorkspace();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  closeCreateDialog();
                }
              }}
              disabled={isCreating}
              className="w-full h-12 px-4 mb-6 border border-border rounded-lg bg-background-secondary focus:outline-none focus:ring-2 focus:ring-primary text-foreground font-medium"
            />
            <div className="flex justify-end gap-3">
              <button
                className="px-5 py-2 rounded-lg font-bold bg-background border border-border text-foreground hover:bg-background-tertiary transition-all"
                onClick={closeCreateDialog}
                disabled={isCreating}
                type="button"
              >
                Cancel
              </button>
              <button
                className="px-5 py-2 rounded-lg font-bold bg-primary text-primary-foreground shadow-md hover:scale-[1.03] active:scale-[0.98] transition-all disabled:opacity-60"
                onClick={handleCreateWorkspace}
                disabled={isCreating || !workspaceName.trim()}
                type="button"
              >
                {isCreating ? "Creating…" : "Create"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
