import { memo, useCallback, useRef } from "react";
import {
  AppWindow,
  Braces,
  Component,
  FileJson,
  FileText,
  FlaskConical,
  Folder,
  Palette,
  Server,
} from "lucide-react";
import type { ContextNode, ContextNodeKind, GraphFocus, NodeEmphasis, PlacedNode } from "./types";
import { cn } from "@/utils/cn";

/**
 * One glass node in the workspace context graph.
 *
 * Rendered in world coordinates inside the camera layer — the camera
 * transform moves it, so this component never re-renders during pan/zoom.
 * Emphasis is derived from a memoized `focus` object (same reference for
 * every unchanged frame), which keeps hover/selection re-renders O(1)
 * per interaction instead of per pointer move.
 */

const KIND_ICON: Record<ContextNodeKind, typeof AppWindow> = {
  entry: AppWindow,
  component: Component,
  service: Server,
  hook: Braces,
  style: Palette,
  test: FlaskConical,
  doc: FileText,
  config: FileJson,
  folder: Folder,
};

/* Muted blue-gray tile tints — the palette stays atmospheric, never neon. */
const KIND_TONE: Record<ContextNodeKind, { bg: string; fg: string }> = {
  entry: { bg: "rgba(10, 132, 255, 0.24)", fg: "#6fb4ff" },
  component: { bg: "rgba(140, 170, 210, 0.16)", fg: "#93b6dd" },
  service: { bg: "rgba(130, 160, 200, 0.16)", fg: "#86a9cf" },
  hook: { bg: "rgba(150, 170, 210, 0.16)", fg: "#9bb1d6" },
  style: { bg: "rgba(158, 150, 210, 0.15)", fg: "#a49cd8" },
  test: { bg: "rgba(120, 190, 170, 0.15)", fg: "#7ec7ab" },
  doc: { bg: "rgba(150, 160, 180, 0.15)", fg: "#9ca6b8" },
  config: { bg: "rgba(172, 152, 132, 0.15)", fg: "#b39d87" },
  folder: { bg: "rgba(160, 160, 170, 0.13)", fg: "#9ba2ae" },
};

const TIER_CLASS: Record<string, string> = {
  foreground: "context-node--tier-fg",
  active: "context-node--tier-mid",
  background: "context-node--tier-bg",
};

function emphasisOf(node: ContextNode, focus: GraphFocus): NodeEmphasis {
  if (focus.activeId) {
    if (node.id === focus.activeId) return "focus";
    if (focus.relatedIds.has(node.id)) return "related";
    return "dimmed";
  }
  if (focus.searching) {
    return focus.searchMatches.has(node.id) ? "match" : "dimmed";
  }
  return "default";
}

const CLICK_SLOP_PX = 6;

interface ContextNodeCardProps {
  placed: PlacedNode;
  focus: GraphFocus;
  isSelected: boolean;
  onSelect: (id: string) => void;
  onDoubleClick: (id: string) => void;
  onHoverStart: (id: string) => void;
  onHoverEnd: () => void;
}

export const ContextNodeCard = memo(function ContextNodeCard({
  placed,
  focus,
  isSelected,
  onSelect,
  onDoubleClick,
  onHoverStart,
  onHoverEnd,
}: ContextNodeCardProps) {
  const { node, x, y, w, h } = placed;
  const em = emphasisOf(node, focus);
  const tone = KIND_TONE[node.kind];
  const Icon = KIND_ICON[node.kind];

  /* Click-vs-drag discrimination lives in refs — no state churn. */
  const gestureRef = useRef<{ pointerId: number; x: number; y: number; moved: boolean } | null>(null);

  const handlePointerDown = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      e.stopPropagation();
      gestureRef.current = { pointerId: e.pointerId, x: e.clientX, y: e.clientY, moved: false };
      const el = e.currentTarget;
      if (typeof el.setPointerCapture === "function") {
        try {
          el.setPointerCapture(e.pointerId);
        } catch {
          /* capture is best-effort */
        }
      }
    },
    [],
  );

  const handlePointerMove = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    const g = gestureRef.current;
    if (!g || g.pointerId !== e.pointerId) return;
    const dx = e.clientX - g.x;
    const dy = e.clientY - g.y;
    if (dx * dx + dy * dy > CLICK_SLOP_PX * CLICK_SLOP_PX) g.moved = true;
  }, []);

  const handlePointerUp = useCallback(
    (e: React.PointerEvent<HTMLDivElement>) => {
      const g = gestureRef.current;
      if (!g || g.pointerId !== e.pointerId) return;
      gestureRef.current = null;
      e.stopPropagation();
      if (!g.moved) onSelect(node.id);
    },
    [node.id, onSelect],
  );

  const handleEnter = useCallback(() => onHoverStart(node.id), [node.id, onHoverStart]);
  const handleLeave = useCallback(() => onHoverEnd(), [onHoverEnd]);
  const handleDbl = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onDoubleClick(node.id);
    },
    [node.id, onDoubleClick],
  );

  const isEntry = node.tier === "foreground";

  return (
    <div
      role="button"
      aria-label={`${node.path} · ${node.role}`}
      data-context-node
      data-emphasis={em}
      data-selected={isSelected || undefined}
      className={cn("context-node", TIER_CLASS[node.tier], `context-node--em-${em}`, isSelected && "context-node--selected")}
      style={{
        left: x - w / 2,
        top: y - h / 2,
        width: w,
        height: h,
        padding: isEntry ? "0 18px" : "0 14px",
        borderRadius: isEntry ? 22 : 17,
      }}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerEnter={handleEnter}
      onPointerLeave={handleLeave}
      onDoubleClick={handleDbl}
    >
      <span
        aria-hidden="true"
        className="flex shrink-0 items-center justify-center rounded-xl"
        style={{
          width: isEntry ? 38 : 30,
          height: isEntry ? 38 : 30,
          backgroundColor: tone.bg,
          boxShadow: "inset 0 1px 0 rgba(255,255,255,0.14)",
        }}
      >
        <Icon
          className={isEntry ? "h-[18px] w-[18px]" : "h-[15px] w-[15px]"}
          strokeWidth={1.75}
          style={{ color: tone.fg }}
        />
      </span>
      <span className="flex min-w-0 flex-col justify-center">
        <span
          className="truncate font-(family-name:--font-display) font-semibold text-(--color-foreground)"
          style={{ fontSize: isEntry ? 15 : 11.5, letterSpacing: isEntry ? -0.01 : 0 }}
        >
          {node.label}
        </span>
        <span className="truncate text-(--color-faint-foreground)" style={{ fontSize: isEntry ? 10.5 : 10, lineHeight: 1.45 }}>
          {isEntry && node.detail ? node.detail : node.role}
        </span>
        {isEntry && node.detail && (
          <span className="truncate text-(--color-faint-foreground)" style={{ fontSize: 10, lineHeight: 1.45 }}>
            {node.role}
          </span>
        )}
      </span>
      {isSelected && (
        <span
          aria-hidden="true"
          className="absolute right-2 top-2 h-1.5 w-1.5 rounded-full bg-(--color-accent)"
          style={{ boxShadow: "0 0 8px rgba(10,132,255,0.7)" }}
        />
      )}
    </div>
  );
});