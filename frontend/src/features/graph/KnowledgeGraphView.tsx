import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import type { KgNode, KgEdge, GraphNodeType, GraphRelationshipType } from "@/types/graph";
import {
  ZoomIn,
  ZoomOut,
  Maximize,
  Focus,
  Search,
  X,
  Keyboard,
  Waypoints,
  LocateFixed,
  ChevronRight,
  Folder,
  File as FileIcon,
  Boxes,
} from "lucide-react";
import { GlassSurface } from "@/components/ui/GlassSurface";

export type GraphMode = "structure" | "activity" | "semantic";

const MODE_LABELS: Record<GraphMode, string> = {
  structure: "Structure",
  activity: "Activity",
  semantic: "Semantic",
};

interface PositionedNode extends KgNode {
  x: number;
  y: number;
  vx: number;
  vy: number;
}

/* ------------------------------------------------------------------ *
 * Type visual language — each entity type owns a hue and an icon.
 * ------------------------------------------------------------------ */
const NODE_RADIUS: Record<GraphNodeType, number> = {
  workspace: 38,
  file: 25,
  planner_report: 23,
  execution: 29,
  memory_record: 26,
  autonomous_session: 31,
};

/* Calm semantic palette — macOS-muted hues, no neon. Workspace = system
   blue, file = steel blue, reports = green, runs = muted orange,
   memory/semantic = violet. Saturation stays low enough for the canvas
   to read as an environment, not a command center. */
const NODE_COLORS: Record<GraphNodeType, string> = {
  workspace: "#4d9fff",
  file: "#7fa9c4",
  planner_report: "#63c98f",
  execution: "#d9a05b",
  memory_record: "#a78bdc",
  autonomous_session: "#b39ddb",
};

const NODE_RING: Record<string, number> = {
  workspace: 0.45,
  execution: 0.7,
  autonomous_session: 1.05,
  file: 0.92,
  planner_report: 1.28,
  memory_record: 1.55,
};

const EDGE_COLORS: Record<GraphRelationshipType, string> = {
  contains: "#5b6472",
  runs_in: "#4d9fff",
  reports_on: "#d9a05b",
  derived_from: "#63c98f",
  related_to: "#a78bdc",
};

const EDGE_LABELS: Record<GraphRelationshipType, string> = {
  contains: "Contains",
  runs_in: "Runs in",
  reports_on: "Reports on",
  derived_from: "Derived from",
  related_to: "Related to",
};

const REPULSION_STRENGTH = 16000;
const COLLISION_RADIUS = 36;
const EDGE_LENGTH = 190;

function nodeRadius(type: GraphNodeType): number {
  return NODE_RADIUS[type] ?? 24;
}

function nodeColor(type: GraphNodeType): string {
  return NODE_COLORS[type] ?? "#4d9fff";
}

function edgeColor(type: GraphRelationshipType): string {
  return EDGE_COLORS[type] ?? "#565664";
}

/* Deterministic RNG so the layout is identical across re-renders. */
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

function runSimulation(nodes: KgNode[], edges: KgEdge[], width: number, height: number): PositionedNode[] {
  if (nodes.length === 0) return [];

  let seed = 7;
  for (const n of nodes.slice(0, 6)) {
    for (let i = 0; i < n.entityId.length; i++) seed = (seed * 31 + n.entityId.charCodeAt(i)) | 0;
  }
  for (const e of edges.slice(0, 12)) seed = (seed * 33 + (e.id?.length ?? 0)) | 0;
  const rand = mulberry32(Math.abs(seed) + 1);

  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(width, height) * 0.24;

  const positioned: PositionedNode[] = nodes.map((n) => {
    const ring = (NODE_RING[n.nodeType] ?? 1) * radius;
    const angle = rand() * Math.PI * 2;
    const drift = (rand() - 0.5) * radius * 0.32;
    return {
      ...n,
      x: cx + ring * Math.cos(angle) + drift * Math.cos(angle + 1.7),
      y: cy + ring * Math.sin(angle) * 0.86 + drift * Math.sin(angle + 1.7),
      vx: 0,
      vy: 0,
    };
  });

  const nodeMap = new Map(positioned.map((n) => [nodeKeyOf(n), n] as const));
  const degreeOf = new Map<string, number>();
  for (const edge of edges) {
    const sk = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
    const tk = `${edge.targetNodeType}-${edge.targetEntityId}`;
    degreeOf.set(sk, (degreeOf.get(sk) ?? 0) + 1);
    degreeOf.set(tk, (degreeOf.get(tk) ?? 0) + 1);
  }

  const iterations = Math.min(160, Math.max(90, Math.round(nodes.length * 0.6)));
  for (let iter = 0; iter < iterations; iter++) {
    const alpha = Math.pow(1 - iter / iterations, 1.5);

    for (const node of positioned) {
      let fx = 0;
      let fy = 0;
      const key = `${node.nodeType}-${node.entityId}`;
      const degree = 1 + (degreeOf.get(key) ?? 0);

      for (const other of positioned) {
        if (node === other) continue;
        const dx = node.x - other.x;
        const dy = node.y - other.y;
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const r = nodeRadius(node.nodeType) + nodeRadius(other.nodeType);
        if (dist < r + COLLISION_RADIUS) {
          const push = ((r + COLLISION_RADIUS - dist) / (r + COLLISION_RADIUS)) * 46 * alpha;
          fx += (dx / dist) * push;
          fy += (dy / dist) * push;
        }
        const force = (REPULSION_STRENGTH / (dist * dist)) * (degree / 2) * alpha;
        fx -= (dx / dist) * force;
        fy -= (dy / dist) * force;
      }

      for (const edge of edges) {
        const srcKey = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
        const tgtKey = `${edge.targetNodeType}-${edge.targetEntityId}`;
        let other: PositionedNode | undefined;
        if (srcKey === key) other = nodeMap.get(tgtKey);
        if (tgtKey === key) other = nodeMap.get(srcKey);
        if (!other) continue;
        const dx = other.x - node.x;
        const dy = other.y - node.y;
        const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
        const stretch = (dist - EDGE_LENGTH) / EDGE_LENGTH;
        const a = Math.min(0.035 + edge.weight * 0.03, 0.09);
        fx += (dx / dist) * stretch * a * 60 * alpha;
        fy += (dy / dist) * stretch * a * 60 * alpha;
      }

      const anchor = (NODE_RING[node.nodeType] ?? 0) * radius;
      const theta = Math.atan2(node.y - cy, node.x - cx);
      fx += (cx + anchor * Math.cos(theta) - node.x) * 0.012 * alpha;
      fy += (cy + anchor * Math.sin(theta) - node.y) * 0.012 * alpha;

      node.vx = (node.vx + fx * 0.5 + (cx - node.x) * 0.003) * 0.82;
      node.vy = (node.vy + fy * 0.5 + (cy - node.y) * 0.003) * 0.82;
      node.x += node.vx;
      node.y += node.vy;
    }
  }

  return positioned;
}

const nodeKeyOf = (n: { nodeType: string; entityId: string }) => `${n.nodeType}-${n.entityId}`;

/* ------------------------------------------------------------------ *
 * Structure mode — an expandable, project-accurate tree layout.
 * Groups workspaces, then clusters files by directory derived from
 * their stored path (`summary`), with `contains`-style parent–child
 * links drawn as clean elbow connectors instead of a hairball.
 * ------------------------------------------------------------------ */

type StructureKind = "layer" | "workspace" | "folder" | "file" | "other";

