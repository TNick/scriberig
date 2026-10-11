/** @file Renders validation diagnostics with explicit Material UI status. */
import type { ReactElement } from "react";
import { List, ListItem } from "@mui/material";
import { MuiPanel, MuiStatus } from "./mui-primitives";
/** Renders the MuiValidationResults Material UI component. */
export function MuiValidationResults({
  status,
  diagnostics,
  className,
}: {
  status: "current" | "failed" | "stale" | "unknown";
  diagnostics: readonly string[];
  className?: string;
}): ReactElement {
  return (
    <MuiPanel title="Validation results" className={className}>
      <MuiStatus
        severity={status === "failed" ? "error" : status === "current" ? "success" : "warning"}
      >
        Validation: {status}
      </MuiStatus>
      <List>
        {diagnostics.map((d) => (
          <ListItem key={d}>{d}</ListItem>
        ))}
      </List>
    </MuiPanel>
  );
}
