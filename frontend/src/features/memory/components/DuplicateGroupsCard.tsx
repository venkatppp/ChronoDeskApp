// DuplicateGroupsCard - RC-6 M3: identical memories detected in the
// store, with a merge action that keeps the best record of each group.

import { useState } from "react";
import { Combine, Copy } from "lucide-react";
import { Card } from "@/components/ui/Card";
import { memoryRepository } from "@/services/memoryRepository";
import type { DuplicateGroup } from "@/types/memory";

interface DuplicateGroupsCardProps {
  groups: DuplicateGroup[];
  onMerged: (merged: number) => void;
}

export function DuplicateGroupsCard({ groups, onMerged }: DuplicateGroupsCardProps) {
  const [merging, setMerging] = useState(false);

  if (groups.length === 0) return null;

  const runMerge = async () => {
    if (merging) return;
    setMerging(true);
    try {
      const result = await memoryRepository.mergeDuplicates();
      onMerged(result.records_merged);
    } catch (err) {
      console.error("Duplicate merge failed:", err);
    } finally {
      setMerging(false);
    }
  };

  return (
    <Card className="p-4">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <Copy className="h-4 w-4 text-(--color-accent)" />
          <h2 className="text-sm font-medium text-(--color-foreground)">Duplicate memories</h2>
        </div>
        <button
          onClick={runMerge}
          disabled={merging}
          className="flex items-center gap-1.5 rounded-md border border-(--color-border) px-3 py-1.5 text-xs font-medium text-(--color-foreground) transition-opacity hover:opacity-80 disabled:opacity-50"
        >
          <Combine className={merging ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />
          {merging ? "Merging…" : "Merge duplicates"}
        </button>
      </div>

      <div className="mt-3 space-y-2">
        {groups.map((group) => (
          <div
            key={group.goal_fingerprint}
            className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3"
          >
            <div className="flex items-center justify-between gap-2">
              <p className="truncate text-sm font-medium text-(--color-foreground)">
                {group.records[0]?.goal ?? group.goal_fingerprint}
              </p>
              <span className="shrink-0 text-[11px] text-(--color-faint-foreground)">
                {group.records.length} identical run(s)
              </span>
            </div>
            <p className="mt-1 text-[11px] text-(--color-muted-foreground)">{group.reason}</p>
          </div>
        ))}
      </div>
    </Card>
  );
}
