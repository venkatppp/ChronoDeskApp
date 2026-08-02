// ExecutionDigest - compact budget / progress counters for an autonomous session.

interface ExecutionDigestProps {
  plansAttempted: number;
  plansCompleted: number;
  stepsCompleted: number;
  stepsLeft: number;
  retriesUsed: number;
  replansUsed: number;
}

export function ExecutionDigest({
  plansAttempted,
  plansCompleted,
  stepsCompleted,
  stepsLeft,
  retriesUsed,
  replansUsed,
}: ExecutionDigestProps) {
  return (
    <div className="grid gap-2 sm:grid-cols-3">
      <Stat label="Plans" value={plansCompleted} total={plansAttempted} />
      <Stat label="Steps" value={stepsCompleted} total={stepsLeft > 0 ? stepsCompleted + stepsLeft : undefined} />
      <Stat label="Retries" value={retriesUsed} />
      <Stat label="Replans" value={replansUsed} />
    </div>
  );
}

function Stat({ label, value, total }: { label: string; value: number; total?: number }) {
  return (
    <div className="rounded-lg border border-(--color-border) bg-(--color-surface-raised) p-3">
      <p className="text-xs text-(--color-muted-foreground)">{label}</p>
      <p className="text-xl font-mono font-medium tabular-nums text-(--color-foreground)">
        {total !== undefined ? `${value} / ${total}` : value}
      </p>
    </div>
  );
}