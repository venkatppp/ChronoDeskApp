/**
 * Formats an ISO-8601 timestamp as a short, human-readable relative string
 * (e.g. "6h ago", "4d ago"). Used throughout the dashboard and timeline so
 * every screen reports recency the same way.
 */
export function formatRelativeTime(iso: string): string {
  const deltaMs = Date.now() - new Date(iso).getTime();
  const minutes = Math.round(deltaMs / 60_000);

  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.round(hours / 24);
  return `${days}d ago`;
}
