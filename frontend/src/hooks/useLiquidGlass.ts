import { useEffect, useRef, useState, type RefObject } from "react";
import { applyLiquidGlass, type LiquidGlassInstance, type LiquidGlassOptions } from "@/lib/liquidGlass";

/**
 * React binding for the liquid-glass material (see lib/liquidGlass.ts).
 * Attaches refraction to the given element on mount and tears it down on
 * unmount; a debounced ResizeObserver inside the material handles resizes.
 *
 * Returns the live `LiquidGlassInstance` (or null before mount / on
 * unsupported engines) so callers can drive surface-level refraction
 * state — e.g. `instance.refresh()` after a manual size change.
 *
 * Usage (mirrors the reference's framework snippet):
 *   const ref = useRef<HTMLDivElement>(null);
 *   const glass = useLiquidGlass(ref, { scale: -64 });
 *
 * @param ref   Ref to the element that should behave as liquid glass.
 * @param opts  Optional optics overrides (scale, chroma, blur, maxArea…).
 */
export function useLiquidGlass<T extends HTMLElement>(
  ref: RefObject<T | null>,
  opts?: Partial<LiquidGlassOptions>,
): LiquidGlassInstance | null {
  const [instance, setInstance] = useState<LiquidGlassInstance | null>(null);
  const optsRef = useRef(opts);
  optsRef.current = opts;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    // "Reduce Transparency" is handled in CSS (near-opaque surfaces, no
    // backdrop-filter). The module's INLINE backdrop-filter would override
    // that media query, so skip applying it entirely while the preference
    // is active — and re-apply if the user toggles it at runtime.
    const reducedTransparency =
      typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-reduced-transparency: reduce)")
        : null;

    let inst: LiquidGlassInstance | null = null;
    const teardown = () => {
      inst?.destroy();
      inst = null;
      setInstance(null);
    };
    const apply = () => {
      if (reducedTransparency?.matches) return;
      inst = applyLiquidGlass(el, optsRef.current);
      setInstance(inst);
    };
    const onChange = () => {
      teardown();
      apply();
    };
    apply();
    reducedTransparency?.addEventListener("change", onChange);
    return () => {
      reducedTransparency?.removeEventListener("change", onChange);
      teardown();
    };
    // Optics are declared once per surface; element identity is stable.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return instance;
}
