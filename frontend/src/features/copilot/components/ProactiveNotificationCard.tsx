// ProactiveNotificationCard - Smart AI notification with evidence

import { useState } from "react";
import { X, ChevronDown, ChevronUp, AlertCircle, Info, AlertTriangle, Zap } from "lucide-react";
import { cn } from "@/utils/cn";
import type { ProactiveNotification, NotificationPriority } from "@/types/proactive";

interface ProactiveNotificationCardProps {
  notification: ProactiveNotification;
  onDismiss: (id: string) => void;
  onActionClick: (action: string) => void;
}

const PRIORITY_CONFIG: Record<
  NotificationPriority,
  { icon: typeof AlertCircle; color: string; bgColor: string }
> = {
  low: {
    icon: Info,
    color: "text-(--color-muted-foreground)",
    bgColor: "bg-(--color-surface-raised)",
  },
  medium: {
    icon: Zap,
    color: "text-(--color-accent)",
    bgColor: "bg-(--color-accent-muted)",
  },
  high: {
    icon: AlertTriangle,
    color: "text-(--color-warning)",
    bgColor: "bg-(--color-warning)/10",
  },
  critical: {
    icon: AlertCircle,
    color: "text-(--color-danger)",
    bgColor: "bg-(--color-danger)/10",
  },
};

export function ProactiveNotificationCard({
  notification,
  onDismiss,
  onActionClick,
}: ProactiveNotificationCardProps) {
  const [showEvidence, setShowEvidence] = useState(false);
  const config = PRIORITY_CONFIG[notification.priority];
  const Icon = config.icon;

  return (
    <div
      className={cn(
        "rounded-lg border border-(--color-border) p-4 transition-all",
        config.bgColor
      )}
    >
      <div className="flex items-start gap-3">
        <Icon className={cn("mt-0.5 h-5 w-5 shrink-0", config.color)} />
        
        <div className="flex-1">
          <div className="flex items-start justify-between gap-2">
            <h3 className="font-semibold text-(--color-foreground)">{notification.title}</h3>
            {notification.dismissible && (
              <button
                onClick={() => onDismiss(notification.id)}
                className="rounded p-1 text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                title="Dismiss"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>

          <p className="mt-1 text-sm text-(--color-muted-foreground)">{notification.message}</p>

          {/* Suggested Actions */}
          {notification.suggested_actions.length > 0 && (
            <div className="mt-3 flex flex-wrap gap-2">
              {notification.suggested_actions.map((action, idx) => (
                <button
                  key={idx}
                  onClick={() => onActionClick(action)}
                  className="rounded-md border border-(--color-border) bg-(--color-surface) px-3 py-1.5 text-sm text-(--color-foreground) transition-colors hover:bg-(--color-surface-hover)"
                >
                  {action}
                </button>
              ))}
            </div>
          )}

          {/* Evidence Toggle */}
          {notification.evidence.length > 0 && (
            <div className="mt-3">
              <button
                onClick={() => setShowEvidence(!showEvidence)}
                className="flex items-center gap-1.5 text-xs text-(--color-muted-foreground) hover:text-(--color-foreground)"
              >
                {showEvidence ? (
                  <ChevronUp className="h-3.5 w-3.5" />
                ) : (
                  <ChevronDown className="h-3.5 w-3.5" />
                )}
                <span>
                  Evidence ({notification.evidence.length}) · Avg Confidence:{" "}
                  {Math.round(
                    notification.evidence.reduce((sum, e) => sum + e.confidence, 0) /
                      notification.evidence.length *
                      100
                  )}
                  %
                </span>
              </button>

              {showEvidence && (
                <div className="mt-2 space-y-2">
                  {notification.evidence.map((evidence, idx) => (
                    <div
                      key={idx}
                      className="rounded-md border border-(--color-border-subtle) bg-(--color-surface) p-2 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-medium text-(--color-foreground)">
                          {evidence.source}
                        </span>
                        <span className="text-(--color-accent)">
                          {Math.round(evidence.confidence * 100)}%
                        </span>
                      </div>
                      <p className="mt-1 text-(--color-muted-foreground)">{evidence.description}</p>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
