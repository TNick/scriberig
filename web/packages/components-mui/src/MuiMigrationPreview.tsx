/** @file Provides a Material UI migration review surface. */
import type { ReactElement } from "react";
import { Button, List, ListItem, Typography } from "@mui/material";
import { MuiPanel, MuiStatus } from "./mui-primitives";
/** Renders the MuiMigrationPreview Material UI component. */
export function MuiMigrationPreview({
  baseSnapshot,
  candidateSnapshot,
  changes,
  validation,
  state,
  onApply,
  onReject,
  className,
}: {
  baseSnapshot: string;
  candidateSnapshot?: string;
  changes: readonly string[];
  validation: "current" | "failed" | "unknown";
  state?: "ready" | "stale" | "conflict";
  onApply?: () => void;
  onReject?: () => void;
  className?: string;
}): ReactElement {
  const blocked = validation === "failed" || state === "stale" || state === "conflict";
  return (
    <MuiPanel title="Migration preview" className={className}>
      <Typography>Base: {baseSnapshot}</Typography>
      {candidateSnapshot ? <Typography>Candidate: {candidateSnapshot}</Typography> : null}
      <MuiStatus
        severity={
          validation === "failed" ? "error" : validation === "current" ? "success" : "warning"
        }
      >
        Validation: {validation}
      </MuiStatus>
      {state ? (
        <MuiStatus severity={state === "ready" ? "info" : "warning"}>State: {state}</MuiStatus>
      ) : null}
      <List>
        {changes.map((c) => (
          <ListItem key={c}>{c}</ListItem>
        ))}
      </List>
      {onApply ? (
        <Button disabled={blocked} onClick={onApply}>
          Apply
        </Button>
      ) : null}
      {onReject ? <Button onClick={onReject}>Reject</Button> : null}
    </MuiPanel>
  );
}
