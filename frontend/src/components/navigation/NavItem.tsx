import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/utils/cn";

interface NavItemProps {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}

export function NavItem({ to, label, icon: Icon, end }: NavItemProps) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) =>
        cn(
          "group relative flex items-center gap-2.5 rounded-[var(--radius-control)] px-3 py-2 text-sm font-medium",
          "transition-colors duration-150",
          isActive
            ? "bg-(--color-surface-hover) text-(--color-foreground)"
            : "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)",
        )
      }
    >
      {({ isActive }) => (
        <>
          {/* Signature "time thread" active indicator instead of a plain highlight bar. */}
          <span
            className={cn(
              "absolute -left-2.5 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full transition-all duration-200",
              isActive ? "bg-(--color-accent) opacity-100" : "opacity-0",
            )}
          />
          <Icon className="h-4 w-4 shrink-0" strokeWidth={1.75} />
          <span className="truncate">{label}</span>
        </>
      )}
    </NavLink>
  );
}
