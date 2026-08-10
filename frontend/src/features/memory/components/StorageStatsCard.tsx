// StorageStatsCard - RC-6 M4: how much space the memory system occupies
// (database file, vector index, embedding cache, snapshots) and how much
// of it sits in each retention policy.

import { HardDrive } from "lucide-react";
import { Card } from "@/components/ui/Card";
import type { MemoryStorageStats } from "@/types/memory";

interface StorageStatsCardProps {
  stats: MemoryStorageStats | null;
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const unit = Math.min(Math.floor(Math.log2(bytes) / 10), units.length - 1);
  const value = bytes / 2 ** (10 * unit);
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function StorageStatsCard({ stats }: StorageStatsCardProps) {
  if (!stats) return null;

  const tiles = [
    { label: "Database", value: formatBytes(stats.database_size_bytes) },
    { label: "Vector index", value: formatBytes(stats.vector_index_size_bytes) },
    {
      label: "Embedding cache",
      value: `${formatBytes(stats.cache_size_bytes)} · ${stats.cache_entries} entries`,
    },
    { label: "Snapshots", value: formatBytes(stats.snapshot_size_bytes) },
  ];

  const retention = [
    { label: "Permanent", count: stats.permanent_memories, tone: "text-(--color-success)" },
    { label: "Temporary", count: stats.temporary_memories, tone: "text-(--color-cyan)" },
    { label: "Archived", count: stats.archived_memories, tone: "text-(--color-warning)" },
    { label: "Expired", count: stats.expired_memories, tone: "text-(--color-danger)" },
  ];

  return (
    <Card className="p-4">
      <div className="flex items-center gap-2">
        <HardDrive className="h-4 w-4 text-(--color-muted-foreground)" />
        <h2 className="text-sm font-medium text-(--color-foreground)">Storage usage</h2>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-4">
        {tiles.map((tile) => (
          <div key={tile.label}>
            <p className="text-[11px] text-(--color-muted-foreground)">{tile.label}</p>
            <p className="mt-0.5 truncate font-mono text-sm font-medium text-(--color-foreground)">
              {tile.value}
            </p>
          </div>
        ))}
      </div>

      <div className="mt-3 flex flex-wrap gap-x-4 gap-y-1 text-xs">
        {retention.map((entry) => (
          <span key={entry.label} className="flex items-center gap-1.5">
            <span className={`font-medium ${entry.tone}`}>{entry.count}</span>
            <span className="text-(--color-muted-foreground)">{entry.label}</span>
          </span>
        ))}
        {stats.compressed_records > 0 && (
          <span className="text-(--color-faint-foreground)">
            {stats.compressed_records} compressed · {stats.compression_archive_count} originals
            preserved
          </span>
        )}
      </div>
    </Card>
  );
}
