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
          "group relative flex items-center gap-2.5 rounded-[var(--radius-control)] px-3 py-2 text-[13px] font-medium",
          "transition-all duration-150 ease-[var(--ease-premium)]",
          isActive
            ? "bg-(--color-surface-raised) text-(--color-foreground) shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]"
            : "text-(--color-muted-foreground) hover:bg-(--color-surface-raised)/60 hover:text-(--color-foreground)",
        )
      }
    >
      {({ isActive }) => (
        <>
          <Icon className="h-4 w-4 shrink-0" strokeWidth={isActive ? 2 : 1.75} />
          <span className="truncate">{label}</span>
          {isActive && (
            <span className="absolute right-2 top-1/2 h-1.5 w-1.5 -translate-y-1/2 rounded-full bg-(--color-accent)" />
          )}
        </>
      )}
    </NavLink>
  );
}