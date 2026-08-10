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
        /* Illuminated glass — translucent blue pane with a bright
           specular top edge. Never a solid blue rectangle. */
        primary:
          "glass-accent text-(--color-accent-foreground) " +
          "hover:brightness-[1.06] active:brightness-[0.97]",
        /* Neutral glass — quiet frosted control, minimal border. */
        secondary:
          "glass-control text-(--color-foreground) " +
          "hover:bg-(--color-surface-raised)/60",
        ghost: "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)",
        danger:
          "bg-(--color-danger)/90 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.25),0_4px_16px_rgba(0,0,0,0.3)] hover:brightness-[1.06]",
        outline:
          "border border-(--color-border) bg-transparent text-(--color-muted-foreground) " +
          "hover:bg-(--color-surface-hover) hover:text-(--color-foreground) hover:border-(--color-border)",
      },
      size: {
        sm: "h-7 px-3 text-xs",
        md: "h-8.5 px-3.5",
        lg: "h-10 px-4 text-[15px]",
        icon: "h-8.5 w-8.5 shrink-0",
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
