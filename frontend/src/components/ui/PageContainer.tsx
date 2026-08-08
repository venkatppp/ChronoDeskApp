import { type HTMLAttributes, type ReactNode } from "react";
import { cn } from "@/utils/cn";

interface PageContainerProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Standard content frame for every page — one grid, one rhythm. */
export function PageContainer({ className, children, ...props }: PageContainerProps) {
  return (
    <div
      className={cn("mx-auto flex w-full max-w-6xl flex-col gap-8 px-8 py-8 lg:px-10", className)}
      {...props}
    >
      {children}
    </div>
  );
}