import { type RefObject } from "react";
import { cn } from "@/utils/cn";
import { useScrollEdge } from "@/hooks/useScrollEdge";

/**
 * Scroll edge treatment for glass chrome — the web equivalent of Apple's
 * `scrollEdgeEffectStyle`. Renders a pointer-transparent strip at the top
 * (or bottom) edge of its parent, driven by the state of a scroll
 * container:
 *
 *   mode="soft"  — gentle shading strip (used where chrome and content are
 *                  already close in tone; the dissolve mask on the scroll
 *                  container itself stays the primary treatment).
 *   mode="dark"  — the dimming treatment for dark content scrolling under.
 *   mode="hard"  — uniform gradient across the FULL height of the parent
 *                  (for pinned accessory views under a toolbar).
 *   mode="auto"  — dark treatment when darker content scrolls under,
 *                  soft otherwise.
 *
 * The strip fades in/out with the scroll state (no hard toggles).
 */
export interface ScrollEdgeProps {
  /** The scroll container driving the effect. */
  containerRef: RefObject<HTMLElement | null>;
  mode?: "soft" | "dark" | "hard" | "auto";
  side?: "top" | "bottom";
  className?: string;
}

export function ScrollEdge({ containerRef, mode = "auto", side = "top", className }: ScrollEdgeProps) {
  const { scrollY, darkContent } = useScrollEdge(containerRef);
  const scrolled = scrollY > 4;

  const treatment =
    mode === "hard"
      ? "scroll-edge-hard"
      : mode === "dark" || (mode === "auto" && darkContent)
        ? "scroll-edge-dark"
        : "scroll-edge-soft";

  return (
    <div
      aria-hidden="true"
      className={cn(
        "pointer-events-none absolute z-[1] transition-opacity duration-300",
        side === "top" ? "inset-x-0 top-0" : "inset-x-0 bottom-0",
        mode === "hard" ? "h-full" : "h-8",
        scrolled ? "opacity-100" : "opacity-0",
        treatment,
        className,
      )}
    />
  );
}