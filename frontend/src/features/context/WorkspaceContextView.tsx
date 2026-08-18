import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from "react";
import { Maximize, ZoomIn, ZoomOut } from "lucide-react";
import type { ContextEdge, ContextEdgeTier, ContextLayout, GraphFocus } from "./types";
import { EMPTY_FOCUS } from "./types";
import { anchorToward } from "./layout";
import { ContextNodeCard } from "./ContextNodeCard";
import { cn } from "@/utils/cn";

/**
 * Workspace Context graph — a real interactive graph, not a mockup.
 *
 * Rendering is separated from application state:
 *  - the page owns selection + search (state);
 *  - this view owns the camera, hover, and the render layers.
 *
 * Performance contract (the previous graph implementation lagged):
 *  - camera (pan/zoom/momentum/tween) lives in refs; the transform is
 *    written to the single camera layer inside a shared rAF batch.
 *    React never re-renders on pointer moves.
 *  - node cards are memoized; hover/selection only re-render the cards
 *    whose emphasis actually changed.
 *  - the edge layer is an SVG memoized on (edges, layout, focus) — pure
 *    world coordinates, GPU-composited, so the relationship layer could
 *    move to Canvas/WebGL later without touching the data model.
 *  - no layout thrashing: the only per-frame DOM write is the camera
 *    layer's transform + the zoom label; the container rect is cached.
 */

export interface WorkspaceContextViewHandle {
  flyToNode: (id: string, scale?: number) => void;
  fitToView: () => void;
}

interface WorkspaceContextViewProps {
  edges: ContextEdge[];
  layout: ContextLayout;
  selectedId: string | null;
  onSelect: (id: string) => void;
  onDeselect: () => void;
  /** Search is active — non-matching nodes recede. */
  searchActive: boolean;
  searchMatches: Set<string>;
  className?: string;
}

const MIN_K = 0.2;
const MAX_K = 2.6;
const FIT_PADDING_RATIO = 0.92;
const MOMENTUM_DECAY = 0.92;
const MOMENTUM_STOP = 0.4;

type CameraMode = "idle" | "snap" | "smooth" | "momentum" | "tween";

interface Camera {
  x: number;
  y: number;
  k: number;
}

const EDGE_STYLE: Record<ContextEdgeTier, { w: number; o: number; fw: number; fo: number }> = {
  primary: { w: 1.1, o: 0.45, fw: 1.7, fo: 0.9 },
  secondary: { w: 0.9, o: 0.26, fw: 1.3, fo: 0.62 },
  faint: { w: 0.7, o: 0.12, fw: 1.0, fo: 0.34 },
};

const raf = typeof requestAnimationFrame === "function"
  ? requestAnimationFrame
  : (cb: FrameRequestCallback) => window.setTimeout(() => cb(performance.now()), 16);

/** Smooth quadratic curve from one node edge to the other, bowing
 *  perpendicular by a deterministic sign so lines never zigzag. */
