import { Sparkles } from "lucide-react";
import { Card } from "@/components/ui/Card";
import { cn } from "@/utils/cn";

interface BriefingBannerProps {
  briefing: string | null;
  isLoading: boolean;
}

export function BriefingBanner({ briefing, isLoading }: BriefingBannerProps) {
  return (
    <Card className="flex items-start gap-3 border-(--color-accent-muted) bg-(--color-surface-raised) p-4">
      <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-(--color-accent-muted)">
        <Sparkles className="h-4 w-4 text-(--color-accent)" strokeWidth={1.75} />
      </div>
      <div className="min-w-0">
        <p className="text-xs font-medium uppercase tracking-wide text-(--color-faint-foreground)">
          Today&apos;s briefing
        </p>
        <p
          className={cn(
            "mt-1 text-sm leading-relaxed text-(--color-foreground)",
            isLoading && "animate-pulse text-(--color-muted-foreground)",
          )}
        >
          {isLoading ? "Reviewing your active workspaces…" : briefing}
        </p>
      </div>
    </Card>
  );
}
