import { type HTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/utils/cn";

const badgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[11px] font-medium leading-none",
  {
    variants: {
      variant: {
        neutral: "border-(--color-border-subtle) bg-(--color-surface) text-(--color-muted-foreground)",
        accent: "border-transparent bg-(--color-accent-muted) text-(--color-accent)",
        cyan: "border-transparent bg-(--color-cyan)/12 text-(--color-cyan)",
        violet: "border-transparent bg-(--color-violet)/14 text-(--color-violet)",
        emerald: "border-transparent bg-(--color-emerald)/14 text-(--color-emerald)",
        orange: "border-transparent bg-(--color-orange)/14 text-(--color-orange)",
        warning: "border-transparent bg-(--color-warning)/15 text-(--color-warning)",
        success: "border-transparent bg-(--color-success)/15 text-(--color-success)",
        danger: "border-transparent bg-(--color-danger)/15 text-(--color-danger)",
        outline: "border-(--color-border) bg-transparent text-(--color-muted-foreground)",
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
