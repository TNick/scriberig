/** @file Renders explicit snapshot and connection state. */
import { Alert } from "@mui/material";
import type { ReactElement } from "react";

/** Displays current, historical, stale, candidate, or offline state. */
/** @returns The rendered SnapshotStatus view or runtime value. */
export function SnapshotStatus({
  state,
}: {
  state: "current" | "historical" | "candidate" | "stale" | "offline";
}): ReactElement {
  return (
    <Alert severity={state === "stale" || state === "offline" ? "warning" : "info"} role="status">
      View state: {state}
    </Alert>
  );
}