interface StructureNode {
  key: string;
  kind: StructureKind;
  label: string;
  node: KgNode | null;
  tone: string;
  children: StructureNode[];
}

const FILE_TONES: Record<string, string> = {
  react: "#5b9dff",
  rust: "#d9a05b",
  database: "#a78bdc",
  ai: "#63c98f",
  file: "#8fa9c4",
};

export const STRUCTURE_TONES: Record<StructureKind, string> = {
  layer: "#5b9dff",
  workspace: "#5b9dff",
  folder: "#8e8e93",
  file: "#8fa9c4",
  other: "#a3a3ad",
};

function fileToneOf(node: KgNode): string {
  const label = node.title.split("/").pop() ?? "";
  const ext = label.includes(".") ? label.split(".").pop()!.toLowerCase() : "";
  if (ext === "tsx" || ext === "jsx") return FILE_TONES.react;
  if (ext === "rs") return FILE_TONES.rust;
  if (ext === "sql" || ext === "db" || ext === "sqlite") return FILE_TONES.database;
  if (ext === "md" || ext === "json" || ext === "yaml" || ext === "yml" || ext === "toml") return FILE_TONES.ai;
  return FILE_TONES.file;
}

function fileLabel(node: KgNode): string {
  return (node.title ?? "").split("/").pop() || node.title;
}

interface StructureBuild {
  signature: string;
  roots: StructureNode[];
  defaultExpanded: string[];
}

/** Strip the shared absolute prefix so files render relative to the
 *  project root; keeps the last 5 segments as a ceiling for deep trees. */
function relativePathFor(paths: string[]): (p: string) => string[] {
  const segPaths = paths
    .filter((p): p is string => Boolean(p))
    .map((p) => p.split("/").filter(Boolean));
  if (segPaths.length === 0) return (p) => p.split("/").filter(Boolean);
  let common = 0;
  for (let i = 0; i < segPaths[0].length; i++) {
    const seg = segPaths[0][i];
    if (segPaths.every((sp) => sp[common] === seg)) common++;
    else break;
  }
  return (p) => {
    let segs = p.split("/").filter(Boolean).slice(common);
    if (segs.length > 5) segs = segs.slice(segs.length - 5);
    return segs;
  };
}

function buildStructure(nodes: KgNode[]): StructureBuild {
  const workspaces = nodes.filter((n) => n.nodeType === "workspace");
  const files = nodes.filter((n) => n.nodeType === "file");
  const others = nodes.filter(
    (n) => n.nodeType !== "workspace" && n.nodeType !== "file",
  );

  const mkFile = (parentKey: string, f: KgNode): StructureNode => ({
    key: `${parentKey}/${nodeKeyOf(f)}`,
    kind: "file",
    label: fileLabel(f),
    node: f,
    tone: fileToneOf(f),
    children: [],
  });

  const addFile = (root: StructureNode, segs: string[], file: KgNode) => {
    let cur = root;
    for (let i = 0; i < segs.length - 1; i++) {
      const seg = segs[i];
      let child = cur.children.find((c) => c.kind === "folder" && c.label === seg);
      if (!child) {
        child = { key: `${cur.key}/${seg}`, kind: "folder", label: seg, node: null, tone: STRUCTURE_TONES.folder, children: [] };
        cur.children.push(child);
      }
      cur = child;
    }
    cur.children.push(mkFile(cur.key, file));
  };

  const mkWorkspaceTree = (wsNode: KgNode | null, wsId: string, label: string, key: string): StructureNode => {
    const groupFiles = files.filter((f) => f.workspaceId === wsId);
    const groupOthers = others.filter((o) => o.workspaceId === wsId);
    const rel = relativePathFor(
      groupFiles.map((f) => f.summary || f.title),
    );
    const root: StructureNode = { key, kind: "workspace", label, node: wsNode, tone: STRUCTURE_TONES.workspace, children: [] };
    for (const f of groupFiles) addFile(root, rel(f.summary || f.title), f);
    for (const o of groupOthers) {
      root.children.push({
        key: `${root.key}/${nodeKeyOf(o)}`,
        kind: "other",
        label: o.nodeType.replace("_", " "),
        node: o,
        tone: STRUCTURE_TONES.other,
        children: [],
      });
    }
    return root;
  };

  const clusters: StructureNode[] = [];
  if (workspaces.length > 0) {
    for (const ws of workspaces) {
      clusters.push(mkWorkspaceTree(ws, ws.entityId, ws.title, `workspace-${ws.entityId}`));
    }
    // Files whose workspace node is not in the filtered set still get a home.
    const knownWsIds = new Set(workspaces.map((w) => w.entityId));
    const detached = files.filter((f) => !f.workspaceId || !knownWsIds.has(f.workspaceId));
    for (const f of detached) {
      const rootCursor = clusters[clusters.length - 1];
      if (rootCursor) rootCursor.children.push(mkFile(rootCursor.key, f));
    }
  } else {
    const root = mkWorkspaceTree(null, "__root__", "Files", "files-root");
    for (const f of files) {
      if (!root.children.some((c) => c.kind === "file" && c.label === fileLabel(f))) {
        root.children.push(mkFile(root.key, f));
      }
    }
    clusters.push(root);
  }

  const layer: StructureNode = {
    key: "__layer__",
    kind: "layer",
    label: clusters.length === 1 && workspaces.length === 0 ? "Files" : "Workspace Layer",
    node: null,
    tone: STRUCTURE_TONES.layer,
    children: clusters,
  };

  const defaultExpanded = [layer.key, ...layer.children.map((c) => c.key)];
  // First-level folder children (e.g. frontend / src-tauri) stay collapsed
  // until clicked — progressive disclosure from the very first paint.

  const signature = workspaces
    .map((w) => w.entityId)
    .slice(0, 40)
    .join(",") + "|" + files.length;

  return { signature, roots: [layer], defaultExpanded };
}

/** Visible children honoring the expansion set; collapsed folders render
 *  as leaves (with a badge) so large sub-trees never flood the canvas. */
function visibleChildren(n: StructureNode, expanded: Set<string>): StructureNode[] {
  if (expanded.has(n.key)) return n.children;
  return [];
}

interface PlacedSNode extends StructureNode {
  x: number;
  y: number;
  w: number;
  h: number;
  depth: number;
  clusterMinY: number;
  clusterMaxY: number;
  clusterMaxDepth: number;
  leafCount: number;
}

interface StructureLayout {
  placed: PlacedSNode[];
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
}

const COL_WIDTH = 236;
const PAD_X = 64;
const PAD_Y = 44;
const ROW_SPACING = 40;
const MIN_CLUSTER_GAP = 18;

