import { useMemo } from "react";
import {
  UncontrolledTreeEnvironment,
  StaticTreeDataProvider,
  Tree,
  type TreeItem,
} from "react-complex-tree";
import { ExplorerNode } from "@/components/explorer/ExplorerNode";
import type { ExplorerNodeData, ExplorerTreeItems } from "@/components/explorer/types";

/**
 * Placeholder tree data (temporary — Phase 4.1 has no filesystem IPC
 * yet). Shaped after ChronoDesk's own repo so the explorer looks "real"
 * rather than a generic demo tree, per the design brief. Swapping this
 * for a live `commands::fs::*`-backed `TreeDataProvider` is the
 * documented follow-up (see the completion summary).
 */
function buildPlaceholderItems(): ExplorerTreeItems {
  const folder = (name: string, path: string, children: string[]): TreeItem<ExplorerNodeData> => ({
    index: path,
    isFolder: true,
    children,
    data: { name, path, kind: "folder" },
  });

  const file = (name: string, path: string): TreeItem<ExplorerNodeData> => {
    const extension = name.includes(".") ? name.split(".").pop() : undefined;
    return {
      index: path,
      isFolder: false,
      data: { name, path, kind: "file", extension },
    };
  };

  const items: ExplorerTreeItems = {
    root: folder("root", "root", ["frontend", "src-tauri", "ARCHITECTURE.md", "README.md"]),

    frontend: folder("frontend", "frontend", ["frontend/src"]),
    "frontend/src": folder("src", "frontend/src", [
      "frontend/src/components",
      "frontend/src/pages",
      "frontend/src/services",
      "frontend/src/App.tsx",
      "frontend/src/main.tsx",
      "frontend/src/index.css",
    ]),
    "frontend/src/components": folder("components", "frontend/src/components", [
      "frontend/src/components/explorer",
      "frontend/src/components/navigation",
      "frontend/src/components/ui",
    ]),
    "frontend/src/components/explorer": folder("explorer", "frontend/src/components/explorer", [
      "frontend/src/components/explorer/FileExplorer.tsx",
      "frontend/src/components/explorer/ExplorerTree.tsx",
      "frontend/src/components/explorer/ExplorerNode.tsx",
      "frontend/src/components/explorer/types.ts",
    ]),
    "frontend/src/components/explorer/FileExplorer.tsx": file(
      "FileExplorer.tsx",
      "frontend/src/components/explorer/FileExplorer.tsx",
    ),
    "frontend/src/components/explorer/ExplorerTree.tsx": file(
      "ExplorerTree.tsx",
      "frontend/src/components/explorer/ExplorerTree.tsx",
    ),
    "frontend/src/components/explorer/ExplorerNode.tsx": file(
      "ExplorerNode.tsx",
      "frontend/src/components/explorer/ExplorerNode.tsx",
    ),
    "frontend/src/components/explorer/types.ts": file(
      "types.ts",
      "frontend/src/components/explorer/types.ts",
    ),
    "frontend/src/components/navigation": folder("navigation", "frontend/src/components/navigation", [
      "frontend/src/components/navigation/Sidebar.tsx",
      "frontend/src/components/navigation/Topbar.tsx",
      "frontend/src/components/navigation/NavItem.tsx",
    ]),
    "frontend/src/components/navigation/Sidebar.tsx": file(
      "Sidebar.tsx",
      "frontend/src/components/navigation/Sidebar.tsx",
    ),
    "frontend/src/components/navigation/Topbar.tsx": file(
      "Topbar.tsx",
      "frontend/src/components/navigation/Topbar.tsx",
    ),
    "frontend/src/components/navigation/NavItem.tsx": file(
      "NavItem.tsx",
      "frontend/src/components/navigation/NavItem.tsx",
    ),
    "frontend/src/components/ui": folder("ui", "frontend/src/components/ui", [
      "frontend/src/components/ui/Card.tsx",
      "frontend/src/components/ui/Button.tsx",
      "frontend/src/components/ui/Badge.tsx",
    ]),
    "frontend/src/components/ui/Card.tsx": file("Card.tsx", "frontend/src/components/ui/Card.tsx"),
    "frontend/src/components/ui/Button.tsx": file("Button.tsx", "frontend/src/components/ui/Button.tsx"),
    "frontend/src/components/ui/Badge.tsx": file("Badge.tsx", "frontend/src/components/ui/Badge.tsx"),

    "frontend/src/pages": folder("pages", "frontend/src/pages", [
      "frontend/src/pages/DashboardPage.tsx",
      "frontend/src/pages/WorkspacesPage.tsx",
      "frontend/src/pages/TimelinePage.tsx",
      "frontend/src/pages/SettingsPage.tsx",
    ]),
    "frontend/src/pages/DashboardPage.tsx": file("DashboardPage.tsx", "frontend/src/pages/DashboardPage.tsx"),
    "frontend/src/pages/WorkspacesPage.tsx": file(
      "WorkspacesPage.tsx",
      "frontend/src/pages/WorkspacesPage.tsx",
    ),
    "frontend/src/pages/TimelinePage.tsx": file("TimelinePage.tsx", "frontend/src/pages/TimelinePage.tsx"),
    "frontend/src/pages/SettingsPage.tsx": file("SettingsPage.tsx", "frontend/src/pages/SettingsPage.tsx"),

    "frontend/src/services": folder("services", "frontend/src/services", [
      "frontend/src/services/workspaceRepository.ts",
      "frontend/src/services/timelineRepository.ts",
    ]),
    "frontend/src/services/workspaceRepository.ts": file(
      "workspaceRepository.ts",
      "frontend/src/services/workspaceRepository.ts",
    ),
    "frontend/src/services/timelineRepository.ts": file(
      "timelineRepository.ts",
      "frontend/src/services/timelineRepository.ts",
    ),

    "frontend/src/App.tsx": file("App.tsx", "frontend/src/App.tsx"),
    "frontend/src/main.tsx": file("main.tsx", "frontend/src/main.tsx"),
    "frontend/src/index.css": file("index.css", "frontend/src/index.css"),

    "src-tauri": folder("src-tauri", "src-tauri", ["src-tauri/src"]),
    "src-tauri/src": folder("src", "src-tauri/src", [
      "src-tauri/src/commands",
      "src-tauri/src/services",
      "src-tauri/src/watcher",
      "src-tauri/src/workspace",
      "src-tauri/src/lib.rs",
      "src-tauri/src/main.rs",
    ]),
    "src-tauri/src/commands": folder("commands", "src-tauri/src/commands", [
      "src-tauri/src/commands/workspace.rs",
      "src-tauri/src/commands/watcher.rs",
      "src-tauri/src/commands/timeline.rs",
    ]),
    "src-tauri/src/commands/workspace.rs": file("workspace.rs", "src-tauri/src/commands/workspace.rs"),
    "src-tauri/src/commands/watcher.rs": file("watcher.rs", "src-tauri/src/commands/watcher.rs"),
    "src-tauri/src/commands/timeline.rs": file("timeline.rs", "src-tauri/src/commands/timeline.rs"),

    "src-tauri/src/services": folder("services", "src-tauri/src/services", [
      "src-tauri/src/services/workspace_service.rs",
      "src-tauri/src/services/timeline_service.rs",
    ]),
    "src-tauri/src/services/workspace_service.rs": file(
      "workspace_service.rs",
      "src-tauri/src/services/workspace_service.rs",
    ),
    "src-tauri/src/services/timeline_service.rs": file(
      "timeline_service.rs",
      "src-tauri/src/services/timeline_service.rs",
    ),

    "src-tauri/src/watcher": folder("watcher", "src-tauri/src/watcher", [
      "src-tauri/src/watcher/watcher.rs",
      "src-tauri/src/watcher/debounce.rs",
    ]),
    "src-tauri/src/watcher/watcher.rs": file("watcher.rs", "src-tauri/src/watcher/watcher.rs"),
    "src-tauri/src/watcher/debounce.rs": file("debounce.rs", "src-tauri/src/watcher/debounce.rs"),

    "src-tauri/src/workspace": folder("workspace", "src-tauri/src/workspace", [
      "src-tauri/src/workspace/manager.rs",
      "src-tauri/src/workspace/detector.rs",
    ]),
    "src-tauri/src/workspace/manager.rs": file("manager.rs", "src-tauri/src/workspace/manager.rs"),
    "src-tauri/src/workspace/detector.rs": file("detector.rs", "src-tauri/src/workspace/detector.rs"),

    "src-tauri/src/lib.rs": file("lib.rs", "src-tauri/src/lib.rs"),
    "src-tauri/src/main.rs": file("main.rs", "src-tauri/src/main.rs"),

    "ARCHITECTURE.md": file("ARCHITECTURE.md", "ARCHITECTURE.md"),
    "README.md": file("README.md", "README.md"),
  };

  return items;
}

