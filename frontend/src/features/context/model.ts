import type { ContextEdge, ContextNode } from "./types";

/**
 * The workspace context manifest — the app's own structure as a graph.
 *
 * Foreground: App.tsx, the workspace entry point.
 * Active context: the components, services, hooks, styles and tests that
 * make up the running app, each connected to the entry point.
 * Wider workspace: the distant artifacts (docs, config, folders) that
 * surround the code — always present, never foreground.
 */
export const CONTEXT_NODES: ContextNode[] = [
  {
    id: "App.tsx",
    label: "App.tsx",
    path: "App.tsx",
    kind: "entry",
    tier: "foreground",
    role: "Workspace Entry Point",
    detail: "React Component",
  },

  /* ---- Active context ---------------------------------------------- */
  {
    id: "components/Header.tsx",
    label: "Header.tsx",
    path: "components/Header.tsx",
    kind: "component",
    tier: "active",
    role: "Component",
  },
  {
    id: "components/Sidebar.tsx",
    label: "Sidebar.tsx",
    path: "components/Sidebar.tsx",
    kind: "component",
    tier: "active",
    role: "Component",
  },
  {
    id: "components/Dashboard.tsx",
    label: "Dashboard.tsx",
    path: "components/Dashboard.tsx",
    kind: "component",
    tier: "active",
    role: "Component",
  },
  {
    id: "services/api.ts",
    label: "api.ts",
    path: "services/api.ts",
    kind: "service",
    tier: "active",
    role: "Service",
  },
  {
    id: "services/auth.ts",
    label: "auth.ts",
    path: "services/auth.ts",
    kind: "service",
    tier: "active",
    role: "Service",
  },
  {
    id: "hooks/useWorkspace.ts",
    label: "useWorkspace.ts",
    path: "hooks/useWorkspace.ts",
    kind: "hook",
    tier: "active",
    role: "Hook",
  },
  {
    id: "styles/theme.css",
    label: "theme.css",
    path: "styles/theme.css",
    kind: "style",
    tier: "active",
    role: "Styles",
  },
  {
    id: "tests/App.test.tsx",
    label: "App.test.tsx",
    path: "tests/App.test.tsx",
    kind: "test",
    tier: "active",
    role: "Test",
  },

  /* ---- Wider workspace context ------------------------------------- */
  {
    id: "README.md",
    label: "README.md",
    path: "README.md",
    kind: "doc",
    tier: "background",
    role: "Documentation",
  },
  {
    id: "docs/architecture.md",
    label: "architecture.md",
    path: "docs/architecture.md",
    kind: "doc",
    tier: "background",
    role: "Documentation",
  },
  {
    id: "package.json",
    label: "package.json",
    path: "package.json",
    kind: "config",
    tier: "background",
    role: "Configuration",
  },
  {
    id: "documentation/",
    label: "documentation",
    path: "documentation/",
    kind: "folder",
    tier: "background",
    role: "Folder",
  },
  {
    id: "repositories/",
    label: "repositories",
    path: "repositories/",
    kind: "folder",
    tier: "background",
    role: "Folder",
  },
  {
    id: "screenshots/",
    label: "screenshots",
    path: "screenshots/",
    kind: "folder",
    tier: "background",
    role: "Folder",
  },
  {
    id: "tests/",
    label: "tests",
    path: "tests/",
    kind: "folder",
    tier: "background",
    role: "Folder",
  },
];

const appId = "App.tsx";

const activeIds = CONTEXT_NODES.filter((n) => n.tier === "active").map((n) => n.id);
const backgroundIds = CONTEXT_NODES.filter((n) => n.tier === "background").map((n) => n.id);

export const CONTEXT_EDGES: ContextEdge[] = [
  /* The active context — every artifact is a direct relationship of the
     workspace entry point. */
  ...activeIds.map((id) => ({ id: `app-${id}`, source: appId, target: id, tier: "primary" as const })),

  /* The wider workspace — present but faint. */
  ...backgroundIds.map((id) => ({ id: `app-${id}`, source: appId, target: id, tier: "faint" as const })),

  /* A few sector-local relationships — placed in their own angular gaps
     so curves never cross the hub spokes. */
  { id: "tests-folder-test-file", source: "tests/", target: "tests/App.test.tsx", tier: "secondary" },
  { id: "readme-architecture", source: "README.md", target: "docs/architecture.md", tier: "secondary" },
];

/** Number of active-context connections of the entry point (inspector). */
export const ENTRY_CONNECTIONS = activeIds.length;