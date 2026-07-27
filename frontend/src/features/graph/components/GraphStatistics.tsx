import type { GraphStats } from "@/types/graph";
import { Network, GitBranch, Weight, Activity, Grid3X3 } from "lucide-react";

interface GraphStatisticsProps {
  stats: GraphStats | null;
  isLoading: boolean;
}

export function GraphStatistics({ stats, isLoading }: GraphStatisticsProps) {
  if (isLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-5 gap-4 mb-6">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="h-24 bg-background-secondary border border-border rounded-2xl animate-pulse" />
        ))}
      </div>
    );
  }

  if (!stats) return null;

  const cards = [
    { label: "Nodes", value: stats.nodeCount, icon: Network, color: "text-blue-500" },
    { label: "Edges", value: stats.edgeCount, icon: GitBranch, color: "text-primary" },
    { label: "Avg Weight", value: stats.avgWeight.toFixed(2), icon: Weight, color: "text-amber-500" },
    { label: "Max Weight", value: stats.maxWeight.toFixed(2), icon: Activity, color: "text-emerald-500" },
    { label: "Density", value: (stats.density * 100).toFixed(1) + "%", icon: Grid3X3, color: "text-purple-500" },
  ];

  return (
    <div className="grid grid-cols-2 md:grid-cols-5 gap-4 mb-8">
      {cards.map((card) => (
        <div 
          key={card.label} 
          className="p-4 bg-background-secondary border border-border rounded-2xl flex flex-col justify-between hover:border-primary/30 transition-all hover:shadow-lg"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-widest">{card.label}</span>
            <card.icon className={`h-4 w-4 ${card.color}`} />
          </div>
          <div className="text-2xl font-bold text-foreground">{card.value}</div>
        </div>
      ))}
    </div>
  );
}
