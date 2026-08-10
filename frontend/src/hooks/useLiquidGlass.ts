import { useEffect, type RefObject } from "react";
import { applyLiquidGlass, type LiquidGlassInstance, type LiquidGlassOptions } from "@/lib/liquidGlass";

/**
 * React binding for the liquid-glass material (see lib/liquidGlass.ts).
 * Attaches refraction to the given element on mount and tears it down on
 * unmount; a debounced ResizeObserver inside the material handles resizes.
 *
 * Usage (mirrors the reference's framework snippet):
 *   const ref = useRef<HTMLDivElement>(null);
 *   useLiquidGlass(ref, { scale: -64 });
 *
 * @param ref   Ref to the element that should behave as liquid glass.
 * @param opts  Optional optics overrides (scale, chroma, blur, maxArea…).
 */
export function useLiquidGlass<T extends HTMLElement>(
  ref: RefObject<T | null>,
  opts?: Partial<LiquidGlassOptions>,
): LiquidGlassInstance | null {
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    return applyLiquidGlass(el, opts).destroy;
    // Optics are declared once per surface; element identity is stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return null;
}
