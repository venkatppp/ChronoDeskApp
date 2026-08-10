import { forwardRef, useRef, type HTMLAttributes, type ElementType } from "react";
import { cn } from "@/utils/cn";
import { useLiquidGlass } from "@/hooks/useLiquidGlass";
import type { LiquidGlassOptions } from "@/lib/liquidGlass";

/**
 * The material system — one vocabulary for the whole application.
 *
 * LEVEL 1 — chrome: floating structural chrome (sidebar, topbar, dialogs).
 *           Bright specular edge, floats highest. Refraction enabled.
 * LEVEL 2 — surface: large hero panels — softer than chrome, fewer edges,
 *           sits closer to the canvas. Refraction enabled.
 * LEVEL 3 — panel / sheet: secondary panels, sheets. Panel frosts;
 *           sheet refracts.
 * LEVEL 4 — well / control / nav: small glass controls (inputs, buttons,
 *           segments, nav rows). Always frosted — never refraction.
 */
export type GlassMaterial = "chrome" | "surface" | "panel" | "well" | "control" | "nav" | "sheet";
type GlassTag = "div" | "aside" | "header" | "section" | "main" | "nav" | "footer";

export interface GlassSurfaceProps extends HTMLAttributes<HTMLElement> {
  /** Semantic tag to render (defaults to div). */
  as?: GlassTag;
  /**
   * Material dressing. "chrome" = toolbar/sidebar/dialog (strongest tint
   * and blur), "panel" = cards and compact panels, "well" = inset input
   * wells, "control" = small controls, "nav" = sidebar rows, "sheet" =
   * dialogs with a legibility wash.
   */
  material?: GlassMaterial;
  /**
   * Apply real SVG-displacement refraction (Chromium only; falls back to
   * frosted blur elsewhere). Defaults: chrome/panel/sheet refract, small
   * controls frost. Repeated small surfaces (cards, list rows) should pass
   * refraction={false} — the maxArea guard also downgrades oversized
   * surfaces automatically.
   */
  refraction?: boolean;
  /** Optional optics overrides, e.g. { scale: -80 } for a more dramatic rim. */
  optics?: Partial<LiquidGlassOptions>;
}

const MATERIAL_CLASS: Record<GlassMaterial, string> = {
  chrome: "glass-chrome",
  surface: "glass-surface",
  panel: "glass-panel",
  well: "glass-well",
  control: "glass-control",
  nav: "glass-nav",
  sheet: "glass-sheet",
};

const REFRACTS_BY_DEFAULT: Record<GlassMaterial, boolean> = {
  chrome: true,
  surface: true,
  panel: true,
  well: false,
  control: false,
  nav: false,
  sheet: true,
};

/**
 * The single reusable Liquid Glass surface. Owns the optics lifecycle
 * (mount/unmount + resize) and the material dressing in one primitive so
 * no page ever re-implements glass styling.
 */
export const GlassSurface = forwardRef<HTMLElement, GlassSurfaceProps>(
  ({ as: Tag = "div", className, material = "panel", refraction, optics, children, ...props }, forwardedRef) => {
    const localRef = useRef<HTMLElement | null>(null);

    const wantsRefraction = refraction ?? REFRACTS_BY_DEFAULT[material];
    useLiquidGlass(localRef, wantsRefraction ? optics : { maxArea: 0 });

    const setRef = (node: HTMLElement | null) => {
      localRef.current = node;
      if (typeof forwardedRef === "function") forwardedRef(node);
      else if (forwardedRef && typeof forwardedRef === "object") forwardedRef.current = node;
    };

    const TagElement = Tag as ElementType;
    return (
      <TagElement ref={setRef} className={cn(MATERIAL_CLASS[material], className)} {...props}>
        {children}
      </TagElement>
    );
  },
);
GlassSurface.displayName = "GlassSurface";
