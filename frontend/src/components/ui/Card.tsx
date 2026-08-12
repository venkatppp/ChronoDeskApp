import { forwardRef, type HTMLAttributes } from "react";
import { cn } from "@/utils/cn";

/**
 * Standard content card — calm, non-glass surface for the content layer.
 * Per Apple HIG: Liquid Glass is reserved for the functional layer
 * (navigation, toolbars, controls, sheets). Content cards use opaque
 * elevated surfaces with hairline borders and subtle shadows.
 *
 * Use `variant="glass"` ONLY for functional-layer cards that genuinely
 * need Liquid Glass (e.g., sheet/dialog content, popover panels).
 */
export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: "content" | "glass";
}

export const Card = forwardRef<HTMLDivElement, CardProps>(({ className, variant = "content", ...props }, ref) => (
  <div
    ref={ref}
    className={cn(
      "rounded-[var(--radius-card)] transition-all duration-300 ease-[var(--ease-premium)]",
      variant === "glass" ? "glass-panel" : "content-card",
      className,
    )}
    {...props}
  />
));
Card.displayName = "Card";

export const CardHeader = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex flex-col gap-1 p-5 pb-3", className)} {...props} />
  ),
);
CardHeader.displayName = "CardHeader";

export const CardTitle = forwardRef<HTMLParagraphElement, HTMLAttributes<HTMLParagraphElement>>(
  ({ className, ...props }, ref) => (
    <h3
      ref={ref}
      className={cn("font-(family-name:--font-display) text-[15px] font-semibold tracking-tight", className)}
      {...props}
    />
  ),
);
CardTitle.displayName = "CardTitle";

export const CardDescription = forwardRef<HTMLParagraphElement, HTMLAttributes<HTMLParagraphElement>>(
  ({ className, ...props }, ref) => (
    <p ref={ref} className={cn("text-[13px] leading-relaxed text-(--color-muted-foreground)", className)} {...props} />
  ),
);
CardDescription.displayName = "CardDescription";

export const CardContent = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => <div ref={ref} className={cn("p-5 pt-0", className)} {...props} />,
);
CardContent.displayName = "CardContent";

export const CardFooter = forwardRef<HTMLDivElement, HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => <div ref={ref} className={cn("flex items-center p-5 pt-0", className)} {...props} />,
);
CardFooter.displayName = "CardFooter";
