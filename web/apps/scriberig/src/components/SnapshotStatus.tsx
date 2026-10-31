/** @file Renders explicit snapshot and connection state. */
import { Alert } from "@mui/material";
import type { ReactElement } from "react";

/** Displays current, historical, stale, candidate, or offline state. */
/** @returns The rendered SnapshotStatus view or runtime value. */
export function SnapshotStatus({
  state,
}: {
  state:
    "current" | "historical" | "candidate" | "stale" | "offline" | "connecting" | "disconnected";
}): ReactElement {
  return (
    <Alert
      severity={
        state === "stale" || state === "offline" || state === "disconnected"
          ? "warning"
          : state === "candidate"
            ? "warning"
            : "info"
      }
      role="status"
      variant="outlined"
      sx={{
        mb: 1.5,
        borderColor: "divider",
        bgcolor: "background.paper",
        "& .MuiAlert-message": { py: 0.25 },
      }}
    >
      {state === "current"
        ? "Viewing the current project snapshot."
        : state === "historical"
          ? "Viewing an earlier snapshot."
          : state === "candidate"
            ? "Reviewing proposed changes."
            : state === "stale"
              ? "This view may be out of date. Refresh its context before making changes."
              : state === "offline" || state === "disconnected"
                ? "Connection lost. Reconnect to continue working."
                : "Connecting to the project…"}
    </Alert>
  );
}
