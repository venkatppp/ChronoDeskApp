// ActivitySummary Component
//
// Displays a summary of activity for a time range.

import { Clock, Files, Edit, GitCommit, Code } from "lucide-react";
import type { ActivitySummary as ActivitySummaryType } from "@/types/analytics";

interface ActivitySummaryProps {
  summary: ActivitySummaryType;
}

export function ActivitySummary({ summary }: ActivitySummaryProps) {
  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  };

  const stats = [
    {
      icon: <Clock className="h-4 w-4" strokeWidth={1.75} />,
      label: "Duration",
      value: formatDuration(summary.durationSeconds),
    },
    {
      icon: <Files className="h-4 w-4" strokeWidth={1.75} />,
      label: "Files",
      value: summary.fileCount.toString(),
    },
    {
      icon: <Edit className="h-4 w-4" strokeWidth={1.75} />,
      label: "Edits",
      value: summary.editCount.toString(),
    },
    {
      icon: <GitCommit className="h-4 w-4" strokeWidth={1.75} />,
      label: "Commits",
      value: summary.commitCount.toString(),
    },
  ];

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-4">
        {stats.map((stat, i) => (
          <div key={i} className="flex items-center gap-3">
            <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[var(--radius-control)] bg-(--color-surface-raised)">
              <span className="text-(--color-muted-foreground)">{stat.icon}</span>
            </span>
            <div className="min-w-0">
              <p className="text-[11px] text-(--color-faint-foreground)">{stat.label}</p>
              <p className="text-base font-bold text-(--color-foreground)">{stat.value}</p>
            </div>
          </div>
        ))}
      </div>

      {summary.primaryLanguage && (
        <div className="flex items-center gap-2 text-sm text-(--color-muted-foreground)">
          <Code className="h-4 w-4" strokeWidth={1.75} />
          <span>Primary language: {summary.primaryLanguage}</span>
        </div>
      )}
    </div>
  );
}
