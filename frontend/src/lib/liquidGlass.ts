/**
 * liquidGlass — Apple-style liquid glass refraction for any element.
 *
 * TypeScript adaptation of the MIT-licensed reference implementation from
 * github.com/deepika-builds/liquid-glass (technique per Aave's "Building
 * glass for the web" and rizroze/liquid-glass). The module owns the optics:
 * SVG displacement filter, canvas displacement map, backdrop-filter wiring,
 * resize handling, and the frosted-blur fallback for browsers that cannot
 * apply SVG-filtered backdrops (Safari, Firefox, jsdom).
 *
 * Visual dressing (tint, highlight, shadows) stays in CSS; this module only
 * adds `backdrop-filter: url(#…) …` inline and manages its lifecycle.
 *
 * NOTE ON PERF: refraction is designed for compact floating surfaces. The
 * `maxArea` guard (default 420,000 px² ≈ 800×525) switches oversized
 * elements to the frosted fallback automatically so large full-screen
 * surfaces never pay the displacement-map cost.
 */

const SVG_NS = "http://www.w3.org/2000/svg";
let uid = 0;
let svgDefs: SVGGElement | null = null;

type WebkitStyle = CSSStyleDeclaration & { webkitBackdropFilter?: string };

function setBackdropFilter(el: HTMLElement, value: string): void {
  el.style.backdropFilter = value;
  (el.style as WebkitStyle).webkitBackdropFilter = value;
}

function clearBackdropFilter(el: HTMLElement): void {
  el.style.backdropFilter = "";
  (el.style as WebkitStyle).webkitBackdropFilter = "";
}

export interface LiquidGlassOptions {
  /** Displacement strength; negative = magnifying bulge (-60 subtle … -180 dramatic). */
  scale: number;
  /** Per-channel scale stagger (prism fringe); 0 disables. */
  chroma: number;
  /** Neutral interior inset as a fraction of the smaller side. */
  border: number;
  /** Edge-curvature softness (px) of the map's gray inset. */
  mapBlur: number;
  /** Backdrop blur (px) behind the glass interior. */
  blur: number;
  /** Backdrop saturation boost. */
  saturate: number;
  /** Corner radius override (px); default reads computed border-radius. */
  radius: number | null;
  /** Frosted blur (px) where refraction is unsupported or oversized. */
  fallbackBlur: number;
  /** Skip refraction when element area (px²) exceeds this. */
  maxArea: number;
}

export interface LiquidGlassInstance {
  supported: boolean;
  /** Regenerate the displacement map after manual size changes. */
  refresh: () => void;
  /** Remove the effect and any inline styles the module added. */
  destroy: () => void;
}

const DEFAULTS: Required<LiquidGlassOptions> = {
  // Reference optics tuned for an app shell: a magnifying bulge at the
  // rim with a faint prism fringe. -112/+5 is the "chrome" default from
  // the handoff — visible lensing on large floating surfaces, never
  // dramatic on small controls.
  scale: -112,
  chroma: 5,
  border: 0.06,
  mapBlur: 12,
  blur: 8,
  saturate: 1.5,
  radius: null,
  fallbackBlur: 24,
  maxArea: 420_000,
};

/** Chromium can apply SVG filters via backdrop-filter; Safari/Firefox no-op,
 *  so they get the frosted fallback instead. */
const supported: boolean = (() => {
  const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
  const isSafari = /Safari/.test(ua) && !/Chrome|Chromium|Edg/.test(ua);
  const isFirefox = /Firefox/.test(ua);
  if (isSafari || isFirefox) return false;
  if (typeof CSS === "undefined" || typeof CSS.supports !== "function") return false;
  if (!CSS.supports("backdrop-filter", "url(#lg)")) return false;
  try {
    const c = document.createElement("canvas");
    c.width = c.height = 4;
    const ctx = c.getContext("2d");
    if (!ctx) return false;
    ctx.getImageData(0, 0, 1, 1);
    return true;
  } catch {
    return false;
  }
})();