interface ExplorerTreeProps {
  onSelectItem?: (item: ExplorerNodeData) => void;
}

/**
 * The tree itself: a `StaticTreeDataProvider` seeded with placeholder
 * data, rendered through `UncontrolledTreeEnvironment` (the library
 * manages expand/collapse/selection view-state internally) with
 * `ExplorerNode` supplying the visuals for every row.
 */
export function ExplorerTree({ onSelectItem }: ExplorerTreeProps) {
  // Kept alongside the provider (rather than read back out of it) so
  // selection lookups stay a plain object access instead of reaching
  // into `StaticTreeDataProvider`'s private state.
  const items = useMemo(() => buildPlaceholderItems(), []);
  const dataProvider = useMemo(() => new StaticTreeDataProvider(items, (item) => item), [items]);

  return (
    <UncontrolledTreeEnvironment<ExplorerNodeData>
      dataProvider={dataProvider}
      getItemTitle={(item) => item.data.name}
      viewState={{
        "explorer-tree": {
          expandedItems: ["frontend", "frontend/src", "src-tauri"],
        },
      }}
      canDragAndDrop={false}
      canReorderItems={false}
      canRename={false}
      canSearch={false}
      onSelectItems={(itemIds) => {
        const [selected] = itemIds;
        if (selected === undefined || onSelectItem === undefined) return;
        const node = items[selected]?.data;
        if (node) onSelectItem(node);
      }}
      renderItem={(props) => <ExplorerNode {...props} />}
      renderItemArrow={() => null}
    >
      <Tree treeId="explorer-tree" rootItem="root" treeLabel="File Explorer" />
    </UncontrolledTreeEnvironment>
  );
}
