import { useEffect, useRef, useState, type RefObject } from "react";
import { useTheme } from "@/hooks/useTheme";

export interface ScrollEdgeState {
  atTop: boolean;
  atBottom: boolean;
  scrollY: number;
  /**
   * True when the content scrolling under the chrome edge is darker than
   * the chrome itself — the trigger for the "dark style" scroll edge
   * (dimming instead of dissolve). Computed by evaluating the ambient
   * environment gradients (the only light source behind every surface)
   * at the position of the chrome edge — deterministic, theme-aware, and
   * cheap. Content cards are near-transparent over this environment, so
   * its luminance is the honest signal of what the glass is seeing.
   */
  darkContent: boolean;
}

interface EnvLayer {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  stop: number;
  varName: string;
}

/* Mirrors the six radial gradients of `.bg-env` in index.css. */
const ENV_LAYERS: EnvLayer[] = [
  { cx: 0.1, cy: -0.1, rx: 1300, ry: 850, stop: 0.62, varName: "--ambient-blue" },
  { cx: 0.95, cy: -0.08, rx: 1100, ry: 800, stop: 0.6, varName: "--ambient-cyan" },
  { cx: 0.04, cy: 1.1, rx: 1200, ry: 900, stop: 0.62, varName: "--ambient-violet" },
  { cx: 0.98, cy: 1.08, rx: 900, ry: 700, stop: 0.6, varName: "--ambient-emerald" },
  { cx: 0.72, cy: 0.26, rx: 760, ry: 520, stop: 0.64, varName: "--ambient-warm" },
  { cx: 0.5, cy: 0.4, rx: 1500, ry: 1000, stop: 0.66, varName: "--ambient-core" },
];

function parseColor(raw: string): [number, number, number] | null {
  const m = raw.match(/rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:[,\s/]+([\d.]+))?\)/);
  if (!m) return null;
  const alpha = m[4] !== undefined ? parseFloat(m[4]) : 1;
  return [parseFloat(m[1]) * alpha, parseFloat(m[2]) * alpha, parseFloat(m[3]) * alpha];
}

/** Average relative luminance of the environment under the chrome's top
 *  edge, evaluated from the `.bg-env` gradient stack (see index.css). */
function envLuminanceAtEdge(): number | null {
  if (typeof document === "undefined") return null;
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const sampleY = 16;
  const sampleX = vw / 2;
  const styles = getComputedStyle(document.documentElement);

  let r = 0;
  let g = 0;
  let b = 0;

  for (const layer of ENV_LAYERS) {
    const raw = styles.getPropertyValue(layer.varName).trim();
    if (!raw) continue;
    const [lr, lg, lb] = parseColor(raw) ?? [0, 0, 0];
    const u = Math.sqrt(
      ((sampleX - layer.cx * vw) / layer.rx) ** 2 +
        ((sampleY - layer.cy * vh) / layer.ry) ** 2,
    );
    const coverage = Math.max(0, Math.min(1, 1 - u / layer.stop));
    const a = coverage;
    const lA = a;
    r = lr * lA + r * (1 - lA);
    g = lg * lA + g * (1 - lA);
    b = lb * lA + b * (1 - lA);
  }

  return (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
}

/**
 * Scroll-edge detection for glass chrome. Reports scroll position plus
 * whether darker-than-chrome content is passing under the top edge.
 *
 * `darkContent` only ever turns on in the light theme: in dark mode the
 * chrome is already dark, so the "glass flipped dark" case that Apple's
 * dimming treatment is for never occurs.
 */
export function useScrollEdge(
  containerRef: RefObject<HTMLElement | null>,
): ScrollEdgeState {
  const { resolvedTheme } = useTheme();
  const isLight = resolvedTheme === "light";
  const stateRef = useRef<ScrollEdgeState>({
    atTop: true,
    atBottom: false,
    scrollY: 0,
    darkContent: false,
  });
  const [state, setState] = useState<ScrollEdgeState>(stateRef.current);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    let frame = 0;

    const measure = () => {
      frame = 0;
      const { scrollTop, clientHeight, scrollHeight } = el;
      const luminance = isLight ? envLuminanceAtEdge() : null;
      const next: ScrollEdgeState = {
        atTop: scrollTop <= 4,
        atBottom: scrollHeight - scrollTop - clientHeight <= 24,
        scrollY: scrollTop,
        darkContent: isLight && scrollTop > 8 && luminance !== null && luminance < 0.3,
      };
      const prev = stateRef.current;
      if (
        next.atTop !== prev.atTop ||
        next.atBottom !== prev.atBottom ||
        next.scrollY !== prev.scrollY ||
        next.darkContent !== prev.darkContent
      ) {
        stateRef.current = next;
        setState(next);
      }
    };

    const onScroll = () => {
      if (frame) return;
      frame = requestAnimationFrame(measure);
    };

    measure();
    el.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", measure);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      el.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", measure);
    };
    // The container element is stable for the lifetime of the component.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLight]);

  return state;
}