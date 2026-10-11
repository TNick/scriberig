/** @file Renders session branches with Material UI navigation. */
import type { ReactElement } from "react";
import { List, ListItem, ListItemButton, ListItemText } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { BranchSummary } from "@scriberig/components";
/** Renders the MuiSessionTree Material UI component. */
export function MuiSessionTree({
  branches,
  onSelect,
  className,
}: {
  branches: readonly BranchSummary[];
  onSelect?: (b: BranchSummary) => void;
  className?: string;
}): ReactElement {
  const render = (b: BranchSummary): ReactElement => (
    <ListItem key={b.id} sx={{ display: "block" }}>
      <ListItemButton onClick={() => onSelect?.(b)}>
        <ListItemText primary={b.title} secondary={b.status} />
      </ListItemButton>
      {b.children?.length ? <List disablePadding>{b.children.map(render)}</List> : null}
    </ListItem>
  );
  return (
    <MuiPanel title="Session branches" className={className}>
      <List>{branches.map(render)}</List>
    </MuiPanel>
  );
}
