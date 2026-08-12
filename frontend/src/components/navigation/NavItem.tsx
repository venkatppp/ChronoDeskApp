import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/utils/cn";

interface NavItemProps {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}

/**
 * Sidebar navigation row — the rail's own raised-material states:
 * quiet by default, subtle raised hover, blue-tinted raised active,
 * slight press-down. Icons keep one optical weight (16px / stroke 1.75).
 */
export function NavItem({ to, label, icon: Icon, end }: NavItemProps) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) =>
        cn(
          "group relative flex h-8 items-center gap-2.5 rounded-[8px] px-2.5 text-[13px] font-medium",
          "transition-all duration-150 ease-[var(--ease-premium)] active:scale-[0.985]",
          isActive
            ? "bg-(--color-surface-raised)/70 text-(--color-foreground) shadow-[inset_0_1px_0_rgba(255,255,255,0.14),inset_0_0_0_1px_rgba(255,255,255,0.08)]"
            : "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground) hover:shadow-[inset_0_1px_0_rgba(255,255,255,0.06)]",
        )
      }
    >
      {({ isActive }) => (
        <>
          {/* Restrained blue indicator for the active scope. */}
          {isActive && (
            <span className="absolute -left-1.5 h-4 w-[3px] rounded-full bg-(--color-accent)" aria-hidden="true" />
          )}
          <Icon
            className={cn("h-4 w-4 shrink-0", isActive ? "text-(--color-accent)" : "text-(--color-faint-foreground)")}
            strokeWidth={1.75}
          />
          <span className="truncate">{label}</span>
        </>
      )}
    </NavLink>
  );
}
