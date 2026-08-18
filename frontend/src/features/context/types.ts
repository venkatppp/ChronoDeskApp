/**
 * Workspace Context Graph — data model.
 *
 * The graph is deliberately hub-and-spoke around the workspace entry
 * point: one foreground node (App.tsx), a ring of active context
 * (components, services, hooks, styles, tests), and a distant ring of
 * wider workspace artifacts (docs, config, folders). Tiers encode the
 * visual depth of the scene: foreground → middle → background.
 */

export type ContextTier = "foreground" | "active" | "background";

export type ContextNodeKind =
  | "entry"
  | "component"
  | "service"
  | "hook"
  | "style"
  | "test"
  | "doc"
  | "config"
  | "folder";

export interface ContextNode {
  /** Stable id — the artifact's path (e.g. "components/Header.tsx"). */
  id: string;
  /** Short display name (file name or folder name). */
  label: string;
  /** Full path relative to the workspace root. */
  path: string;
  kind: ContextNodeKind;
  tier: ContextTier;
  /** Role caption shown under the label, e.g. "Component", "Service". */
  role: string;
  /** Optional second caption line (used by the entry node). */
  detail?: string;
}

/** Relationship weight tier — maps directly to line styling. */
export type ContextEdgeTier = "primary" | "secondary" | "faint";

export interface ContextEdge {
  id: string;
  source: string;
  target: string;
  tier: ContextEdgeTier;
}

export interface PlacedNode {
  node: ContextNode;
  /** World-space position (center) and size. */
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ContextLayout {
  placed: PlacedNode[];
  bounds: { minX: number; minY: number; maxX: number; maxY: number };
}

/** Visual emphasis of a node given the current graph focus. */
export type NodeEmphasis = "default" | "focus" | "related" | "match" | "dimmed";

export interface GraphFocus {
  /** Node currently under the pointer or selected (hover wins). */
  activeId: string | null;
  relatedIds: Set<string>;
  /** Search is active; `searchMatches` holds the matching node ids. */
  searching: boolean;
  searchMatches: Set<string>;
}

export const EMPTY_FOCUS: GraphFocus = {
  activeId: null,
  relatedIds: new Set(),
  searching: false,
  searchMatches: new Set(),
};