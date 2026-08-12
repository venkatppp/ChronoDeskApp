// LineageExplorerCard - RC-6 M4: explore the evolution of a memory —
// version ancestry, descendants, merges — and its export/import actions.

import { useCallback, useState } from "react";
import { Download, GitBranch, Search, Upload } from "lucide-react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { memoryRepository } from "@/services/memoryRepository";
import type { LineageNode, MemoryLineage } from "@/types/memory";

function statusTone(status: string): "neutral" | "accent" | "warning" | "success" {
  if (status === "success") return "success";
  if (status === "failed") return "warning";
  return "neutral";
}

function NodeRow({ node, role }: { node: LineageNode; role: string }) {
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] bg-(--color-surface) px-3 py-1.5">
      <span className="w-16 shrink-0 text-[10px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
        {role}
      </span>
      <p className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">{node.goal}</p>
      <Badge variant={statusTone(node.status)}>{node.status}</Badge>
      <span className="text-[10px] text-(--color-faint-foreground)">v{node.version}</span>
      <span className="font-mono text-[10px] text-(--color-faint-foreground)">{node.id.slice(0, 8)}</span>
    </div>
  );
}

export function LineageExplorerCard() {
  const [memoryId, setMemoryId] = useState("");
  const [lineage, setLineage] = useState<MemoryLineage | null>(null);
  const [notFound, setNotFound] = useState(false);
  const [exportText, setExportText] = useState<string | null>(null);
  const [importText, setImportText] = useState("");
  const [importNotice, setImportNotice] = useState<string | null>(null);

  const explore = useCallback(async () => {
    const id = memoryId.trim();
    if (!id) return;
    setNotFound(false);
    setLineage(null);
    try {
      const result = await memoryRepository.lineage(id);
      if (!result) {
        setNotFound(true);
        return;
      }
      setLineage(result);
    } catch (err) {
      console.error("Lineage lookup failed:", err);
    }
  }, [memoryId]);

  const exportStore = async () => {
    try {
      setExportText(await memoryRepository.exportJson());
    } catch (err) {
      console.error("Export failed:", err);
    }
  };

  const importStore = async () => {
    if (!importText.trim()) return;
    try {
      const result = await memoryRepository.importJson(importText);
      setImportNotice(`Imported ${result.imported} memories, skipped ${result.skipped}.`);
      setImportText("");
    } catch (err) {
      setImportNotice(`Import failed: ${String(err)}`);
    }
  };

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <GitBranch className="h-4 w-4 text-(--color-muted-foreground)" strokeWidth={1.75} />
        <h2 className="text-sm font-medium text-(--color-foreground)">Lineage explorer</h2>
      </div>

      <div className="mt-3 flex gap-2">
        <input
          value={memoryId}
          onChange={(e) => setMemoryId(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && explore()}
          placeholder="Memory id to explore"
          className="glass-well min-w-0 h-8 flex-1 rounded-[var(--radius-control)] px-3 text-xs text-(--color-foreground) placeholder:text-(--color-faint-foreground) transition-all duration-200 ease-[var(--ease-premium)] focus-well"
        />
        <Button size="sm" onClick={() => void explore()}>
          <Search className="h-3.5 w-3.5" strokeWidth={1.75} /> Explore
        </Button>
      </div>

      {notFound && (
        <p className="mt-2 text-xs text-(--color-danger)">No memory with that id.</p>
      )}

      {lineage && (
        <div className="mt-3 space-y-2">
          <div className="space-y-1">
            {lineage.ancestors.map((node) => (
              <NodeRow key={node.id} node={node} role={node.relation ?? "parent"} />
            ))}
            <div className="flex flex-wrap items-center gap-2 px-1 py-1">
              <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-(--color-accent)" aria-hidden="true" />
              <span className="w-16 shrink-0 text-[10px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                current
              </span>
              <p className="min-w-0 flex-1 truncate text-xs font-medium text-(--color-foreground)">
                {lineage.version === 1 ? "root workflow" : `version ${lineage.version}`}
              </p>
              <span className="font-mono text-[10px] text-(--color-faint-foreground)">
                {lineage.memory_id.slice(0, 8)}
              </span>
            </div>
            {lineage.children.map((node) => (
              <NodeRow key={node.id} node={node} role={node.relation ?? "child"} />
            ))}
          </div>

          {lineage.merged_into.length > 0 && (
            <div>
              <p className="mb-1 text-[10px] font-medium uppercase tracking-wide text-(--color-faint-foreground)">
                Merged into this memory
              </p>
              {lineage.merged_into.map((node) => (
                <NodeRow key={node.id} node={node} role="merged" />
              ))}
            </div>
          )}
          {lineage.merged_into_id && (
            <p className="text-[11px] text-(--color-muted-foreground)">
              This memory was merged into{" "}
              <span className="font-mono">{lineage.merged_into_id.slice(0, 8)}</span> as a duplicate.
            </p>
          )}
        </div>
      )}

      <div className="mt-4 border-t border-(--color-border) pt-3">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="secondary" size="sm" onClick={() => void exportStore()}>
            <Download className="h-3.5 w-3.5" strokeWidth={1.75} /> Export JSON
          </Button>
          <input
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
            placeholder="Paste export JSON to import"
            className="glass-well min-w-0 h-8 flex-1 rounded-[var(--radius-control)] px-3 text-xs text-(--color-foreground) placeholder:text-(--color-faint-foreground) transition-all duration-200 ease-[var(--ease-premium)] focus-well"
          />
          <Button size="sm" variant="secondary" onClick={() => void importStore()} disabled={!importText.trim()}>
            <Upload className="h-3.5 w-3.5" strokeWidth={1.75} /> Import
          </Button>
        </div>
        {importNotice && (
          <p className="mt-2 text-[11px] text-(--color-muted-foreground)">{importNotice}</p>
        )}
        {exportText && (
          <details className="mt-2">
            <summary className="cursor-pointer text-[11px] text-(--color-muted-foreground)">
              Exported JSON ({exportText.length.toLocaleString()} chars)
            </summary>
            <textarea
              readOnly
              value={exportText}
              className="mt-2 h-40 w-full resize-y rounded-[var(--radius-control)] border border-(--color-border) bg-(--color-surface) p-2 font-mono text-[10px] text-(--color-foreground)"
            />
          </details>
        )}
      </div>
    </Card>
  );
}
