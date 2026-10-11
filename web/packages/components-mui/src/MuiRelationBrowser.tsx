/** @file Renders semantic relation members with Material UI. */
import type { ReactElement } from "react";
import { Button, List, ListItemButton, ListItemText } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { ObjectSummary } from "@scriberig/protocol";
/** Renders the MuiRelationBrowser Material UI component. */
export function MuiRelationBrowser({
  relation,
  items,
  nextCursor,
  onSelect,
  onLoadMore,
  className,
}: {
  relation: string;
  items: readonly ObjectSummary[];
  nextCursor?: string;
  onSelect?: (item: ObjectSummary) => void;
  onLoadMore?: () => void;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title={relation} className={className}>
      <List>
        {items.map((i) => (
          <ListItemButton key={i.ref.entityId} onClick={() => onSelect?.(i)}>
            <ListItemText primary={i.label ?? i.ref.entityId} secondary={i.typeId} />
          </ListItemButton>
        ))}
      </List>
      {nextCursor ? <Button onClick={onLoadMore}>Load more</Button> : null}
    </MuiPanel>
  );
}
