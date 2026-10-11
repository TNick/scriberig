/** @file Renders caller-computed semantic differences with Material UI. */
import type { ReactElement } from "react";
import { Chip, List, ListItem, ListItemText } from "@mui/material";
import { MuiPanel } from "./mui-primitives";
import type { DiffEntry } from "@scriberig/components";
/** Renders the MuiSemanticDiffView Material UI component. */
export function MuiSemanticDiffView({
  entries,
  renderEntry,
  className,
}: {
  entries: readonly DiffEntry[];
  renderEntry?: (e: DiffEntry) => ReactElement;
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title="Semantic differences" className={className}>
      <List>
        {entries.map((e) => (
          <ListItem key={e.id}>
            {renderEntry?.(e) ?? (
              <>
                <Chip label={e.kind} size="small" />
                <ListItemText primary={e.summary} />
              </>
            )}
          </ListItem>
        ))}
      </List>
    </MuiPanel>
  );
}
