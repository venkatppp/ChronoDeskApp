import type { ContextLayout, ContextNode, ContextTier, PlacedNode } from "./types";

/**
 * Deterministic radial layout for the workspace context graph.
 *
 * The entry node sits at the origin; active-context nodes ring it at
 * radius 330; wider-workspace artifacts ring farther out at radius 560,
 * placed in the angular gaps between active spokes so hub lines and
 * sector-local curves never cross. Angles are hand-tuned per artifact,
 * then nudged by a tiny per-node jitter (seeded, so the layout is stable
 * across renders and never looks mechanical).
 */

const NODE_SIZE: Record<ContextTier, { w: number; h: number }> = {
  foreground: { w: 232, h: 96 },
  active: { w: 220, h: 62 },
  background: { w: 140, h: 48 },
};

const RING_RADIUS: Record<ContextTier, number> = {
  foreground: 0,
  active: 330,
  background: 560,
};

/** Hand-tuned angular sectors (degrees, clockwise from top). Background
 *  artifacts occupy the gaps between the active spokes. */
const ANGLES: Record<string, number> = {
  "App.tsx": 0,
  "components/Header.tsx": -90,
  "components/Sidebar.tsx": -45,
  "components/Dashboard.tsx": 0,
  "services/api.ts": 45,
  "services/auth.ts": 90,
  "hooks/useWorkspace.ts": 135,
  "styles/theme.css": 180,
  "tests/App.test.tsx": -135,
  "README.md": -112.5,
  "docs/architecture.md": -67.5,
  "package.json": -22.5,
  "documentation/": 22.5,
  "repositories/": 67.5,
  "screenshots/": 112.5,
  "tests/": 157.5,
};

/* Small deterministic jitter so the rings read organic, not plotted. */
function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function jitterFor(id: string): () => number {
  let seed = 2166136261;
  for (let i = 0; i < id.length; i++) {
    seed ^= id.charCodeAt(i);
    seed = Math.imul(seed, 16777619);
  }
  return mulberry32(seed);
}

function placedAt(node: ContextNode, angleDeg: number, radius: number): PlacedNode {
  const { w, h } = NODE_SIZE[node.tier];
  const rand = jitterFor(node.id);
  const a = ((angleDeg + (rand() - 0.5) * 5) * Math.PI) / 180;
  const r = radius + (rand() - 0.5) * 26;
  return { node, x: Math.cos(a) * r, y: Math.sin(a) * r, w, h };
}

export function computeContextLayout(nodes: ContextNode[]): ContextLayout {
  const placed = nodes.map((n) => placedAt(n, ANGLES[n.id] ?? 0, RING_RADIUS[n.tier]));

  const pad = 60;
  const minX = Math.min(...placed.map((p) => p.x - p.w / 2)) - pad;
  const minY = Math.min(...placed.map((p) => p.y - p.h / 2)) - pad;
  const maxX = Math.max(...placed.map((p) => p.x + p.w / 2)) + pad;
  const maxY = Math.max(...placed.map((p) => p.y + p.h / 2)) + pad;

  return { placed, bounds: { minX, minY, maxX, maxY } };
}

/** Anchor point where a line from `target` meets the edge of `source`
 *  (rect approximation — the curves tuck under the node borders instead
 *  of disappearing beneath them). */
export function anchorToward(source: PlacedNode, target: PlacedNode): { x: number; y: number } {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  if (dx === 0 && dy === 0) return { x: source.x, y: source.y };
  const halfW = source.w / 2;
  const halfH = source.h / 2;
  const sx = Math.abs(dx) / halfW;
  const sy = Math.abs(dy) / halfH;
  if (sx > sy) {
    const t = halfW / Math.abs(dx);
    return { x: source.x + dx * t, y: source.y + dy * t };
  }
  const t = halfH / Math.abs(dy);
  return { x: source.x + dx * t, y: source.y + dy * t };
}