function computeStructureLayout(
  roots: StructureNode[],
  expanded: Set<string>,
  width: number,
  height: number,
): StructureLayout {
  const countLeaves = (n: StructureNode): number => {
    const kids = visibleChildren(n, expanded);
    if (kids.length === 0) return 1;
    return kids.reduce((s, c) => s + countLeaves(c), 0);
  };
  const leaves = roots.reduce((s, r) => s + countLeaves(r), 0);
  const usableH = height - PAD_Y * 2;
  const spacing = leaves > 1 ? Math.max((usableH - MIN_CLUSTER_GAP * 2) / Math.max(leaves - 1, 1), ROW_SPACING) : 0;

  let leafIndex = 0;
  const place = (n: StructureNode, depth: number): PlacedSNode => {
    const kids = visibleChildren(n, expanded);
    let y: number;
    let clusterMinY = y = 0;
    let clusterMaxY = 0;
    let clusterMaxDepth = depth;
    if (kids.length > 0) {
      const placed = kids.map((k) => place(k, depth + 1));
      clusterMinY = Math.min(...placed.map((p) => p.y)) - 22;
      clusterMaxY = Math.max(...placed.map((p) => p.y)) + 22;
      clusterMaxDepth = Math.max(...placed.map((p) => p.clusterMaxDepth));
      y = (Math.min(...placed.map((p) => p.y)) + Math.max(...placed.map((p) => p.y))) / 2;
    } else {
      y = PAD_Y + leafIndex * spacing;
      leafIndex += 1;
    }
    const w = n.kind === "file" || n.kind === "other"
      ? Math.min(96, Math.max(44, n.label.length * 6.4 + 30))
      : Math.min(216, Math.max(88, n.label.length * 6.6 + 70));
    const h = n.kind === "file" || n.kind === "other" ? 26 : 32;
    return {
      ...n,
      x: PAD_X + depth * COL_WIDTH,
      y,
      w,
      h,
      depth,
      clusterMinY,
      clusterMaxY,
      clusterMaxDepth,
      leafCount: leaves,
    };
  };

  const placed = roots.map((r) => place(r, 0));
  const all = flattenPlaced(placed);
  const minX = Math.min(...all.map((p) => p.x - (p.w + 40) / 2));
  const maxX = Math.max(...all.map((p) => p.x + (p.w + 40) / 2));
  const minY = Math.min(...all.map((p) => p.y - p.h / 2));
  const maxY = Math.max(...all.map((p) => p.y + p.h / 2));

  const spanX = maxX - minX;
  const spanY = maxY - minY;
  const shiftX = spanX < width ? (width - spanX) / 2 - minX : PAD_X;
  const shiftY = spanY < height ? (height - spanY) / 2 - minY : Math.max((height - spanY) / 2 - minY, -maxY + height - PAD_Y);

  for (const p of all) {
    p.x += shiftX;
    p.y += shiftY;
    p.clusterMinY += shiftY;
    p.clusterMaxY += shiftY;
  }

  return { placed, minX: minX + shiftX, maxX: maxX + shiftX, minY: minY + shiftY, maxY: maxY + shiftY };
}

function flattenPlaced(placed: PlacedSNode[]): PlacedSNode[] {
  const out: PlacedSNode[] = [];
  const walk = (p: PlacedSNode) => {
    out.push(p);
    for (const c of p.children as unknown as PlacedSNode[]) walk(c);
  };
  for (const p of placed) walk(p);
  return out;
}

function elbowPath(p: { x: number; y: number; w: number }, c: { x: number; y: number; w: number }): string {
  const sx = p.x + p.w / 2;
  const sy = p.y;
  const ex = c.x - c.w / 2;
  const ey = c.y;
  const mx = (sx + ex) / 2;
  return `M ${sx} ${sy} H ${mx} V ${ey} H ${ex}`;
}

function truncateLabel(label: string, max: number): string {
  return label.length > max ? label.slice(0, max - 1) + "\u2026" : label;
}

/** Ancestor keys for a structure key path, e.g. `a/b/c` -> [`a`, `a/b`]. */
function ancestorKeys(key: string): string[] {
  const parts = key.split("/");
  const out: string[] = [];
  for (let i = 1; i < parts.length - 1; i++) out.push(parts.slice(0, i).join("/"));
  return out;
}

/** Walk a structure tree to reveal (expand) the ancestors of a node key. */
function revealAncestors(rootKeys: string[], key: string, expanded: Set<string>): Set<string> {
  const next = new Set(expanded);
  for (const a of ancestorKeys(key)) {
    if (rootKeys.some((r) => a.startsWith(r) || r === a)) next.add(a);
  }
  return next;
}

function renderStructure(
  layout: StructureLayout,
  expanded: Set<string>,
  handleNodeClick: (key: string, kind: StructureKind, node: KgNode | null) => void,
  selectedNodeId?: string,
) {
  const all = flattenPlaced(layout.placed);
  const isExpanded = (n: PlacedSNode) => expanded.has(n.key);
  const hasChildren = (n: PlacedSNode) => n.children.length > 0;

  return (
    <g>
      {/* Cluster panels — one soft panel per expanded folder/workspace. */}
      {all.map((n) => {
        if (!hasChildren(n) || n.depth === 0) return null;
        const color = n.tone;
        const gapX = Math.max((n.clusterMaxDepth - n.depth) * COL_WIDTH - 26, 44);
        const x = n.x + n.w / 2 + 14;
        const w = gapX - 18;
        if (w <= 0) return null;
        return (
          <rect
            key={`cluster-${n.key}`}
            x={x}
            y={n.clusterMinY}
            width={w}
            height={n.clusterMaxY - n.clusterMinY}
            rx={14}
            fill={color}
            opacity={0.045}
            stroke={color}
            strokeOpacity={0.14}
            strokeDasharray="3 5"
            className="pointer-events-none"
          />
        );
      })}

      {/* Elbow connectors between visible parent–child relationships. */}
      {all.map((n) =>
        hasChildren(n) && isExpanded(n)
          ? n.children.map((c) => (
              <path
                key={`edge-${n.key}-${c.key}`}
                d={elbowPath(n, c as unknown as { x: number; y: number; w: number })}
                fill="none"
                stroke={c.tone}
                strokeOpacity={0.4}
                strokeWidth={1.2}
                className="transition-all duration-500 ease-[var(--ease-premium)]"
              />
            ))
          : null,
      )}

      {/* Node pills. */}
      {all.map((n) => {
        const isSel = n.node != null && selectedNodeId === n.node.entityId;
        const canExpand = hasChildren(n);
        const active = canExpand && isExpanded(n);
        const multiple = n.kind === "workspace" || n.kind === "folder" || n.kind === "layer";
        const bg = n.tone + "14";
        const stroke = n.tone;
        const label = truncateLabel(n.label, 27);
        const icon =
          n.kind === "folder" ? (
            <Folder className="h-3.5 w-3.5" strokeWidth={1.75} style={{ color: n.tone }} />
          ) : n.kind === "workspace" || n.kind === "layer" ? (
            <Boxes className="h-4 w-4" strokeWidth={1.75} style={{ color: n.tone }} />
          ) : n.kind === "other" ? (
            <FileIcon className="h-3.5 w-3.5" strokeWidth={1.75} style={{ color: n.tone }} />
          ) : (
            <FileIcon className="h-3.5 w-3.5" strokeWidth={1.75} style={{ color: n.tone }} />
          );

        const childCount = n.children.length;

        return (
          <g
            key={n.key}
            onClick={(e) => {
              e.stopPropagation();
              handleNodeClick(n.key, n.kind, n.node);
            }}
            onDoubleClick={(e) => {
              e.stopPropagation();
              if (canExpand) handleNodeClick(n.key, n.kind, n.node);
            }}
            className="cursor-pointer"
            style={{ transform: `translate(${n.x - n.w / 2}px, ${n.y - n.h / 2}px)`, transition: "transform 0.5s cubic-bezier(0.32,0.08,0.24,1), opacity 0.3s ease" }}
          >
            <rect
              width={n.w}
              height={n.h}
              rx={n.h / 2}
              fill={isSel ? bg : n.tone === "#8e8e93" ? "rgba(8,8,10,0.7)" : bg}
              stroke={isSel ? "#f4f4f6" : stroke}
              strokeOpacity={isSel ? 1 : 0.42}
              strokeWidth={isSel ? 1.5 : 1}
              className="transition-all duration-300 ease-[var(--ease-premium)]"
            />
            <g transform="translate(10, 0)">
              <svg y={n.h / 2 - 7} width="14" height="14" viewBox="0 0 24 24" className="overflow-visible">
                {icon}
              </svg>
            </g>
            <text
              x={30}
              y={n.h / 2 + 0.5}
              fill={n.kind === "folder" && !isSel ? "#c9c9d1" : "#f4f4f6"}
              fontSize={n.kind === "folder" ? 11 : 10.5}
              fontWeight={n.kind === "folder" ? 600 : 550}
              className="pointer-events-none select-none"
              style={{ letterSpacing: 0.1 }}
            >
              {label}
            </text>
            {canExpand && (
              <>
                <circle
                  cx={n.w - 11}
                  cy={n.h / 2}
                  r={9}
                  fill={active ? n.tone : "rgba(255,255,255,0.06)"}
                  stroke={active ? n.tone : "rgba(255,255,255,0.15)"}
                  strokeWidth={1}
                />
                {multiple && (
                  <ChevronRight
                    x={n.w - 16}
                    y={n.h / 2 - 6}
                    width={10}
                    height={12}
                    strokeWidth={2.25}
                    style={{
                      color: active ? "#060609" : n.tone,
                      transform: active ? "rotate(90deg)" : "rotate(0deg)",
                      transition: "transform 0.28s cubic-bezier(0.32,0.08,0.24,1)",
                      transformOrigin: "center",
                    }}
                  />
                )}
                {!multiple && (
                  <text x={n.w - 10.5} y={n.h / 2 + 3.5} textAnchor="middle" fontSize={8} fontWeight={700} fill="#a3a3ad" className="pointer-events-none select-none">
                    {active ? "−" : `${Math.min(childCount, 99)}`}
                  </text>
                )}
              </>
            )}
          </g>
        );
      })}
    </g>
  );
}

