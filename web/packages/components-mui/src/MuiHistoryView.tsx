/** @file Renders semantic history entries with Material UI. */
import type { ReactElement } from "react";
import { List, ListItemButton, ListItemText } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { DiffEntry } from "@scriberig/components";
/** Renders the MuiHistoryView Material UI component. */
export function MuiHistoryView({
  entries,
  onSelect,
  className,
}: {
  entries: readonly DiffEntry[];
  onSelect?: (e: DiffEntry) => void;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title="History" className={className}>
      <List>
        {entries.map((e) => (
          <ListItemButton key={e.id} onClick={() => onSelect?.(e)}>
            <ListItemText primary={e.summary} secondary={e.kind} />
          </ListItemButton>
        ))}
      </List>
    </MuiPanel>
  );
}
