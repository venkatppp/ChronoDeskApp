import { useState, useEffect, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Plus, Search, Folder, Calendar, Shield, Trash2, Archive, ArrowRight, Pencil, RotateCcw, AlertTriangle } from "lucide-react";
import { getWorkspaceRepository } from "@/services/workspaceRepository";
import type { UpdateWorkspaceInput, Workspace, WorkspaceStatus } from "@/types/workspace";
import { formatRelativeTime } from "@/utils/formatRelativeTime";
import { Button } from "@/components/ui/Button";
import { GlassInput } from "@/components/ui/GlassInput";
import { PageContainer } from "@/components/ui/PageContainer";
import { PageHeader } from "@/components/ui/PageHeader";
import { EmptyState } from "@/components/ui/EmptyState";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { ProgressRing } from "@/components/ui/ProgressRing";
import { Dialog } from "@/components/ui/Dialog";
import { detectLanguage } from "@/features/dashboard/components/WorkspaceCard";

const STATUS_STYLES: Record<WorkspaceStatus, { dot: string; label: string; tile: string }> = {
  active: {
    dot: "bg-(--color-success)",
    label: "text-(--color-success)",
    tile: "bg-(--color-surface-raised) text-(--color-muted-foreground)",
  },
  archived: {
    dot: "bg-(--color-faint-foreground)",
    label: "text-(--color-faint-foreground)",
    tile: "bg-(--color-surface-hover) text-(--color-muted-foreground)",
  },
};

