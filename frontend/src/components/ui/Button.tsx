import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/utils/cn";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius-control)] text-sm font-medium " +
    "transition-all duration-150 ease-[var(--ease-premium)] disabled:pointer-events-none disabled:opacity-50 " +
    "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-accent) " +
    "active:scale-[0.98]",
  {
    variants: {
      variant: {
        primary:
          "bg-(--color-accent) text-(--color-accent-foreground) shadow-[0_1px_2px_rgba(0,0,0,0.4)] hover:brightness-110",
        secondary:
          "bg-(--color-surface-hover) text-(--color-foreground) border border-(--color-border) hover:bg-(--color-border-subtle)",
        ghost: "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)",
        danger: "bg-(--color-danger) text-white hover:brightness-110",
        outline: "border border-(--color-border) bg-transparent text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)",
      },
      size: {
        sm: "h-8 px-3 text-xs",
        md: "h-9 px-4",
        icon: "h-9 w-9 shrink-0",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  },
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => {
    return <button ref={ref} className={cn(buttonVariants({ variant, size }), className)} {...props} />;
  },
);
Button.displayName = "Button";
