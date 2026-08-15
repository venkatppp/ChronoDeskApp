import { useEffect, type RefObject } from "react";
import { useReducedMotion } from "@/utils/motion";

/**
 * useGlare — cursor-tracked specular glare for glass surfaces.
 *
 * Drives the --gx/--gy custom properties consumed by the `.glare` CSS
 * overlay (see index.css). The pointer position is sampled relative to
 * the element and written through rAF so the highlight follows the
 * cursor smoothly without layout thrash.
 *
 * Under prefers-reduced-motion the glare is intentionally NOT tracked:
 * the CSS holds it at its resting bias position instead.
 */
export function useGlare<T extends HTMLElement>(ref: RefObject<T | null>): void {
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    const el = ref.current;
    if (!el || reducedMotion) return;

    let frame = 0;

    const onPointerMove = (e: PointerEvent) => {
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        const rect = el.getBoundingClientRect();
        const x = ((e.clientX - rect.left) / rect.width) * 100;
        const y = ((e.clientY - rect.top) / rect.height) * 100;
        el.style.setProperty("--gx", `${x.toFixed(1)}%`);
        el.style.setProperty("--gy", `${y.toFixed(1)}%`);
      });
    };

    el.addEventListener("pointermove", onPointerMove);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      el.removeEventListener("pointermove", onPointerMove);
    };
    // Element identity is stable; tracking depends only on the motion
    // preference (matches the useLiquidGlass pattern).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reducedMotion]);
}