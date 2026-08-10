import { type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface PageContainerProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/**
 * Standard content frame for every page — one grid, one rhythm.
 * Uses the full available width; readable content sections set their own
 * local widths.
 */
export function PageContainer({ className, children, ...props }: PageContainerProps) {
  return (
    <div className={cn("flex w-full flex-col gap-8 px-6 py-7 lg:px-8", className)} {...props}>
      {children}
    </div>
  );
}