function getNeighborIds(node: KgNode, edges: KgEdge[]): Set<string> {
  const ids = new Set<string>();
  const key = nodeKeyOf(node);
  for (const edge of edges) {
    const srcKey = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
    const tgtKey = `${edge.targetNodeType}-${edge.targetEntityId}`;
    if (srcKey === key) ids.add(tgtKey);
    if (tgtKey === key) ids.add(srcKey);
  }
  return ids;
}

function curvePath(x1: number, y1: number, x2: number, y2: number, weight: number): string {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
  const curvature = Math.min(weight * 26 + 10, 54);
  const cx = (x1 + x2) / 2 + (dy / dist) * curvature;
  const cy = (y1 + y2) / 2 - (dx / dist) * curvature;
  return `M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`;
}

function nodeIcon(type: GraphNodeType): React.ReactNode {
  switch (type) {
    case "workspace":
      return <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />;
    case "file":
      return <><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" /><polyline points="14 2 14 8 20 8" /></>;
    case "planner_report":
      return <><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" /><polyline points="14 2 14 8 20 8" /><line x1="9" y1="13" x2="15" y2="13" /><line x1="9" y1="17" x2="13" y2="17" /></>;
    case "execution":
      return <><circle cx="12" cy="12" r="10" /><polyline points="12 6 12 12 16 14" /></>;
    case "memory_record":
      return <><ellipse cx="12" cy="5" rx="9" ry="3" /><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3" /><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" /></>;
    default:
      return <><circle cx="12" cy="12" r="10" /><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" /></>;
  }
}

interface KnowledgeGraphViewProps {
  nodes: KgNode[];
  edges: KgEdge[];
  onNodeSelect: (node: KgNode) => void;
  selectedNodeId?: string;
  emptyMessage?: string;
  totalHint?: number;
  onLoadMore?: () => void;
  mode?: GraphMode;
}