function ensureDefs(): SVGGElement {
  if (svgDefs) return svgDefs;
  const svg = document.createElementNS(SVG_NS, "svg");
  // width/height 0 keeps it renderable (display:none would break feImage).
  svg.setAttribute("width", "0");
  svg.setAttribute("height", "0");
  svg.setAttribute("aria-hidden", "true");
  svg.style.position = "absolute";
  svgDefs = document.createElementNS(SVG_NS, "defs");
  svg.appendChild(svgDefs);
  document.body.appendChild(svg);
  return svgDefs;
}

/**
 * Displacement map, gradient-difference method: a red left→right ramp
 * encodes X displacement, a blue top→bottom ramp encodes Y ("difference"
 * keeps both since the channels are disjoint). A blurred, inset 50%-gray
 * rounded rect neutralizes the interior, confining refraction to an edge
 * band whose curvature is set by the blur radius.
 */
function makeMap(w: number, h: number, radius: number, border: number, mapBlur: number): string {
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) return "";

  const gx = ctx.createLinearGradient(0, 0, w, 0);
  gx.addColorStop(0, "rgb(0,0,0)");
  gx.addColorStop(1, "rgb(255,0,0)");
  ctx.fillStyle = gx;
  ctx.fillRect(0, 0, w, h);

  const gy = ctx.createLinearGradient(0, 0, 0, h);
  gy.addColorStop(0, "rgb(0,0,0)");
  gy.addColorStop(1, "rgb(0,0,255)");
  ctx.globalCompositeOperation = "difference";
  ctx.fillStyle = gy;
  ctx.fillRect(0, 0, w, h);

  ctx.globalCompositeOperation = "source-over";
  const inset = border * Math.min(w, h);
  ctx.filter = `blur(${mapBlur}px)`;
  ctx.fillStyle = "rgba(128,128,128,0.93)";
  ctx.beginPath();
  ctx.roundRect(inset, inset, w - inset * 2, h - inset * 2, Math.max(radius - inset, 2));
  ctx.fill();
  ctx.filter = "none";
  return canvas.toDataURL();
}

/** Three displacement passes at staggered scales (R strongest), channels
 *  isolated with feColorMatrix and recombined with screen blends — the
 *  faint prism fringe at the rim. */
function buildFilter(id: string, scales: number[]): { filter: SVGFilterElement; feImage: SVGImageElement } {
  const filter = document.createElementNS(SVG_NS, "filter");
  filter.setAttribute("id", id);
  filter.setAttribute("x", "0");
  filter.setAttribute("y", "0");
  filter.setAttribute("width", "100%");
  filter.setAttribute("height", "100%");
  // Load-bearing: filters default to linearRGB, which re-maps the map's
  // neutral gray 128 to ~0.216 and injects a constant phantom displacement.
  filter.setAttribute("color-interpolation-filters", "sRGB");

  const feImage = document.createElementNS(SVG_NS, "image");
  feImage.setAttribute("x", "0");
  feImage.setAttribute("y", "0");
  feImage.setAttribute("result", "map");
  feImage.setAttribute("preserveAspectRatio", "none");
  filter.appendChild(feImage);

  const keep = [
    "1 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 1 0",
    "0 0 0 0 0  0 1 0 0 0  0 0 0 0 0  0 0 0 1 0",
    "0 0 0 0 0  0 0 0 0 0  0 0 1 0 0  0 0 0 1 0",
  ];
  const channels: string[] = [];
  for (let i = 0; i < 3; i++) {
    const disp = document.createElementNS(SVG_NS, "feDisplacementMap");
    disp.setAttribute("in", "SourceGraphic");
    disp.setAttribute("in2", "map");
    disp.setAttribute("scale", String(scales[i]));
    disp.setAttribute("xChannelSelector", "R");
    disp.setAttribute("yChannelSelector", "B");
    disp.setAttribute("result", `d${i}`);
    filter.appendChild(disp);

    const cm = document.createElementNS(SVG_NS, "feColorMatrix");
    cm.setAttribute("in", `d${i}`);
    cm.setAttribute("type", "matrix");
    cm.setAttribute("values", keep[i]);
    cm.setAttribute("result", `c${i}`);
    filter.appendChild(cm);
    channels.push(`c${i}`);
  }

  const blend1 = document.createElementNS(SVG_NS, "feBlend");
  blend1.setAttribute("in", channels[0]);
  blend1.setAttribute("in2", channels[1]);
  blend1.setAttribute("mode", "screen");
  blend1.setAttribute("result", "c01");
  filter.appendChild(blend1);

  const blend2 = document.createElementNS(SVG_NS, "feBlend");
  blend2.setAttribute("in", "c01");
  blend2.setAttribute("in2", channels[2]);
  blend2.setAttribute("mode", "screen");
  filter.appendChild(blend2);

  ensureDefs().appendChild(filter);
  return { filter, feImage };
}

