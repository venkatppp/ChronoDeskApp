import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Merges conditional class names and resolves conflicting Tailwind
 * utility classes (e.g. `px-2` vs `px-4`) so the last one wins.
 *
 * This is the standard shadcn/ui `cn` helper — every component in
 * `components/ui` composes class names through this function so that
 * consumers can always override styling via a `className` prop.
 */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
