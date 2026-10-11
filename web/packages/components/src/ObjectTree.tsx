/** @file Renders a lazy, accessible semantic object tree. */
import { useSyncExternalStore, type ReactElement } from "react";
import type { TreeNode } from "@scriberig/editor-core";
import type { ObjectTreeProps } from "./component-types";

/** Renders only nodes loaded by the headless tree controller. */
export function ObjectTree({
  tree,
  rootId,
  onSelect,
  onLoadMore,
  className,
}: ObjectTreeProps): ReactElement {
  useSyncExternalStore(tree.subscribe.bind(tree), tree.version.bind(tree));
  const renderNode = (node: TreeNode): ReactElement => (
    <li
      key={node.id}
      role="treeitem"
      aria-expanded={node.children.length > 0 ? node.expanded : undefined}
    >
      <button type="button" onClick={() => onSelect?.(node)}>
        {node.summary?.label ?? node.entityId}
      </button>
      {node.children.length > 0 ? (
        <button
          type="button"
          aria-label={node.expanded ? "Collapse" : "Expand"}
          onClick={() => (node.expanded ? tree.collapse(node.id) : tree.expand(node.id))}
        >
          {node.expanded ? "Collapse" : "Expand"}
        </button>
      ) : null}
      {node.loading ? <span role="status">Loading…</span> : null}
      {node.error ? <span role="alert">{node.error}</span> : null}
      {node.expanded && node.children.length > 0 ? (
        <ul role="group">
          {node.children.map((id) => {
            const child = tree.get(id);
            return child ? renderNode(child) : null;
          })}
        </ul>
      ) : null}
      {node.nextCursor ? (
        <button type="button" onClick={() => onLoadMore?.(node)}>
          Load more
        </button>
      ) : null}
    </li>
  );
  const root = tree.get(rootId);
  return (
    <ul className={className} role="tree">
      {root ? renderNode(root) : null}
    </ul>
  );
}
