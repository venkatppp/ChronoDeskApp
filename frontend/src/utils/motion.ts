import { useCallback, useEffect, useRef, useState } from "react";

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(false);
  useEffect(() => {
    const mql = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReduced(mql.matches);
    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);
  return reduced;
}

export const SPRING = {
  smooth: { duration: 350, easing: [0.32, 0.08, 0.24, 1] } as const,
  stiff: { duration: 200, easing: [0.32, 0.08, 0.24, 1] } as const,
  bouncy: { duration: 450, easing: [0.34, 1.56, 0.64, 1] } as const,
  slow: { duration: 500, easing: [0.32, 0.08, 0.24, 1] } as const,
} as const;

export const ELEVATION = {
  flat: "shadow-[0_0_0_0_rgba(0,0,0,0)]",
  raised:
    "shadow-[0_1px_2px_rgba(0,0,0,0.3),0_1px_3px_rgba(0,0,0,0.15)]",
  lifted:
    "shadow-[0_4px_12px_rgba(0,0,0,0.4),0_2px_6px_rgba(0,0,0,0.2)]",
  elevated:
    "shadow-[0_8px_24px_rgba(0,0,0,0.5),0_4px_12px_rgba(0,0,0,0.25)]",
} as const;

export function useHover() {
  const [isHovered, setIsHovered] = useState(false);
  const onMouseEnter = useCallback(() => setIsHovered(true), []);
  const onMouseLeave = useCallback(() => setIsHovered(false), []);
  return { isHovered, handlers: { onMouseEnter, onMouseLeave } };
}

export function useMounted(initial = false) {
  const [mounted, setMounted] = useState(initial);
  const on = useCallback(() => setMounted(true), []);
  const off = useCallback(() => setMounted(false), []);
  const toggle = useCallback(() => setMounted((p) => !p), []);
  return { mounted, on, off, toggle };
}

export function useStaggeredIndex(index: number, baseDelay = 30): string {
  const delay = index * baseDelay;
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (el) {
      el.style.opacity = "0";
      el.style.transform = "translateY(6px)";
      const id = requestAnimationFrame(() => {
        el.style.transition = `opacity ${SPRING.smooth.duration}ms ${SPRING.smooth.easing}, transform ${SPRING.smooth.duration}ms ${SPRING.smooth.easing}`;
        el.style.transitionDelay = `${delay}ms`;
        el.style.opacity = "1";
        el.style.transform = "translateY(0)";
      });
      return () => cancelAnimationFrame(id);
    }
  }, [delay]);

  return "opacity-0 translate-y-1.5";
}

export function springStyle(duration = 350): React.CSSProperties {
  return {
    transitionDuration: `${duration}ms`,
    transitionTimingFunction: `cubic-bezier(0.32, 0.08, 0.24, 1)`,
  };
}