function resolveRadius(el: HTMLElement, w: number, h: number, override: number | null): number {
  if (override != null) return override;
  const raw = getComputedStyle(el).borderTopLeftRadius || "0px";
  const v = parseFloat(raw) || 0;
  return raw.trim().endsWith("%") ? (v / 100) * Math.min(w, h) : v;
}

/** Apply liquid glass to an element. Returns a handle with refresh/destroy. */
export function applyLiquidGlass(el: HTMLElement, opts?: Partial<LiquidGlassOptions>): LiquidGlassInstance {
  const o: Required<LiquidGlassOptions> = { ...DEFAULTS, ...opts };

  const frosted = (blur: number) => `blur(${blur}px) saturate(${o.saturate})`;
  const applyFrosted = (blur: number) => {
    setBackdropFilter(el, frosted(blur));
    el.classList.add("lg-fallback");
    el.classList.remove("lg-refract");
  };

  if (!supported) {
    applyFrosted(o.fallbackBlur);
    return {
      supported: false,
      refresh: () => {},
      destroy: () => {
        clearBackdropFilter(el);
        el.classList.remove("lg-fallback");
      },
    };
  }

  const id = `lg-filter-${++uid}`;
  const scales = [o.scale, o.scale + o.chroma, o.scale + 2 * o.chroma];
  const parts = buildFilter(id, scales);

  let active = true;

  function refresh(): void {
    if (!active) return;
    const w = el.offsetWidth;
    const h = el.offsetHeight;
    if (!w || !h) return;
    if (w * h > o.maxArea) {
      // Oversized surface: skip refraction, keep the frosted material.
      parts.feImage.setAttribute("href", "");
      setBackdropFilter(el, frosted(o.blur));
      el.classList.add("lg-fallback");
      el.classList.remove("lg-refract");
      return;
    }
    const radius = resolveRadius(el, w, h, o.radius);
    parts.feImage.setAttribute("href", makeMap(w, h, radius, o.border, o.mapBlur));
    parts.feImage.setAttribute("width", String(w));
    parts.feImage.setAttribute("height", String(h));
  }

  refresh();
  if (el.classList.contains("lg-fallback")) {
    // Exceeded maxArea on first pass; nothing more to do.
    return {
      supported: true,
      refresh,
      destroy: () => {
        active = false;
        parts.filter.remove();
        clearBackdropFilter(el);
        el.classList.remove("lg-fallback");
      },
    };
  }

  setBackdropFilter(el, `url(#${id}) blur(${o.blur}px) saturate(${o.saturate})`);
  el.classList.add("lg-refract");
  el.classList.remove("lg-fallback");

  let timer: ReturnType<typeof setTimeout> | null = null;
  const ro = new ResizeObserver(() => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(refresh, 120);
  });
  ro.observe(el);

  return {
    supported: true,
    refresh,
    destroy: () => {
      active = false;
      ro.disconnect();
      if (timer) clearTimeout(timer);
      parts.filter.remove();
      clearBackdropFilter(el);
      el.classList.remove("lg-refract");
      el.classList.remove("lg-fallback");
    },
  };
}
