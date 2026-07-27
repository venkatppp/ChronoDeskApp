import { BarChart3 } from "lucide-react";
import { ComingSoon } from "@/components/ui/ComingSoon";

export function AnalyticsPage() {
  return (
    <ComingSoon
      icon={BarChart3}
      title="Analytics"
      description="Focus time, interruptions, and workspace-score charts ship once the Analytics Engine is producing real Timeline aggregates."
      phase="Phase 5 · Analytics Engine"
    />
  );
}
