import type { LucideIcon } from "lucide-react";

interface ComingSoonProps {
  icon: LucideIcon;
  title: string;
  description: string;
  phase: string;
}

/**
 * Placeholder shown for routes wired up in Phase 1 but whose feature
 * modules ship in a later phase. Keeps navigation fully functional
 * end-to-end while the corresponding `features/*` module is pending.
 */
export function ComingSoon({ icon: Icon, title, description, phase }: ComingSoonProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
      <div className="flex h-11 w-11 items-center justify-center rounded-xl border border-(--color-border-subtle) bg-(--color-surface-raised) text-(--color-muted-foreground)">
        <Icon className="h-5 w-5" strokeWidth={1.5} />
      </div>
      <h1 className="font-(family-name:--font-display) text-lg font-semibold">{title}</h1>
      <p className="max-w-sm text-sm text-(--color-muted-foreground)">{description}</p>
      <span className="mt-1 font-(family-name:--font-mono) text-xs text-(--color-faint-foreground)">{phase}</span>
    </div>
  );
}
