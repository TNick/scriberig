/** @file Renders a lazy semantic tree with Material UI controls. */
import { useSyncExternalStore, type ReactElement } from "react";
import { List, ListItem, ListItemButton, ListItemText, Button } from "@mui/material";
import type { ObjectTreeProps } from "@scriberig/components";
import type { TreeNode } from "@scriberig/editor-core";
/** Renders the MuiObjectTree Material UI component. */
export function MuiObjectTree({
  tree,
  rootId,
  onSelect,
  onLoadMore,
  className,
}: ObjectTreeProps): ReactElement {
  useSyncExternalStore(tree.subscribe.bind(tree), tree.version.bind(tree), tree.version.bind(tree));
  const render = (n: TreeNode): ReactElement => (
    <ListItem key={n.id} sx={{ display: "block" }}>
      <ListItemButton onClick={() => onSelect?.(n)}>
        {n.summary?.label ?? n.entityId}
      </ListItemButton>
      {n.children.length || n.nextCursor !== undefined ? (
        <Button
          onClick={() => {
            if (n.expanded) tree.collapse(n.id);
            else {
              tree.expand(n.id);
              onLoadMore?.(n);
            }
          }}
        >
          {n.expanded ? "Collapse" : "Expand"}
        </Button>
      ) : null}
      {n.nextCursor ? <Button onClick={() => onLoadMore?.(n)}>Load more</Button> : null}
      {n.loading ? <span role="status">Loadingâ€¦</span> : null}
      {n.error ? <span role="alert">{n.error}</span> : null}
      {n.expanded ? (
        <List disablePadding>
          {n.children.map((id) => {
            const c = tree.get(id);
            return c ? render(c) : null;
          })}
        </List>
      ) : null}
    </ListItem>
  );
  const root = tree.get(rootId);
  return (
    <List className={className} role="tree">
      {root ? (
        render(root)
      ) : (
        <ListItem>
          <ListItemText primary="No objects loaded" />
        </ListItem>
      )}
    </List>
  );
}
