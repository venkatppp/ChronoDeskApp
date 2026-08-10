// SnapshotManagerCard - RC-6 M4: periodic memory snapshots — create them
// on demand, list what is stored, and restore the full store from one.

import { useCallback, useEffect, useState } from "react";
import { Camera, History, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { memoryRepository } from "@/services/memoryRepository";
import type { MemorySnapshot, RestoreResult } from "@/types/memory";

export function SnapshotManagerCard() {
  const [snapshots, setSnapshots] = useState<MemorySnapshot[]>([]);
  const [label, setLabel] = useState("");
  const [creating, setCreating] = useState(false);
  const [restoringId, setRestoringId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const rows = await memoryRepository.snapshotList();
      setSnapshots(rows ?? []);
    } catch (err) {
      console.error("Failed to load snapshots:", err);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const create = async () => {
    setCreating(true);
    setNotice(null);
    try {
      const snapshot = await memoryRepository.snapshotCreate(label.trim() || undefined);
      setNotice(`Snapshot "${snapshot.label}" captured (${snapshot.record_count} memories).`);
      setLabel("");
      await load();
    } catch (err) {
      console.error("Snapshot creation failed:", err);
    } finally {
      setCreating(false);
    }
  };

  const restore = async (snapshot: MemorySnapshot) => {
    const confirmed = window.confirm(
      `Restore the memory store from "${snapshot.label}"? Current memories are replaced.`
    );
    if (!confirmed) return;
    setRestoringId(snapshot.id);
    setNotice(null);
    try {
      const result: RestoreResult = await memoryRepository.snapshotRestore(snapshot.id);
      setNotice(
        `Restored ${result.records_restored} memories (${result.acceptance_restored} feedback entries).`
      );
      await load();
    } catch (err) {
      console.error("Snapshot restore failed:", err);
    } finally {
      setRestoringId(null);
    }
  };

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <Camera className="h-4 w-4 text-(--color-muted-foreground)" />
        <h2 className="text-sm font-medium text-(--color-foreground)">Snapshots</h2>
      </div>

      <div className="mt-3 flex gap-2">
        <input
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && create()}
          placeholder="Label (optional, e.g. before refactor)"
          className="glass-well min-w-0 h-8 flex-1 rounded-[var(--radius-control)] px-3 text-xs text-(--color-foreground) placeholder:text-(--color-faint-foreground) transition-all duration-200 ease-[var(--ease-premium)] focus:shadow-[inset_0_1px_2px_rgba(0,0,0,0.25),0_0_0_1px_rgba(10,132,255,0.5)] focus:outline-none"
        />
        <Button size="sm" onClick={() => void create()} disabled={creating}>
          <Camera className="h-3.5 w-3.5" />
          {creating ? "Capturing…" : "Capture"}
        </Button>
      </div>

      {notice && (
        <p className="mt-2 text-[11px] text-(--color-muted-foreground)">{notice}</p>
      )}

      <div className="mt-3 space-y-1.5">
        {snapshots.length === 0 && (
          <p className="text-xs text-(--color-faint-foreground)">
            No snapshots yet — one is captured automatically every few hours.
          </p>
        )}
        {snapshots.map((snapshot) => (
          <div
            key={snapshot.id}
            className="flex flex-wrap items-center gap-2 rounded-[var(--radius-control)] bg-(--color-surface) px-3 py-2"
          >
            <History className="h-3.5 w-3.5 shrink-0 text-(--color-muted-foreground)" />
            <p className="min-w-0 flex-1 truncate text-xs text-(--color-foreground)">
              {snapshot.label}
            </p>
            <span className="text-[10px] text-(--color-faint-foreground)">
              {snapshot.record_count} memories · {new Date(snapshot.created_at).toLocaleString()}
            </span>
            <Button
              variant="secondary"
              size="sm"
              disabled={restoringId === snapshot.id}
              onClick={() => void restore(snapshot)}
            >
              <RotateCcw className="h-3 w-3" />
              {restoringId === snapshot.id ? "Restoring…" : "Restore"}
            </Button>
          </div>
        ))}
      </div>
    </Card>
  );
}