function healthTone(score: number): { ring: string; label: string } {
  if (score > 80) return { ring: "text-(--color-success)", label: "Healthy" };
  if (score > 40) return { ring: "text-(--color-warning)", label: "Needs attention" };
  return { ring: "text-(--color-danger)", label: "At risk" };
}

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
      <PageContainer>
        <PageHeader
          eyebrow="Projects"
          title="Workspaces"
          description="Manage your project environments and watched folders."
          actions={
            <Button onClick={() => setShowCreateDialog(true)} disabled={isCreating}>
              <Plus className="h-4 w-4" strokeWidth={1.75} />
              New workspace
            </Button>
          }
        />

        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <GlassInput
            className="flex-1 md:max-w-md"
            size="md"
            icon={<Search className="h-4 w-4" strokeWidth={1.75} />}
            type="text"
            aria-label="Search workspaces by name or description"
            placeholder="Search by name or description..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          <SegmentedControl
            ariaLabel="Filter workspaces by status"
            value={statusFilter}
            onChange={(value) => setStatusFilter(value)}
            options={[
              { value: "all", label: "All" },
              { value: "active", label: "Active" },
              { value: "archived", label: "Archived" },
            ]}
          />
        </div>

        {createError && (
          <div className="rounded-[var(--radius-card)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-4 py-3 text-sm font-medium text-(--color-danger)">
            {createError}
          </div>
        )}
        {isLoading ? (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {[...Array(8)].map((_, i) => (
              <div key={i} className="h-56 animate-pulse rounded-[var(--radius-card)] border border-(--color-border-subtle) bg-(--color-surface)" />
            ))}
          </div>
        ) : error ? (
          <div className="flex flex-col items-center gap-4 rounded-[var(--radius-card)] border border-(--color-danger)/20 bg-(--color-danger)/5 px-6 py-16 text-center">
            <AlertTriangle className="h-8 w-8 text-(--color-danger)" strokeWidth={1.5} />
            <h3 className="font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">{error}</h3>
            <Button variant="outline" size="sm" onClick={fetchWorkspaces}>Try again</Button>
          </div>
        ) : filteredWorkspaces.length === 0 ? (
          <EmptyState
            icon={<Folder className="h-4 w-4" strokeWidth={1.75} />}
            title={searchQuery ? "No workspace that matches" : "No workspaces yet"}
            description={
              searchQuery
                ? "Try adjusting your search or filters."
                : "Start by creating your first workspace to organize your files. Workspaces keep your watched folders, timeline, and graph scoped per project."
            }
            primaryAction={
              !searchQuery ? (
                <Button onClick={() => setShowCreateDialog(true)}>
                  <Plus className="h-4 w-4" strokeWidth={1.75} />
                  Create the first workspace
                </Button>
              ) : undefined
            }
          />
        ) : (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {filteredWorkspaces.map((workspace) => {
              const tone = healthTone(workspace.healthScore);
              const statusStyle = STATUS_STYLES[workspace.status];
              const lang = detectLanguage(workspace.rootPath);
              return (
                <div
                  key={workspace.id}
                  className="glass-panel group relative flex flex-col overflow-hidden rounded-[var(--radius-card)] p-5 transition-all duration-300 ease-[var(--ease-premium)] hover:-translate-y-0.5 hover:shadow-[var(--shadow-float)]"
                >
                  <div className="mb-4 flex items-start justify-between">
                    <div className={`flex h-10 w-10 items-center justify-center rounded-xl ${statusStyle.tile}`}>
                      <Folder className="h-5 w-5" strokeWidth={1.75} />
                    </div>
                    <div className="flex items-center gap-1.5">
                      <span className={`h-1.5 w-1.5 rounded-full ${statusStyle.dot}`} />
                      <span className={`text-[10px] font-semibold uppercase tracking-wider ${statusStyle.label}`}>
                        {workspace.status}
                      </span>
                    </div>
                  </div>

                  <h3 className="truncate font-(family-name:--font-display) text-lg font-semibold text-(--color-foreground)">
                    {workspace.name}
                  </h3>
                  <p className="mb-5 mt-1 line-clamp-2 flex-1 text-[13px] leading-relaxed text-(--color-muted-foreground)">
                    {workspace.description || "No description provided."}
                  </p>

                  <div className="mb-5 space-y-2">
                    <div className="flex items-center gap-2 text-xs text-(--color-muted-foreground)">
                      <Calendar className="h-3.5 w-3.5 shrink-0 text-(--color-faint-foreground)" strokeWidth={1.75} />
                      <span>Last active {formatRelativeTime(workspace.lastActiveAt)}</span>
                    </div>
                    {workspace.rootPath && (
                      <div className="flex items-center gap-2 text-xs text-(--color-muted-foreground)">
                        <Shield className="h-3.5 w-3.5 shrink-0 text-(--color-faint-foreground)" strokeWidth={1.75} />
                        <span className="truncate font-(family-name:--font-mono) text-[11px]">{workspace.rootPath}</span>
                        {lang && (
                          <span
                            className="shrink-0 rounded-md border border-(--color-border-subtle) bg-(--color-surface-hover) px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
                            style={lang.style}
                          >
                            {lang.label}
                          </span>
                        )}
                      </div>
                    )}
                    <div className="flex items-center justify-between gap-2">
                      <div className="flex items-center gap-2.5">
                        <ProgressRing value={workspace.healthScore} size={36} strokeWidth={3} />
                        <span className={`text-[11px] font-semibold uppercase tracking-wider ${tone.ring}`}>
                          {tone.label}
                        </span>
                      </div>
                      <span className="text-xs tabular-nums text-(--color-faint-foreground)">
                        {workspace.healthScore} / 100
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center justify-between border-t border-(--color-border-subtle) pt-3.5">
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => openEditDialog(workspace)}
                        className="rounded-lg p-2 text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                        title="Edit Workspace"
                        aria-label="Edit Workspace"
                      >
                        <Pencil className="h-4 w-4" strokeWidth={1.75} />
                      </button>
                      <button
                        onClick={() => handleDelete(workspace.id)}
                        className="rounded-lg p-2 text-(--color-faint-foreground) transition-colors hover:bg-(--color-danger)/10 hover:text-(--color-danger)"
                        title="Delete Workspace"
                        aria-label="Delete Workspace"
                      >
                        <Trash2 className="h-4 w-4" strokeWidth={1.75} />
                      </button>
                      {workspace.status === "active"
                        ? (
                          <button
                            onClick={() => handleArchive(workspace)}
                            className="rounded-lg p-2 text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                            title="Archive Workspace"
                            aria-label="Archive Workspace"
                          >
                            <Archive className="h-4 w-4" strokeWidth={1.75} />
                          </button>
                        )
                        : (
                          <button
                            onClick={() => handleRestore(workspace)}
                            className="rounded-lg p-2 text-(--color-faint-foreground) transition-colors hover:bg-(--color-success)/10 hover:text-(--color-success)"
                            title="Restore Workspace"
                            aria-label="Restore Workspace"
                          >
                            <RotateCcw className="h-4 w-4" strokeWidth={1.75} />
                          </button>
                        )}
                    </div>
                    <button
                      onClick={() => handleOpenWorkspace(workspace)}
                      className="group/open flex items-center gap-1.5 text-sm font-semibold text-(--color-foreground) transition-all duration-200 hover:gap-2.5 hover:text-(--color-accent)"
                    >
                      Open
                      <ArrowRight className="h-4 w-4 transition-transform duration-200 group-hover/open:translate-x-0.5" strokeWidth={1.75} />
                    </button>
                  </div>
                </div>
              );
            })}
        </div>
      )}
      </PageContainer>
      {/* Edit Workspace Modal */}
      <Dialog
        open={editingWorkspace !== null}
        onClose={closeEditDialog}
        title="Edit Workspace"
        description="Update the name or description for this workspace."
        footer={
          <>
            <Button variant="ghost" onClick={closeEditDialog} disabled={isUpdating} type="button">
              Cancel
            </Button>
            <Button onClick={handleUpdateWorkspace} disabled={isUpdating || !editName.trim() || !hasEditChanges} type="button">
              {isUpdating ? "Saving…" : "Save"}
            </Button>
          </>
        }
      >
        <GlassInput
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
          className="mb-3"
        />
        <textarea
          placeholder="Description (optional)"
          value={editDescription}
          onChange={e => setEditDescription(e.target.value)}
          disabled={isUpdating}
          rows={3}
          className="mb-5 w-full resize-none rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface-raised) px-3.5 py-2.5 text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:border-(--color-accent)/60 focus:outline-none focus:ring-2 focus:ring-(--color-accent)/15"
        />
        {updateError && (
          <div className="mb-4 rounded-[var(--radius-control)] border border-(--color-danger)/30 bg-(--color-danger)/10 px-4 py-2.5 text-sm font-medium text-(--color-danger)">
            {updateError}
          </div>
        )}
      </Dialog>

      {/* Create Workspace Modal */}
      <Dialog
        open={showCreateDialog}
        onClose={closeCreateDialog}
        title="Create Workspace"
        description="Enter a name for your new workspace."
        footer={
          <>
            <Button variant="ghost" onClick={closeCreateDialog} disabled={isCreating} type="button">
              Cancel
            </Button>
            <Button onClick={handleCreateWorkspace} disabled={isCreating || !workspaceName.trim()} type="button">
              {isCreating ? "Creating…" : "Create"}
            </Button>
          </>
        }
      >
        <GlassInput
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
          className="mb-1"
        />
      </Dialog>
    </>
  );
}
