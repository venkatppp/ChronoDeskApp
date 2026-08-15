import { useEffect, useRef, useState, type ReactNode } from "react";
import { GlassSurface } from "@/components/ui/GlassSurface";
import { useReducedMotion } from "@/utils/motion";
import { cn } from "@/utils/cn";

/**
 * DraggableGlassCard — a Liquid Glass card that floats over the page with
 * real spring physics, ported from the reference demo
 * (github.com/deepika-builds/liquid-glass).
 *
 * The card lives in `position: fixed`, so the page scrolls *underneath* it
 * and the refraction map bends whatever detail passes behind it. A
 * velocity-squash transform gives it a droplet-like stretch along its
 * direction of travel; the rim clamps the card inside the window.
 *
 * Interactive children (buttons, links, inputs) keep their own behavior —
 * drags only start from the glass itself. Under prefers-reduced-motion the
 * card is inert and rests at its original spot.
 */
export function DraggableGlassCard({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  const ref = useRef<HTMLElement | null>(null);
  const reducedMotion = useReducedMotion();

  const [rest] = useState(() => {
    if (typeof window === "undefined") return { x: 64, y: 96 };
    return {
      x: Math.max(16, window.innerWidth * 0.68 - 170),
      y: Math.max(16, window.innerHeight * 0.3 - 212),
    };
  });

  useEffect(() => {
    const el = ref.current;
    if (!el || reducedMotion) return;

    const RIM = 12;
    const STIFFNESS = 0.12;
    const DAMPING = 0.72;

    let w = el.offsetWidth;
    let h = el.offsetHeight;
    let x = rest.x;
    let y = rest.y;
    let tx = x;
    let ty = y;
    let vx = 0;
    let vy = 0;
    let dragging = false;
    let grabDX = 0;
    let grabDY = 0;
    let frame = 0;
    let disposed = false;

    const clampTarget = () => {
      tx = Math.min(Math.max(tx, RIM), window.innerWidth - w - RIM);
      ty = Math.min(Math.max(ty, RIM), window.innerHeight - h - RIM);
    };

    const render = () => {
      const speed = Math.hypot(vx, vy);
      const squash = Math.min(speed / 120, 0.08);
      const angle = Math.atan2(vy, vx);
      el.style.transform =
        `translate(${x}px, ${y}px) ` +
        `rotate(${angle}rad) scale(${1 + squash}, ${1 - squash}) rotate(${-angle}rad)`;
    };

    const tick = () => {
      if (disposed) return;
      vx = (vx + (tx - x) * STIFFNESS) * DAMPING;
      vy = (vy + (ty - y) * STIFFNESS) * DAMPING;
      x += vx;
      y += vy;
      render();
      frame = requestAnimationFrame(tick);
    };

    const onDown = (e: PointerEvent) => {
      if (e.target instanceof Element && e.target.closest("input, button, a, label")) return;
      dragging = true;
      grabDX = e.clientX - tx;
      grabDY = e.clientY - ty;
      try {
        el.setPointerCapture(e.pointerId);
      } catch {
        /* synthetic pointer */
      }
    };

    const onMove = (e: PointerEvent) => {
      if (!dragging) return;
      tx = e.clientX - grabDX;
      ty = e.clientY - grabDY;
      clampTarget();
    };

    const onUp = (e: PointerEvent) => {
      dragging = false;
      if (el.hasPointerCapture(e.pointerId)) el.releasePointerCapture(e.pointerId);
    };

    const onResize = () => {
      w = el.offsetWidth;
      h = el.offsetHeight;
      clampTarget();
    };

    el.addEventListener("pointerdown", onDown);
    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerup", onUp);
    el.addEventListener("pointercancel", onUp);
    window.addEventListener("resize", onResize);

    clampTarget();
    x = tx;
    y = ty;
    render();
    frame = requestAnimationFrame(tick);

    return () => {
      disposed = true;
      if (frame) cancelAnimationFrame(frame);
      el.removeEventListener("pointerdown", onDown);
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerup", onUp);
      el.removeEventListener("pointercancel", onUp);
      window.removeEventListener("resize", onResize);
    };
  }, [reducedMotion, rest.x, rest.y]);

  return (
    <GlassSurface
      ref={ref}
      material="chrome"
      optics={{ scale: -90, chroma: 5 }}
      className={cn(
        "fixed left-0 top-0 z-30 cursor-grab touch-none select-none will-change-transform active:cursor-grabbing",
        className,
      )}
      style={{ position: "fixed", transform: `translate(${rest.x}px, ${rest.y}px)` }}
    >
      {children}
    </GlassSurface>
  );
}