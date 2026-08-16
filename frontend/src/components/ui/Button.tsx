import { forwardRef, type ButtonHTMLAttributes } from "react";
import { motion } from "motion/react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/utils/cn";
import { springs } from "@/lib/springs";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius-control)] text-sm font-medium " +
    "transition-all duration-100 ease-out disabled:pointer-events-none disabled:opacity-50 " +
    "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-accent)",
  {
    variants: {
      variant: {
        /* Illuminated glass — translucent blue pane with a bright
           specular top edge. Never a solid blue rectangle. Press is a
           spring scale (motion) — interruptible and physical. */
        primary:
          "glass-accent text-(--color-accent-foreground) illuminate " +
          "hover:brightness-[1.06] active:brightness-[0.97]",
        /* Neutral glass — quiet frosted control, minimal border. */
        secondary:
          "glass-control text-(--color-foreground) illuminate " +
          "hover:bg-(--color-surface-raised)/60",
        ghost: "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground) motion-safe:active:scale-[0.97]",
        danger:
          "bg-(--color-danger)/90 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.25),0_4px_16px_rgba(0,0,0,0.3)] hover:brightness-[1.06] motion-safe:active:scale-[0.97]",
        outline:
          "border border-(--color-border) bg-transparent text-(--color-muted-foreground) " +
          "hover:bg-(--color-surface-hover) hover:text-(--color-foreground) hover:border-(--color-border) motion-safe:active:scale-[0.97]",
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
  extends Omit<
      ButtonHTMLAttributes<HTMLButtonElement>,
      | "onDrag"
      | "onDragStart"
      | "onDragEnd"
      | "onDragEnter"
      | "onDragExit"
      | "onDragLeave"
      | "onDragOver"
      | "onAnimationStart"
    >,
    VariantProps<typeof buttonVariants> {}

/**
 * Glass buttons. The two glass variants (primary/secondary) get a spring
 * press-scale from motion — interruptible, with a physical settle — plus
 * inner illumination on press. The quiet variants stay pure CSS.
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, ...props }, ref) => {
    const isGlass = variant === "primary" || variant === "secondary";
    return (
      <motion.button
        ref={ref}
        className={cn(buttonVariants({ variant, size }), className)}
        whileTap={isGlass ? { scale: 0.97 } : undefined}
        transition={isGlass ? springs.snap : undefined}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";