function edgePath(
  s: { x: number; y: number },
  t: { x: number; y: number },
  id: string,
  ox: number,
  oy: number,
): string {
  const ax = s.x - ox;
  const ay = s.y - oy;
  const bx = t.x - ox;
  const by = t.y - oy;
  const mx = (ax + bx) / 2;
  const my = (ay + by) / 2;
  const dx = bx - ax;
  const dy = by - ay;
  const len = Math.max(Math.hypot(dx, dy), 1);
  const bow = Math.min(Math.max(len * 0.16, 18), 64);
  let hash = 2166136261;
  for (let i = 0; i < id.length; i++) {
    hash ^= id.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  const sign = hash % 2 === 0 ? 1 : -1;
  const cx = mx + (-dy / len) * bow * sign;
  const cy = my + (dx / len) * bow * sign;
  return `M ${ax.toFixed(1)} ${ay.toFixed(1)} Q ${cx.toFixed(1)} ${cy.toFixed(1)} ${bx.toFixed(1)} ${by.toFixed(1)}`;
}

export const WorkspaceContextView = forwardRef<WorkspaceContextViewHandle, WorkspaceContextViewProps>(
  function WorkspaceContextView(
    { edges, layout, selectedId, onSelect, onDeselect, searchActive, searchMatches, className },
    forwardedRef,
  ) {
    const containerRef = useRef<HTMLDivElement | null>(null);
    const layerRef = useRef<HTMLDivElement | null>(null);
    const zoomLabelRef = useRef<HTMLSpanElement | null>(null);

    /* ---- Camera: refs only — React never re-renders for it. --------- */
    const camRef = useRef<Camera>({ x: 0, y: 0, k: 1 });
    const targetRef = useRef<Camera>({ x: 0, y: 0, k: 1 });
    const modeRef = useRef<CameraMode>("idle");
    const velRef = useRef({ x: 0, y: 0 });
    const tweenRef = useRef({ from: { x: 0, y: 0, k: 1 }, start: 0, dur: 600 });
    const rafRef = useRef<number | null>(null);
    const panRef = useRef<{
      pointerId: number;
      startX: number;
      startY: number;
      camX: number;
      camY: number;
      lastX: number;
      lastY: number;
    } | null>(null);
    const panningRef = useRef(false);
    const viewportRef = useRef({ w: 0, h: 0 });
    const rectRef = useRef({ left: 0, top: 0 });
    const hasFitRef = useRef(false);

    const [hoveredId, setHoveredId] = useState<string | null>(null);

    /* The frame loop touches refs only, so it is defined once and stays
       stable; both it and `schedule` are plain stable callbacks. */
    const runFrame = useCallback((now: number) => {
      rafRef.current = null;
      const mode = modeRef.current;
      if (mode === "idle") return;
      const cam = camRef.current;
      const target = targetRef.current;

      if (mode === "snap") {
        /* Drag writes the camera directly — target is stale. Just flush
           this frame and stop; the next pointermove re-schedules. */
        modeRef.current = "idle";
      } else if (mode === "momentum") {
        cam.x += velRef.current.x;
        cam.y += velRef.current.y;
        velRef.current.x *= MOMENTUM_DECAY;
        velRef.current.y *= MOMENTUM_DECAY;
        if (Math.abs(velRef.current.x) < MOMENTUM_STOP && Math.abs(velRef.current.y) < MOMENTUM_STOP) {
          modeRef.current = "idle";
        }
      } else if (mode === "smooth") {
        const f = 0.24;
        cam.x += (target.x - cam.x) * f;
        cam.y += (target.y - cam.y) * f;
        cam.k += (target.k - cam.k) * f;
        if (Math.abs(target.x - cam.x) < 0.4 && Math.abs(target.y - cam.y) < 0.4 && Math.abs(target.k - cam.k) < 0.002) {
          cam.x = target.x;
          cam.y = target.y;
          cam.k = target.k;
          modeRef.current = "idle";
        }
      } else if (mode === "tween") {
        const t = Math.min((now - tweenRef.current.start) / tweenRef.current.dur, 1);
        const e = 1 - Math.pow(1 - t, 3);
        const from = tweenRef.current.from;
        cam.x = from.x + (target.x - from.x) * e;
        cam.y = from.y + (target.y - from.y) * e;
        cam.k = from.k + (target.k - from.k) * e;
        if (t >= 1) {
          cam.x = target.x;
          cam.y = target.y;
          cam.k = target.k;
          modeRef.current = "idle";
        }
      }

      const layer = layerRef.current;
      if (layer) {
        layer.style.transform = `translate3d(${cam.x.toFixed(2)}px, ${cam.y.toFixed(2)}px, 0) scale(${cam.k})`;
      }
      if (zoomLabelRef.current) {
        zoomLabelRef.current.textContent = `${Math.round(cam.k * 100)}%`;
      }
      if (modeRef.current !== "idle") {
        rafRef.current = raf(runFrame);
      }
    }, []);

    const schedule = useCallback(() => {
      if (rafRef.current != null) return;
      rafRef.current = raf(runFrame);
    }, [runFrame]);

    /* ---- Focus: hover wins over search, search wins over selection;
            search dims the whole canvas down to its matches. ---------- */
    const focus = useMemo<GraphFocus>(() => {
      if (!hoveredId && !selectedId && !searchActive) return EMPTY_FOCUS;
      const activeId = hoveredId ?? (searchActive ? null : selectedId);
      const related = new Set<string>();
      if (activeId) {
        for (const e of edges) {
          if (e.source === activeId) related.add(e.target);
          if (e.target === activeId) related.add(e.source);
        }
      }
      return { activeId, relatedIds: related, searching: searchActive, searchMatches };
    }, [hoveredId, selectedId, edges, searchActive, searchMatches]);

    const placedMap = useMemo(() => {
      const map = new Map<string, (typeof layout.placed)[number]>();
      for (const p of layout.placed) map.set(p.node.id, p);
      return map;
    }, [layout]);

    /* ---- Edge layer — one memoized SVG pass, world coordinates. ---- */
    const edgeLayer = useMemo(() => {
      const { minX, minY } = layout.bounds;
      const fid = focus.activeId;
      return edges.map((edge) => {
        const s = placedMap.get(edge.source);
        const t = placedMap.get(edge.target);
        if (!s || !t) return null;
        const a = anchorToward(s, t);
        const b = anchorToward(t, s);
        const d = edgePath(a, b, edge.id, minX, minY);
        const style = EDGE_STYLE[edge.tier];
        const incident = fid !== null && (edge.source === fid || edge.target === fid);
        const dimmed = fid !== null && !incident;
        const width = incident ? style.fw : style.w;
        const opacity = dimmed ? style.o * 0.2 : incident ? style.fo : style.o;
        const cls = `ctx-edge ctx-edge--${edge.tier}`;
        return (
          <g key={edge.id}>
            {incident && <path d={d} className={cls} strokeWidth={width * 3.4} opacity={0.13} />}
            <path d={d} className={cls} strokeWidth={width} opacity={opacity} />
          </g>
        );
      });
    }, [edges, placedMap, layout, focus.activeId]);

    /* ---- Camera actions. ------------------------------------------- */

    const fitToView = useCallback(
      (animate = true) => {
        const { w, h } = viewportRef.current;
        if (w < 40 || h < 40) return;
        const { minX, minY, maxX, maxY } = layout.bounds;
        const bw = maxX - minX;
        const bh = maxY - minY;
        if (bw <= 0 || bh <= 0) return;
        const k = Math.min(Math.max(Math.min(w / bw, h / bh) * FIT_PADDING_RATIO, MIN_K), MAX_K);
        const cx = (minX + maxX) / 2;
        const cy = (minY + maxY) / 2;
        const target = { x: w / 2 - cx * k, y: h / 2 - cy * k, k };
        const cam = camRef.current;
        targetRef.current = target;
        if (!animate) {
          cam.x = target.x;
          cam.y = target.y;
          cam.k = target.k;
          modeRef.current = "idle";
          const layer = layerRef.current;
          if (layer) layer.style.transform = `translate3d(${cam.x.toFixed(2)}px, ${cam.y.toFixed(2)}px, 0) scale(${cam.k})`;
          if (zoomLabelRef.current) zoomLabelRef.current.textContent = `${Math.round(cam.k * 100)}%`;
          return;
        }
        tweenRef.current = { from: { ...cam }, start: performance.now(), dur: 680 };
        modeRef.current = "tween";
        schedule();
      },
      [layout, schedule],
    );

    const flyToNode = useCallback(
      (id: string, scale = 1.55) => {
        const p = placedMap.get(id);
        if (!p) return;
        const { w, h } = viewportRef.current;
        if (w < 40 || h < 40) return;
        const k = Math.min(Math.max(scale, MIN_K), MAX_K);
        const cam = camRef.current;
        targetRef.current = { x: w / 2 - p.x * k, y: h / 2 - p.y * k, k };
        tweenRef.current = { from: { ...cam }, start: performance.now(), dur: 620 };
        modeRef.current = "tween";
        schedule();
      },
      [placedMap, schedule],
    );

    useImperativeHandle(
      forwardedRef,
      () => ({
        flyToNode,
        fitToView: () => fitToView(true),
      }),
      [flyToNode, fitToView],
    );

    const zoomAt = useCallback(
      (clientX: number, clientY: number, factor: number) => {
        const cam = camRef.current;
        const rect = rectRef.current;
        const sx = clientX - rect.left;
        const sy = clientY - rect.top;
        const wx = (sx - cam.x) / cam.k;
        const wy = (sy - cam.y) / cam.k;
        const k2 = Math.min(Math.max(cam.k * factor, MIN_K), MAX_K);
        targetRef.current = { x: sx - wx * k2, y: sy - wy * k2, k: k2 };
        modeRef.current = "smooth";
        schedule();
      },
      [schedule],
    );

    /* ---- Measure + fit on mount, then track resizes. ---------------- */
    useEffect(() => {
      const el = containerRef.current;
      if (!el) return;

      const measure = () => {
        const rect = el.getBoundingClientRect();
        const w = rect.width || el.clientWidth;
        const h = rect.height || el.clientHeight;
        viewportRef.current = { w, h };
        rectRef.current = { left: rect.left, top: rect.top };
        if (!hasFitRef.current && w > 40 && h > 40) {
          hasFitRef.current = true;
          fitToView(true);
        }
      };

      measure();
      if (typeof ResizeObserver !== "undefined") {
        const ro = new ResizeObserver(measure);
        ro.observe(el);
        return () => ro.disconnect();
      }
      return undefined;
    }, [fitToView]);

    /* ---- Wheel zoom — native non-passive listener so preventDefault
            actually stops the page from scrolling. -------------------- */
    useEffect(() => {
      const el = containerRef.current;
      if (!el) return;
      const onWheel = (e: WheelEvent) => {
        e.preventDefault();
        zoomAt(e.clientX, e.clientY, Math.exp(-e.deltaY * 0.0016));
      };
      el.addEventListener("wheel", onWheel, { passive: false });
      return () => el.removeEventListener("wheel", onWheel);
    }, [zoomAt]);

    useEffect(() => () => {
      /* Release the frame id as well as cancelling it, and re-arm the
         initial fit: React StrictMode (dev) mounts → cleans up →
         remounts, and both the `schedule` guard and the one-shot
         `hasFitRef` flag would otherwise leave the remount with a dead
         camera (no fit tween, no pan/zoom). */
      if (rafRef.current != null) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
      hasFitRef.current = false;
    }, []);

    /* ---- Canvas pointer interactions: pan + momentum. --------------- */
    const handlePointerDown = useCallback(
      (e: React.PointerEvent<HTMLDivElement>) => {
        if (e.button !== 0) return;
        const el = containerRef.current;
        panRef.current = {
          pointerId: e.pointerId,
          startX: e.clientX,
          startY: e.clientY,
          camX: camRef.current.x,
          camY: camRef.current.y,
          lastX: e.clientX,
          lastY: e.clientY,
        };
        panningRef.current = true;
        setHoveredId(null);
        velRef.current = { x: 0, y: 0 };
        el?.classList.add("cursor-grabbing");
        el?.classList.remove("cursor-grab");
        if (el && typeof el.setPointerCapture === "function") {
          try {
            el.setPointerCapture(e.pointerId);
          } catch {
            /* best-effort */
          }
        }
      },
      [],
    );

    const handlePointerMove = useCallback(
      (e: React.PointerEvent<HTMLDivElement>) => {
        const pan = panRef.current;
        if (!pan || pan.pointerId !== e.pointerId) return;
        const cam = camRef.current;
        cam.x = pan.camX + (e.clientX - pan.startX);
        cam.y = pan.camY + (e.clientY - pan.startY);
        velRef.current = { x: e.clientX - pan.lastX, y: e.clientY - pan.lastY };
        pan.lastX = e.clientX;
        pan.lastY = e.clientY;
        modeRef.current = "snap";
        schedule();
      },
      [schedule],
    );

    const endPan = useCallback(
      (e: React.PointerEvent<HTMLDivElement>) => {
        const pan = panRef.current;
        if (!pan || pan.pointerId !== e.pointerId) return;
        panRef.current = null;
        panningRef.current = false;
        const el = containerRef.current;
        el?.classList.remove("cursor-grabbing");
        el?.classList.add("cursor-grab");
        if (Math.abs(velRef.current.x) > 2 || Math.abs(velRef.current.y) > 2) {
          modeRef.current = "momentum";
          schedule();
        }
      },
      [schedule],
    );

    const handleSelect = useCallback(
      (id: string) => {
        if (panningRef.current) return;
        onSelect(id);
      },
      [onSelect],
    );

    const handleNodeDoubleClick = useCallback(
      (id: string) => {
        flyToNode(id, 1.55);
      },
      [flyToNode],
    );

    const handleHoverStart = useCallback((id: string) => {
      if (panningRef.current) return;
      setHoveredId(id);
    }, []);
    const handleHoverEnd = useCallback(() => setHoveredId(null), []);

    const zoomBy = useCallback(
      (factor: number) => {
        const { w, h } = viewportRef.current;
        zoomAt(rectRef.current.left + w / 2, rectRef.current.top + h / 2, factor);
      },
      [zoomAt],
    );

    const { minX, minY, maxX, maxY } = layout.bounds;
    const spanX = maxX - minX;
    const spanY = maxY - minY;

    return (
      <div
        ref={containerRef}
        className={cn("relative h-full w-full touch-none overflow-hidden cursor-grab select-none", className)}
        style={{ userSelect: "none", WebkitUserSelect: "none" }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={endPan}
        onPointerCancel={endPan}
        onPointerLeave={endPan}
        onKeyDown={(e) => {
          if (e.key === "Escape") onDeselect();
        }}
        tabIndex={0}
        onDoubleClick={() => fitToView(true)}
      >
        {/* Screen-space atmosphere over the whole scene. */}
        <div className="pointer-events-none absolute inset-0 bg-vignette" aria-hidden="true" />

        {/* Entrance fade — opacity only; the camera transform lives on
            the inner layer so the two never fight. */}
        <div className="absolute inset-0 animate-fade-in">
          <div
            ref={layerRef}
            className="absolute left-0 top-0"
            style={{ willChange: "transform" }}
          >
            {/* World-space ambient light the node glass frosts + bends. */}
            <div
              aria-hidden="true"
              className="pointer-events-none absolute bg-worldglow"
              style={{ left: -800, top: -800, width: 1600, height: 1600 }}
            />
            <div
              aria-hidden="true"
              className="pointer-events-none absolute bg-dotgrid"
              style={{ left: minX - 900, top: minY - 900, width: spanX + 1800, height: spanY + 1800, opacity: 0.05 }}
            />

            <svg
              aria-hidden="true"
              className="pointer-events-none absolute overflow-visible"
              style={{ left: minX, top: minY, width: spanX, height: spanY }}
            >
              {edgeLayer}
            </svg>

            {layout.placed.map((p) => (
              <ContextNodeCard
                key={p.node.id}
                placed={p}
                focus={focus}
                isSelected={selectedId === p.node.id}
                onSelect={handleSelect}
                onDoubleClick={handleNodeDoubleClick}
                onHoverStart={handleHoverStart}
                onHoverEnd={handleHoverEnd}
              />
            ))}
          </div>
        </div>

        {/* Zoom controls — canvas chrome: swallow pointer/double-click
            events so buttons never start a pan or refit. */}
        <div
          className="glass-control absolute bottom-5 left-5 z-20 flex flex-col items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border) p-1"
          onPointerDown={(e) => e.stopPropagation()}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          <button
            onClick={() => zoomBy(1.3)}
            className="rounded-[var(--radius-control)] p-2 text-(--color-muted-foreground) transition-all duration-100 ease-out hover:bg-(--color-surface-hover) hover:text-(--color-foreground) motion-safe:active:scale-[0.97]"
            title="Zoom in"
            aria-label="Zoom in"
          >
            <ZoomIn className="h-4 w-4" strokeWidth={1.75} />
          </button>
          <span
            ref={zoomLabelRef}
            className="font-(family-name:--font-mono) text-[10px] tabular-nums text-(--color-muted-foreground)"
          >
            100%
          </span>
          <button
            onClick={() => zoomBy(1 / 1.3)}
            className="rounded-[var(--radius-control)] p-2 text-(--color-muted-foreground) transition-all duration-100 ease-out hover:bg-(--color-surface-hover) hover:text-(--color-foreground) motion-safe:active:scale-[0.97]"
            title="Zoom out"
            aria-label="Zoom out"
          >
            <ZoomOut className="h-4 w-4" strokeWidth={1.75} />
          </button>
          <button
            onClick={() => fitToView(true)}
            className="rounded-[var(--radius-control)] p-2 text-(--color-muted-foreground) transition-all duration-100 ease-out hover:bg-(--color-surface-hover) hover:text-(--color-foreground) motion-safe:active:scale-[0.97]"
            title="Fit to view"
            aria-label="Fit to view"
          >
            <Maximize className="h-4 w-4" strokeWidth={1.75} />
          </button>
        </div>
      </div>
    );
  },
);