export function KnowledgeGraphView({
  nodes,
  edges,
  onNodeSelect,
  selectedNodeId,
  emptyMessage,
  totalHint,
  onLoadMore,
  mode = "structure",
}: KnowledgeGraphViewProps) {
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const containerRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [hoveredNodeId, setHoveredNodeId] = useState<string | null>(null);
  const [dimensions, setDimensions] = useState({ width: 1200, height: 800 });
  const momentumRef = useRef({ vx: 0, vy: 0 });
  const animRef = useRef<number>(0);
  const [showSearch, setShowSearch] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [focusedSearchIndex, setFocusedSearchIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [focusMode, setFocusMode] = useState(false);
  const [focusNodeId, setFocusNodeId] = useState<string | null>(null);

  useEffect(() => {
    if (containerRef.current) {
      const rect = containerRef.current.getBoundingClientRect();
      setDimensions({ width: rect.width || 1200, height: rect.height || 800 });
    }
  }, []);

  const forceEdges = useMemo(() => {
    if (mode === "semantic") return edges.filter((e) => e.relationshipType === "related_to");
    if (mode === "activity")
      return edges.filter(
        (e) =>
          e.relationshipType === "runs_in" ||
          e.relationshipType === "reports_on" ||
          e.relationshipType === "derived_from",
      );
    if (mode === "structure") return edges.filter((e) => e.relationshipType === "contains");
    return edges;
  }, [mode, edges]);

  // Structure mode: progressive, expandable tree derived from file paths.
  const structure = useMemo(() => buildStructure(nodes), [nodes]);
  const [expanded, setExpanded] = useState<Set<string> | null>(null);
  const effectiveExpanded = expanded ?? new Set(structure.defaultExpanded);
  const structureLayout = useMemo(
    () =>
      mode === "structure"
        ? computeStructureLayout(structure.roots, effectiveExpanded, dimensions.width, dimensions.height)
        : null,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [mode, structure, effectiveExpanded, dimensions.width, dimensions.height],
  );

  useEffect(() => {
    setExpanded(null);
  }, [structure.signature]);

  const toggleExpand = useCallback(
    (key: string, kind: StructureKind) => {
      setExpanded((prev) => {
        const next = new Set(prev ?? structure.defaultExpanded);
        if (next.has(key) && kind !== "workspace" && kind !== "layer") next.delete(key);
        else next.add(key);
        return next;
      });
    },
    [structure],
  );

  const revealForNode = useCallback(
    (n: KgNode) => {
      if (mode !== "structure" || !structureLayout) return;
      const found = flattenPlaced(structureLayout.placed).find(
        (p) => p.node && p.node.entityId === n.entityId,
      );
      if (!found) return;
      const rootKeys = structure.roots.map((r) => r.key);
      setExpanded((prev) => revealAncestors(rootKeys, found.key, prev ?? new Set(structure.defaultExpanded)));
    },
    [mode, structureLayout, structure],
  );

  const activityOf = useCallback((n: KgNode) => {
    const t = Date.parse(n.updatedAt || n.createdAt);
    return Number.isFinite(t) ? t : 0;
  }, []);
  const maxActivity = useMemo(
    () => Math.max(1, ...nodes.map((n) => activityOf(n))),
    [nodes, activityOf],
  );

  const layoutNodes = useMemo(
    () =>
      mode === "structure"
        ? []
        : runSimulation(nodes, forceEdges, dimensions.width, dimensions.height),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [nodes, forceEdges, dimensions.width, dimensions.height, mode],
  );

  const nodesMap = useMemo(
    () => new Map(layoutNodes.map((n) => [nodeKeyOf(n), n])),
    [layoutNodes],
  );

  const degrees = useMemo(() => {
    const map = new Map<string, number>();
    for (const edge of edges) {
      const sk = `${edge.sourceNodeType}-${edge.sourceEntityId}`;
      const tk = `${edge.targetNodeType}-${edge.targetEntityId}`;
      map.set(sk, (map.get(sk) ?? 0) + 1);
      map.set(tk, (map.get(tk) ?? 0) + 1);
    }
    return map;
  }, [edges]);

  const selected = useMemo(() => {
    if (!selectedNodeId) return null;
    return nodes.find((n) => n.entityId === selectedNodeId) ?? null;
  }, [selectedNodeId, nodes]);

  const selectedNodeKey = useMemo(() => (selected ? nodeKeyOf(selected) : null), [selected]);

  const selectedNeighbors = useMemo(() => {
    if (!selected) return null;
    return getNeighborIds(selected, edges);
  }, [selected, edges]);

  const visibleNodeIds = useMemo(() => {
    if (!focusMode || !focusNodeId) return new Set(nodes.map((n) => nodeKeyOf(n)));
    const ids = new Set<string>();
    const focusKey = nodes.find((n) => n.entityId === focusNodeId);
    if (!focusKey) return ids;
    const fk = nodeKeyOf(focusKey);
    ids.add(fk);
    const neighbors = getNeighborIds(focusKey, edges);
    for (const nid of neighbors) ids.add(nid);
    return ids;
  }, [focusMode, focusNodeId, nodes, edges]);

  const isDimmed = useCallback(
    (node: KgNode) => {
      const key = nodeKeyOf(node);
      if (focusMode && focusNodeId) return !visibleNodeIds.has(key);
      if (!selectedNeighbors || !selectedNodeKey) return false;
      if (key === selectedNodeKey) return false;
      return !selectedNeighbors.has(key);
    },
    [focusMode, focusNodeId, visibleNodeIds, selectedNeighbors, selectedNodeKey],
  );

  const isNeighbor = useCallback(
    (node: KgNode) => {
      const key = nodeKeyOf(node);
      return Boolean(selectedNeighbors && selectedNeighbors.has(key) && key !== selectedNodeKey);
    },
    [selectedNeighbors, selectedNodeKey],
  );

  const searchResults = useMemo(() => {
    if (!searchQuery.trim()) return [] as PositionedNode[];
    const q = searchQuery.toLowerCase();
    if (mode === "structure" && structureLayout) {
      const hits: PositionedNode[] = [];
      for (const p of flattenPlaced(structureLayout.placed)) {
        if (!p.node) continue;
        if (p.label.toLowerCase().includes(q) || p.node.title.toLowerCase().includes(q)) {
          hits.push({ ...p.node, x: p.x, y: p.y, vx: 0, vy: 0 });
        }
      }
      return hits;
    }
    return layoutNodes.filter((n) => n.title.toLowerCase().includes(q));
  }, [searchQuery, layoutNodes, mode, structureLayout]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const rect = containerRef.current?.getBoundingClientRect();
      if (!rect) return;
      const px = e.clientX - rect.left - dimensions.width / 2;
      const py = e.clientY - rect.top - dimensions.height / 2;
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      setZoom((prev) => {
        const next = Math.min(Math.max(prev * delta, 0.12), 6);
        const worldX = (px - offset.x) / prev;
        const worldY = (py - offset.y) / prev;
        setOffset({ x: px - worldX * next, y: py - worldY * next });
        return next;
      });
    },
    [dimensions, offset],
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (e.button === 0) {
        setIsDragging(true);
        setDragStart({ x: e.clientX - offset.x, y: e.clientY - offset.y });
        momentumRef.current = { vx: 0, vy: 0 };
        cancelAnimationFrame(animRef.current);
      }
    },
    [offset],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (isDragging) {
        momentumRef.current = { vx: e.clientX - dragStart.x - offset.x, vy: e.clientY - dragStart.y - offset.y };
        setOffset({ x: e.clientX - dragStart.x, y: e.clientY - dragStart.y });
      }
    },
    [isDragging, dragStart, offset],
  );

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
    const m = momentumRef.current;
    if (Math.abs(m.vx) > 2 || Math.abs(m.vy) > 2) {
      const decay = () => {
        momentumRef.current.vx *= 0.92;
        momentumRef.current.vy *= 0.92;
        if (Math.abs(momentumRef.current.vx) < 0.5 && Math.abs(momentumRef.current.vy) < 0.5) return;
        setOffset((prev) => ({
          x: prev.x + momentumRef.current.vx,
          y: prev.y + momentumRef.current.vy,
        }));
        animRef.current = requestAnimationFrame(decay);
      };
      animRef.current = requestAnimationFrame(decay);
    }
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowSearch(false);
        setSearchQuery("");
        return;
      }
      if (showSearch) {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setFocusedSearchIndex((prev) => Math.min(prev + 1, searchResults.length - 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          setFocusedSearchIndex((prev) => Math.max(prev - 1, 0));
        } else if (e.key === "Enter" && searchResults.length > 0) {
          const result = searchResults[focusedSearchIndex];
          if (result) {
            revealForNode(result);
            jumpTo(result);
          }
        }
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [showSearch, searchResults, focusedSearchIndex],
  );

  useEffect(() => {
    return () => cancelAnimationFrame(animRef.current);
  }, []);

  useEffect(() => {
    if (showSearch && searchInputRef.current) searchInputRef.current.focus();
  }, [showSearch]);

  const toggleFocusMode = useCallback(() => {
    if (selectedNodeId) {
      if (focusMode && focusNodeId === selectedNodeId) {
        setFocusMode(false);
        setFocusNodeId(null);
      } else {
        setFocusMode(true);
        setFocusNodeId(selectedNodeId);
      }
    }
  }, [selectedNodeId, focusMode, focusNodeId]);

  const jumpTo = useCallback(
    (node: PositionedNode) => {
      onNodeSelect(node);
      setZoom(2);
      setOffset({ x: -(node.x * 2 - dimensions.width / 2), y: -(node.y * 2 - dimensions.height / 2) });
      setShowSearch(false);
      setSearchQuery("");
    },
    [onNodeSelect, dimensions],
  );

  // Auto-fit the Structure tree the first time it becomes visible, so the
  // default view fills the canvas instead of hugging one corner.
  const didFitRef = useRef<{ mode: GraphMode; sig: string } | null>(null);
  useEffect(() => {
    if (mode !== "structure" || !structureLayout) return;
    const sig = structure.signature;
    if (didFitRef.current && didFitRef.current.mode === mode && didFitRef.current.sig === sig) return;
    didFitRef.current = { mode, sig };
    if (structureLayout.placed.length > 0) fitToView();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, structureLayout, structure.signature]);

  // Clicking a folder expands/collapses it; clicking a file selects it.
  const handleStructureNodeClick = useCallback(
    (key: string, kind: StructureKind, node: KgNode | null) => {
      if (kind === "file" || kind === "other") {
        if (node) onNodeSelect(node);
        return;
      }
      toggleExpand(key, kind);
    },
    [onNodeSelect, toggleExpand],
  );

  const resetView = useCallback(() => {
    setZoom(1);
    setOffset({ x: 0, y: 0 });
    momentumRef.current = { vx: 0, vy: 0 };
  }, []);

  const fitToView = useCallback(() => {
    if (mode === "structure") {
      if (!structureLayout || structureLayout.placed.length === 0) return;
      const { minX, maxX, minY, maxY } = structureLayout;
      const w = maxX - minX;
      const h = maxY - minY;
      const next = Math.min(Math.max(Math.min(dimensions.width / (w || 1), dimensions.height / (h || 1)), 0.15), 1.4);
      setZoom(next);
      setOffset({
        x: dimensions.width / 2 - (minX + w / 2) * next,
        y: dimensions.height / 2 - (minY + h / 2) * next,
      });
      return;
    }
    if (layoutNodes.length === 0) return;
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    for (const n of layoutNodes) {
      minX = Math.min(minX, n.x - nodeRadius(n.nodeType) - 22);
      minY = Math.min(minY, n.y - nodeRadius(n.nodeType) - 22);
      maxX = Math.max(maxX, n.x + nodeRadius(n.nodeType) + 22);
      maxY = Math.max(maxY, n.y + nodeRadius(n.nodeType) + 22);
    }
    const w = maxX - minX;
    const h = maxY - minY;
    const next = Math.min(Math.max(Math.min(dimensions.width / (w || 1), dimensions.height / (h || 1)), 0.12), 1.6);
    setZoom(next);
    setOffset({
      x: dimensions.width / 2 - (minX + w / 2) * next,
      y: dimensions.height / 2 - (minY + h / 2) * next,
    });
  }, [layoutNodes, dimensions, mode, structureLayout]);

  const applyZoom = useCallback((factor: number) => {
    setZoom((prev) => Math.min(Math.max(prev * factor, 0.12), 6));
  }, []);

  if (nodes.length === 0) {
    return (
      <div className="relative flex h-full w-full flex-col items-center justify-center gap-6 overflow-hidden p-12 text-center">
        <div className="pointer-events-none absolute inset-0 bg-dotgrid opacity-40" aria-hidden="true" />
        <div className="relative flex h-20 w-20 items-center justify-center rounded-3xl border border-(--color-border-subtle) bg-(--color-surface-raised) shadow-[var(--shadow-pop)]">
          <Waypoints className="h-8 w-8 text-(--color-muted-foreground)" strokeWidth={1.5} />
        </div>
        <div className="relative max-w-md">
          <h3 className="font-(family-name:--font-display) text-2xl font-bold tracking-tight text-(--color-foreground)">
            A living map of your work
          </h3>
          <p className="mx-auto mt-2 text-sm leading-relaxed text-(--color-muted-foreground)">{emptyMessage}</p>
        </div>
      </div>
    );
  }

  const viewportW = dimensions.width;
  const viewportH = dimensions.height;
  const mmScale = 0.15;
  const mmW = viewportW * mmScale;
  const mmH = viewportH * mmScale;
  const maxDegree = Math.max(1, ...degrees.values());
  const labelsVisible = zoom >= 0.55;

  return (
    <div
      ref={containerRef}
      className="relative h-full w-full cursor-grab overflow-hidden bg-dotgrid active:cursor-grabbing"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      aria-label="Knowledge graph canvas"
    >
      {/* Spatial depth — vignette over the ambient light fields. */}
      <div className="pointer-events-none absolute inset-0 bg-vignette" aria-hidden="true" />
      <div
        className="absolute inset-0 transition-transform duration-150 ease-[var(--ease-premium)]"
        style={{ transform: `translate(${offset.x}px, ${offset.y}px) scale(${zoom})` }}
      >
        <svg width={dimensions.width} height={dimensions.height} className="overflow-visible">
          <defs>
            {Object.entries(NODE_COLORS).map(([type, color]) => (
              <radialGradient key={`grad-${type}`} id={`grad-${type}`} cx="50%" cy="38%" r="70%">
                <stop offset="0%" stopColor={color} stopOpacity="0.34" />
                <stop offset="55%" stopColor={color} stopOpacity="0.12" />
                <stop offset="100%" stopColor={color} stopOpacity="0.02" />
              </radialGradient>
            ))}
          </defs>

          {mode === "structure" && structureLayout && renderStructure(structureLayout, effectiveExpanded, handleStructureNodeClick, selectedNodeId)}

          {mode !== "structure" &&
            forceEdges.map((edge) => {
            const source = nodesMap.get(`${edge.sourceNodeType}-${edge.sourceEntityId}`);
            const target = nodesMap.get(`${edge.targetNodeType}-${edge.targetEntityId}`);
            if (!source || !target) return null;

            const isHighlighted =
              selectedNodeId &&
              (edge.sourceEntityId === selectedNodeId || edge.targetEntityId === selectedNodeId);
            const isHovered = hoveredEdge === edge.id;
            const isRelated = isHighlighted || isHovered;
            const ec = edgeColor(edge.relationshipType);
            const isSemantic = edge.relationshipType === "related_to";
            const confidenceOpacity = isSemantic ? 0.25 + (edge.confidence ?? 1) * 0.75 : 1;
            const baseOpacity = isRelated ? 0.85 : 0.28;
            const baseWidth = isRelated ? 1.6 + edge.weight * 2.4 : 0.6 + edge.weight * 1.4;

            return (
              <g key={edge.id} className="group">
                {isRelated && (
                  <path
                    d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                    fill="none"
                    stroke={ec}
                    strokeWidth={baseWidth * 3.2}
                    strokeOpacity={0.16}
                    className="pointer-events-none"
                  />
                )}
                <path
                  d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                  fill="none"
                  stroke={isHighlighted ? nodeColor(source.nodeType) : ec}
                  strokeWidth={baseWidth}
                  strokeOpacity={baseOpacity * confidenceOpacity}
                  strokeLinecap="round"
                  className="pointer-events-none transition-all duration-300 ease-[var(--ease-premium)]"
                />
                {isHovered && (
                  <path
                    d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                    fill="none"
                    stroke={ec}
                    strokeWidth={baseWidth}
                    strokeOpacity={0.9}
                    strokeDasharray="3 7"
                    className="pointer-events-none"
                  />
                )}
                <path
                  d={curvePath(source.x, source.y, target.x, target.y, edge.weight)}
                  fill="none"
                  stroke="transparent"
                  strokeWidth={14}
                  onMouseEnter={() => setHoveredEdge(edge.id)}
                  onMouseLeave={() => setHoveredEdge(null)}
                >
                  <title>{`${EDGE_LABELS[edge.relationshipType] ?? edge.relationshipType} · weight ${(edge.weight * 100).toFixed(0)}%${isSemantic ? ` · confidence ${((edge.confidence ?? 1) * 100).toFixed(0)}%` : ""}`}</title>
                </path>
              </g>
            );
          })}

          {mode !== "structure" &&
            layoutNodes.map((node) => {
            const key = nodeKeyOf(node);
            const isSelected = selectedNodeId === node.entityId;
            const dimmed = isDimmed(node);
            const neighbor = isNeighbor(node);
            const hovered = hoveredNodeId === key;
            const activityFrac = maxActivity > 1 ? activityOf(node) / maxActivity : 0.5;
            const sizeFactor = mode === "activity" ? 0.78 + 0.55 * activityFrac : 1 + 0.25 * ((degrees.get(key) ?? 0) / maxDegree);
            const r = nodeRadius(node.nodeType) * sizeFactor;
            const col = nodeColor(node.nodeType);
            const iconScale = Math.max(0.38, Math.min(0.5, 0.4 + ((degrees.get(key) ?? 0) / maxDegree) * 0.1));

            return (
              <g
                key={key}
                transform={`translate(${node.x}, ${node.y})`}
                className="cursor-pointer"
                style={{ transition: "opacity 0.35s ease" }}
                onClick={(e) => {
                  e.stopPropagation();
                  onNodeSelect(node);
                }}
                onMouseEnter={() => setHoveredNodeId(key)}
                onMouseLeave={() => setHoveredNodeId(null)}
                onDoubleClick={() => jumpTo(node)}
              >
                {isSelected && (
                  <>
                    <circle
                      r={r + 9}
                      fill={col}
                      opacity={0.1}
                      className="pointer-events-none"
                    />
                    <circle
                      r={r + 5}
                      fill="none"
                      stroke="#f4f4f6"
                      strokeOpacity={0.65}
                      strokeWidth={1.25}
                      className="pointer-events-none"
                    />
                  </>
                )}

                <circle
                  r={r}
                  fill={dimmed ? "#0a0a0d" : `url(#grad-${node.nodeType})`}
                  stroke={isSelected ? "#f4f4f6" : dimmed ? "#26262e" : col}
                  strokeWidth={isSelected ? 1.75 : neighbor ? 1.75 : 1.25}
                  className="transition-all duration-500 ease-[var(--ease-premium)]"
                  style={{ opacity: dimmed ? 0.28 : 1 }}
                />
                <circle r={r} fill="none" stroke={hovered ? "#ffffff" : "none"} strokeWidth={hovered ? 0.75 : 0} strokeDasharray="2 5" className="transition-all duration-200" style={{ opacity: hovered ? 0.8 : 0 }} />
                <foreignObject x={-r * 0.4} y={-r * 0.4} width={r * 0.8} height={r * 0.8} className="pointer-events-none">
                  <div className="flex h-full w-full items-center justify-center" style={{ opacity: dimmed ? 0.28 : 1 }}>
                    <svg
                      width={r * iconScale}
                      height={r * iconScale}
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke={isSelected ? "#0a0a0d" : "#ffffff"}
                      strokeWidth="1.9"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      {nodeIcon(node.nodeType)}
                    </svg>
                  </div>
                </foreignObject>
                {labelsVisible && (
                  <text
                    y={r + 15}
                    textAnchor="middle"
                    fill={dimmed ? "#3a3a44" : hovered ? "#f4f4f6" : "#a3a3ad"}
                    fontSize={9.5}
                    fontWeight={650}
                    className="pointer-events-none transition-colors duration-300"
                    style={{ opacity: dimmed ? 0.4 : 1, letterSpacing: 0.2 }}
                  >
                    {node.title.length > 16 ? node.title.slice(0, 14) + "\u2026" : node.title}
                  </text>
                )}
              </g>
            );
          })}
        </svg>
      </div>
      {selected && (
        <GlassSurface
          material="panel"
          refraction={true}
          className="absolute left-4 top-4 z-20 w-[272px] animate-fade-in rounded-[var(--radius-card)] p-4"
        >
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="truncate font-(family-name:--font-display) text-sm font-semibold text-(--color-foreground)">
                {selected.title}
              </p>
              <span
                className="mt-1.5 inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
                style={{ backgroundColor: `${nodeColor(selected.nodeType)}22`, color: nodeColor(selected.nodeType) }}
              >
                <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: nodeColor(selected.nodeType) }} />
                {selected.nodeType.replace("_", " ")}
              </span>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              <button
                onClick={toggleFocusMode}
                className={`rounded-[var(--radius-control)] p-1.5 transition-colors ${
                  focusMode && focusNodeId === selected.entityId
                    ? "bg-(--color-accent)/15 text-(--color-accent)"
                    : "text-(--color-faint-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                }`}
                title={focusMode && focusNodeId === selected.entityId ? "Exit focus mode" : "Focus on this node"}
              >
                <Focus className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
              <button
                onClick={() => jumpTo(layoutNodes.find((n) => nodeKeyOf(n) === selectedNodeKey) ?? (selected as PositionedNode))}
                className="shrink-0 rounded-[var(--radius-control)] p-1.5 text-(--color-faint-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
                title="Center on this node"
              >
                <LocateFixed className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
            </div>
          </div>
          {selectedNeighbors && (
            <p className="mt-2 text-[11px] text-(--color-muted-foreground)">
              <span className="font-semibold text-(--color-foreground)">{selectedNeighbors.size}</span>{" "}
              connection{selectedNeighbors.size !== 1 ? "s" : ""} — related nodes are highlighted
            </p>
          )}
          <div className="mt-3 flex flex-wrap gap-1.5">
            <button
              onClick={toggleFocusMode}
              className={`flex items-center gap-1.5 rounded-[var(--radius-control)] px-2.5 py-1.5 text-[11px] font-medium transition-colors ${
                focusMode && focusNodeId === selected.entityId
                  ? "bg-(--color-accent)/15 text-(--color-accent)"
                  : "bg-(--color-surface-hover) text-(--color-muted-foreground) hover:text-(--color-foreground)"
              }`}
            >
              <Focus className="h-3 w-3" strokeWidth={1.75} />
              {focusMode && focusNodeId === selected.entityId ? "Restore view" : "Focus view"}
            </button>
            <button
              onClick={() => {
                setShowSearch(true);
                setSearchQuery("");
              }}
              className="flex items-center gap-1.5 rounded-[var(--radius-control)] bg-(--color-surface-hover) px-2.5 py-1.5 text-[11px] font-medium text-(--color-muted-foreground) transition-colors hover:text-(--color-foreground)"
            >
              <Search className="h-3 w-3" strokeWidth={1.75} />
              Search
            </button>
          </div>
        </GlassSurface>
      )}

      {showSearch && (
        <div className="absolute left-1/2 top-4 z-30 w-80 -translate-x-1/2 animate-slide-down">
          <div className="glass-control flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) px-3 py-2.5">
            <Search className="h-4 w-4 shrink-0 text-(--color-muted-foreground)" strokeWidth={1.75} />
            <input
              ref={searchInputRef}
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setFocusedSearchIndex(0);
              }}
              placeholder="Search nodes…"
              aria-label="Search graph nodes"
              className="flex-1 bg-transparent text-sm text-(--color-foreground) placeholder:text-(--color-faint-foreground) focus:outline-none"
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setShowSearch(false);
                  setSearchQuery("");
                }
              }}
            />
            {searchQuery && (
              <button
                onClick={() => {
                  setSearchQuery("");
                  setFocusedSearchIndex(0);
                }}
                className="rounded p-0.5 text-(--color-faint-foreground) hover:text-(--color-foreground)"
              >
                <X className="h-3.5 w-3.5" strokeWidth={1.75} />
              </button>
            )}
            <kbd className="hidden rounded border border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground) sm:inline">
              ESC
            </kbd>
          </div>
          {searchResults.length > 0 && (
            <div className="glass-panel mt-1.5 max-h-56 overflow-y-auto rounded-[var(--radius-control)] py-1">
              {searchResults.map((result, i) => (
                <button
                  key={nodeKeyOf(result)}
                  onClick={() => {
                    revealForNode(result);
                    jumpTo(result);
                  }}
                  className={`flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm transition-colors ${
                    i === focusedSearchIndex
                      ? "bg-(--color-accent)/10 text-(--color-accent)"
                      : "text-(--color-foreground) hover:bg-(--color-surface-hover)"
                  }`}
                >
                  <span className="h-2 w-2 shrink-0 rounded-full" style={{ backgroundColor: nodeColor(result.nodeType) }} />
                  <span className="min-w-0 flex-1 truncate">{result.title}</span>
                  <span className="ml-auto shrink-0 text-[10px] text-(--color-faint-foreground)">
                    {result.nodeType.replace("_", " ")}
                  </span>
                </button>
              ))}
            </div>
          )}
          {searchResults.length === 0 && searchQuery.trim() && (
            <div className="glass-panel mt-1.5 rounded-[var(--radius-control)] px-3 py-2.5 text-sm text-(--color-muted-foreground)">
              No nodes match “{searchQuery}”.
            </div>
          )}
        </div>
      )}

      <div className="absolute right-5 bottom-5 z-20 flex flex-col items-center gap-1.5">
        <div className="glass-control flex flex-col items-center gap-1 rounded-[var(--radius-control)] border border-(--color-border) p-1">
          <button
            onClick={() => applyZoom(1.28)}
            className="rounded-[var(--radius-control)] p-2 text-(--color-muted-foreground) transition-all duration-150 hover:bg-(--color-surface-hover) hover:text-(--color-foreground) active:scale-90"
            title="Zoom in"
          >
            <ZoomIn className="h-4 w-4" strokeWidth={1.75} />
          </button>
          <span className="font-(family-name:--font-mono) text-[10px] tabular-nums text-(--color-muted-foreground)">
            {Math.round(zoom * 100)}%
          </span>
          <button
            onClick={() => applyZoom(1 / 1.28)}
            className="rounded-[var(--radius-control)] p-2 text-(--color-muted-foreground) transition-all duration-150 hover:bg-(--color-surface-hover) hover:text-(--color-foreground) active:scale-90"
            title="Zoom out"
          >
            <ZoomOut className="h-4 w-4" strokeWidth={1.75} />
          </button>
        </div>
        <button
          onClick={fitToView}
          className="glass-control rounded-[var(--radius-control)] border border-(--color-border) p-2 text-(--color-muted-foreground) transition-all duration-150 hover:bg-(--color-surface-hover) hover:text-(--color-foreground) active:scale-90"
          title="Fit to view"
        >
          <Maximize className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={resetView}
          className="glass-control rounded-[var(--radius-control)] border border-(--color-border) p-2 text-(--color-muted-foreground) transition-all duration-150 hover:bg-(--color-surface-hover) hover:text-(--color-foreground) active:scale-90"
          title="Reset view"
        >
          <LocateFixed className="h-4 w-4" strokeWidth={1.75} />
        </button>
        <button
          onClick={() => setShowSearch((p) => !p)}
          className={`glass-control rounded-[var(--radius-control)] border p-2 transition-all duration-150 active:scale-90 ${
            showSearch
              ? "border-(--color-accent)/50 bg-(--color-accent)/10 text-(--color-accent)"
              : "border-(--color-border) text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          }`}
          title="Search"
        >
          <Search className="h-4 w-4" strokeWidth={1.75} />
        </button>
      </div>

      <div className="glass-control absolute bottom-5 left-5 z-20 flex items-center gap-2.5 rounded-[var(--radius-control)] border border-(--color-border) px-3.5 py-2 text-xs text-(--color-muted-foreground)">
        <span className="font-medium text-(--color-foreground)">{nodes.length} nodes</span>
        <span className="h-3 w-px bg-(--color-border)" />
        <span className="font-medium text-(--color-foreground)">
          {mode === "structure"
            ? `${flattenPlaced(structureLayout?.placed ?? []).filter((p) => p.kind === "folder" && p.depth > 1).length} folders`
            : `${forceEdges.length} edges`}
        </span>
        <span className="h-3 w-px bg-(--color-border)" />
        <span>{MODE_LABELS[mode]}</span>
      </div>

      {onLoadMore && totalHint != null && nodes.length < totalHint && (
        <div className="absolute bottom-5 left-1/2 z-20 -translate-x-1/2">
          <button
            onClick={onLoadMore}
            className="glass-control flex items-center gap-2 rounded-[var(--radius-control)] border border-(--color-border) px-4 py-2 text-xs font-medium text-(--color-muted-foreground) transition-colors hover:bg-(--color-surface-hover) hover:text-(--color-foreground)"
          >
            <span className="relative flex h-2 w-2">
              <span className="inline-flex h-2 w-2 rounded-full bg-(--color-accent)" />
            </span>
            {nodes.length} of {totalHint} nodes loaded — load more
          </button>
        </div>
      )}

      <div className="glass-panel absolute bottom-5 left-1/2 z-10 hidden -translate-x-1/2 overflow-hidden rounded-[var(--radius-control)] p-0.5 transition-opacity duration-300 hover:opacity-100 lg:block xl:opacity-80" style={{ width: mmW + 16, height: mmH + 16, cursor: "pointer" }}>
        {mode === "structure" && structureLayout ? (
          <svg viewBox={`${structureLayout.minX - 20} ${structureLayout.minY - 20} ${structureLayout.maxX - structureLayout.minX + 40} ${structureLayout.maxY - structureLayout.minY + 40}`} width={mmW + 16} height={mmH + 16}>
            {flattenPlaced(structureLayout.placed).map((n) => (
              <rect
                key={`mm-${n.key}`}
                x={n.x - n.w / 2}
                y={n.y - n.h / 2}
                width={n.w}
                height={n.h}
                rx={n.h / 2}
                fill={n.tone}
                opacity={0.4}
              />
            ))}
          </svg>
        ) : (
          <svg viewBox={`0 0 ${viewportW} ${viewportH}`} width={mmW + 16} height={mmH + 16}>
            {layoutNodes.map((node) => (
              <circle
                key={nodeKeyOf(node)}
                cx={node.x}
                cy={node.y}
                r={Math.max(1.6, nodeRadius(node.nodeType) * mmScale)}
                fill={nodeColor(node.nodeType)}
                opacity={0.55}
              />
            ))}
          </svg>
        )}
        <rect x={0} y={0} width={mmW} height={mmH} fill="none" pointerEvents="none" />
      </div>

      <div className="glass-control absolute left-1/2 top-4 z-10 hidden -translate-x-1/2 items-center gap-3 rounded-full border border-(--color-border-subtle) px-4 py-1.5 text-[10px] text-(--color-faint-foreground) xl:flex">
        {mode === "structure" ? (
          <>
            {[
              ["Workspace", STRUCTURE_TONES.workspace],
              ["Folder", "#8e8e93"],
              ["File", "#8fa9c4"],
              ["React", "#5b9dff"],
              ["Rust", "#d9a05b"],
              ["Database", "#a78bdc"],
              ["AI", "#63c98f"],
            ].map(([label, tone]) => (
              <span key={label} className="flex items-center gap-1.5">
                <span className="h-2 w-2 rounded-full" style={{ backgroundColor: tone }} />
                {label}
              </span>
            ))}
            <span className="h-3 w-px bg-(--color-border)" />
            <span>Click a folder to expand</span>
          </>
        ) : (
          <>
            <span className="flex items-center gap-1.5">
              <Keyboard className="h-3 w-3" strokeWidth={1.75} />
              Scroll to zoom · drag to pan
            </span>
            <span className="h-3 w-px bg-(--color-border)" />
            {focusMode && focusNodeId ? (
              <span className="text-(--color-violet)">Focus mode on — click a node to explore</span>
            ) : (
              <span>Click a node to explore its context</span>
            )}
          </>
        )}
      </div>
    </div>
  );
}