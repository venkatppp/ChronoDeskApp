import type { ReactNode } from "react";
import {
  ChevronRight,
  Folder,
  FolderOpen,
  File,
  FileCode,
  FileJson,
  FileText,
  FileImage,
} from "lucide-react";
import type { TreeItem, TreeItemRenderContext } from "react-complex-tree";
import { cn } from "@/utils/cn";
import type { ExplorerNodeData } from "@/components/explorer/types";

/** Extensions rendered with the "code" icon, grouped for readability. */
const CODE_EXTENSIONS = new Set([
  "ts",
  "tsx",
  "js",
  "jsx",
  "rs",
  "css",
  "html",
  "toml",
  "yml",
  "yaml",
  "sql",
]);
const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "svg", "webp"]);

/**
 * Picks a file/folder icon by node kind + extension. Kept as a small pure
 * lookup rather than a big switch so a future real filesystem provider
 * can reuse it unchanged.
 */
function iconFor(node: ExplorerNodeData, isExpanded: boolean) {
  if (node.kind === "folder") {
    return isExpanded ? FolderOpen : Folder;
  }
  if (node.extension === "json") return FileJson;
  if (node.extension === "md") return FileText;
  if (node.extension && CODE_EXTENSIONS.has(node.extension)) return FileCode;
  if (node.extension && IMAGE_EXTENSIONS.has(node.extension)) return FileImage;
  return File;
}

interface ExplorerNodeProps {
  item: TreeItem<ExplorerNodeData>;
  depth: number;
  children: ReactNode | null;
  title: ReactNode;
  arrow: ReactNode;
  context: TreeItemRenderContext<never>;
}

/**
 * Renders a single row of the explorer tree — folder/file icon,
 * expand/collapse arrow, selection + focus highlight. Passed to
 * `ExplorerTree`'s `UncontrolledTreeEnvironment` as `renderItem`;
 * `react-complex-tree` supplies all interaction wiring
 * (`context.interactiveElementProps`, `arrowProps`, etc.) — this
 * component only supplies the visuals, matching the "unopinionated
 * rendering" model the library expects.
 */
export function ExplorerNode({ item, depth, children, title, context }: ExplorerNodeProps) {
  const Icon = iconFor(item.data, Boolean(context.isExpanded));

  return (
    <li {...context.itemContainerWithChildrenProps} className="list-none">
      <div
        {...context.itemContainerWithoutChildrenProps}
        style={{ paddingLeft: `${depth * 14 + 8}px` }}
        className="group relative"
      >
        <button
          type="button"
          {...context.interactiveElementProps}
          className={cn(
            "flex w-full items-center gap-1.5 rounded-[var(--radius-control)] py-1 pr-2 text-left text-[13px]",
            "transition-colors duration-150",
            context.isSelected
              ? "bg-(--color-accent-muted) text-(--color-foreground)"
              : "text-(--color-muted-foreground) hover:bg-(--color-surface-hover) hover:text-(--color-foreground)",
            context.isFocused && "outline outline-1 outline-(--color-accent) outline-offset-[-1px]",
          )}
        >
          <span
            className={cn(
              "flex h-4 w-4 shrink-0 items-center justify-center text-(--color-faint-foreground)",
              "transition-transform duration-150",
              item.isFolder && context.isExpanded && "rotate-90",
            )}
          >
            {item.isFolder && <ChevronRight className="h-3.5 w-3.5" strokeWidth={2} />}
          </span>
          <Icon
            className={cn(
              "h-4 w-4 shrink-0",
              item.isFolder ? "text-(--color-accent)" : "text-(--color-faint-foreground)",
            )}
            strokeWidth={1.75}
          />
          <span className="truncate">{title}</span>
        </button>
      </div>
      {children}
    </li>
  );
}
