// ActivitySummary Component
//
// Displays a summary of activity for a time range.

import { Clock, Files, Edit, GitCommit, Code } from "lucide-react";
import { Card } from "@/components/ui/Card";
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
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        {stats.map((stat, i) => (
          <Card key={i} className="flex items-center gap-3 p-3">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
              <div className="text-(--color-accent)">{stat.icon}</div>
            </div>
            <div className="min-w-0">
              <p className="text-xs text-(--color-faint-foreground)">{stat.label}</p>
              <p className="text-base font-bold text-(--color-foreground)">{stat.value}</p>
            </div>
          </Card>
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
