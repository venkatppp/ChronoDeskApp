/**
 * File Explorer domain types (Phase 4.1 — first version).
 *
 * This is a UI-only tree shape for now: the explorer is seeded with
 * placeholder data (see `ExplorerTree.tsx`) rather than a real
 * filesystem read, since Phase 4.1 explicitly excludes filesystem IPC
 * (that's a follow-up phase). Keeping the node shape deliberately close
 * to what a future `commands::fs::*` IPC command would return (`path`,
 * `kind`, `children`) means swapping the placeholder provider for a
 * real one later is a data-source change, not a component rewrite.
 */

export type ExplorerNodeKind = "folder" | "file";

export interface ExplorerNodeData {
  /** Display name, e.g. "AppLayout.tsx". */
  name: string;
  /** Full placeholder path, e.g. "frontend/src/layouts/AppLayout.tsx". */
  path: string;
  kind: ExplorerNodeKind;
  /**
   * File extension without the leading dot (e.g. "tsx"), used to pick an
   * icon. Undefined for folders.
   */
  extension?: string;
}

/**
 * `react-complex-tree`'s `TreeItem<T>` record, keyed by
 * `TreeItemIndex` (string). `index` doubles as the id referenced by
 * `children` arrays elsewhere in the map.
 */
export type ExplorerTreeItems = Record<string, import("react-complex-tree").TreeItem<ExplorerNodeData>>;
