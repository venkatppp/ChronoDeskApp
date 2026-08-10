import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/utils/cn";

export interface GlassInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Optional leading icon, anchored inside the field. */
  icon?: React.ReactNode;
  /** Larger macOS-style search field variant. */
  size?: "sm" | "md" | "lg";
}

/** Shared glass text field — one input vocabulary for the whole app. */
export const GlassInput = forwardRef<HTMLInputElement, GlassInputProps>(
  ({ className, icon, size = "md", ...props }, ref) => (
    <span className={cn("relative block w-full", className)}>
      {icon && (
        <span className="pointer-events-none absolute inset-y-0 left-3.5 flex items-center text-(--color-faint-foreground)">
          {icon}
        </span>
      )}
      <input
        ref={ref}
        className={cn(
          "glass-well w-full rounded-[var(--radius-control)] text-(--color-foreground)",
          "placeholder:text-(--color-faint-foreground) transition-all duration-200 ease-[var(--ease-premium)]",
          "focus:shadow-[inset_0_1px_2px_rgba(0,0,0,0.25),0_0_0_1px_rgba(10,132,255,0.5)] focus:outline-none",
          size === "sm" && "h-7 px-3 text-xs",
          size === "md" && "h-9 px-3.5 text-sm",
          size === "lg" && "h-11 px-4 text-[15px]",
          icon && (size === "lg" ? "pl-11" : "pl-10"),
          props.disabled && "pointer-events-none opacity-50",
        )}
        {...props}
      />
    </span>
  ),
);
GlassInput.displayName = "GlassInput";
