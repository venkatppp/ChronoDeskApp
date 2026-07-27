import { type HTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/utils/cn";

const badgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium",
  {
    variants: {
      variant: {
        neutral: "border-(--color-border) bg-(--color-surface-hover) text-(--color-muted-foreground)",
        accent: "border-transparent bg-(--color-accent-muted) text-(--color-accent)",
        warning: "border-transparent bg-(--color-warning)/15 text-(--color-warning)",
        success: "border-transparent bg-(--color-success)/15 text-(--color-success)",
      },
    },
    defaultVariants: {
      variant: "neutral",
    },
  },
);

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement>, VariantProps<typeof badgeVariants> {}

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}
